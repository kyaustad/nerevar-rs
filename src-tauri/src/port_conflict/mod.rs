use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

use netstat2::{
    get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState,
};
use serde::{Deserialize, Serialize};
use sysinfo::{Pid, ProcessesToUpdate, System};
use tauri::{AppHandle, Emitter, State};
use ts_rs::TS;

use crate::data::NerevarConfig;
use crate::instance_setup::{instance_tes3mp_dir, read_tes3mp_server_settings};
use crate::AppState;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PortRole {
    NerevarSync,
    Tes3mpServer,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PortConflict {
    pub port: u16,
    pub role: PortRole,
    pub pid: u32,
    pub process_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_name: Option<String>,
}

pub fn is_addr_in_use_error(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    message.contains("address already in use")
        || message.contains("only one usage of each socket address")
        || message.contains("os error 10048")
        || message.contains("os error 98")
}

pub fn find_listening_process(port: u16) -> Result<Option<(u32, String, Option<String>)>, String> {
    let sockets = get_sockets_info(
        AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6,
        ProtocolFlags::TCP,
    )
    .map_err(|error| format!("Failed to inspect network sockets: {error}"))?;

    for socket in sockets {
        let ProtocolSocketInfo::Tcp(tcp) = socket.protocol_socket_info else {
            continue;
        };
        if tcp.state != TcpState::Listen || tcp.local_port != port {
            continue;
        }

        let Some(pid) = socket.associated_pids.first().copied() else {
            continue;
        };

        if pid == std::process::id() {
            continue;
        }

        let (process_name, executable_path) = process_details(pid);
        return Ok(Some((pid, process_name, executable_path)));
    }

    Ok(None)
}

fn process_details(pid: u32) -> (String, Option<String>) {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);

    match system.process(Pid::from_u32(pid)) {
        Some(process) => (
            process.name().to_string_lossy().into_owned(),
            process
                .exe()
                .map(|path| path.to_string_lossy().into_owned()),
        ),
        None => (format!("PID {pid}"), None),
    }
}

fn is_protected_pid(pid: u32) -> bool {
    pid == 0 || pid == 4
}

pub fn conflict_for_port(
    port: u16,
    role: PortRole,
    instance_id: Option<String>,
    instance_name: Option<String>,
) -> Result<Option<PortConflict>, String> {
    let Some((pid, process_name, executable_path)) = find_listening_process(port)? else {
        return Ok(None);
    };

    Ok(Some(PortConflict {
        port,
        role,
        pid,
        process_name,
        executable_path,
        instance_id,
        instance_name,
    }))
}

pub fn check_startup_conflicts(config: &NerevarConfig) -> Result<Vec<PortConflict>, String> {
    let mut conflicts = Vec::new();

    if let Some(conflict) =
        conflict_for_port(config.sync_port as u16, PortRole::NerevarSync, None, None)?
    {
        conflicts.push(conflict);
    }

    if let Some(owned) = &config.owned_instances {
        for instance in owned {
            let tes3mp_dir = instance_tes3mp_dir(Path::new(&instance.path));
            let settings = match read_tes3mp_server_settings(&tes3mp_dir) {
                Ok(settings) => settings,
                Err(_) => continue,
            };

            let Some(conflict) = conflict_for_port(
                settings.port,
                PortRole::Tes3mpServer,
                Some(instance.id.clone()),
                Some(instance.name.clone()),
            )?
            else {
                continue;
            };

            if conflicts
                .iter()
                .any(|existing| existing.port == conflict.port && existing.pid == conflict.pid)
            {
                continue;
            }

            conflicts.push(conflict);
        }
    }

    Ok(conflicts)
}

pub fn kill_process(pid: u32) -> Result<(), String> {
    if is_protected_pid(pid) {
        return Err(format!("Refusing to terminate protected process (PID {pid})"));
    }

    #[cfg(windows)]
    {
        let output = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .output()
            .map_err(|error| format!("Failed to run taskkill: {error}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let message = format!("Failed to terminate PID {pid}: {stdout} {stderr}");
            return Err(message.trim().to_string());
        }
    }

    #[cfg(not(windows))]
    {
        let output = Command::new("kill")
            .args(["-9", &pid.to_string()])
            .output()
            .map_err(|error| format!("Failed to run kill: {error}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to terminate PID {pid}: {stderr}".trim()));
        }
    }

    Ok(())
}

pub fn emit_port_conflicts(app: &AppHandle, conflicts: Vec<PortConflict>) {
    if conflicts.is_empty() {
        return;
    }
    let _ = app.emit("port-conflicts-detected", conflicts);
}

#[tauri::command]
pub fn check_port_conflicts(
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<PortConflict>, String> {
    let guard = state.lock().map_err(|_| "App state lock poisoned".to_string())?;
    check_startup_conflicts(&guard.nerevar_config)
}

#[tauri::command]
pub fn kill_port_process(pid: u32) -> Result<(), String> {
    kill_process(pid)
}

#[tauri::command]
pub fn retry_sync_server(state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    let mut guard = state
        .lock()
        .map_err(|_| "App state lock poisoned".to_string())?;

    let port = guard.nerevar_config.sync_port;
    let next_retry = guard.server_retry_generation.wrapping_add(1);
    guard.server_retry_generation = next_retry;

    if let Some(tx) = guard.server_retry_tx.clone() {
        let _ = tx.send(next_retry);
    }
    if let Some(tx) = guard.server_port_tx.clone() {
        let _ = tx.send(port);
    }

    Ok(())
}
