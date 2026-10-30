use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Emitter};

use crate::instance_data::{resolve_instance_openmw_config, ClientConfigFile};
use crate::instance_setup::{instance_tes3mp_dir, write_required_data_files_for_resolved};
use crate::sync_client::types::{ProcessOutputEvent, ProcessStatusEvent, ProcessStream};

use super::state::{spawn_exit_watcher, ProcessManager};
use super::types::ProcessRole;

// TES3MP-NG's client is built as openmw.exe
const CLIENT_EXE_NAMES: &[&str] = &["openmw.exe", "tes3mp.exe"];
const SERVER_EXE_NAMES: &[&str] = &["tes3mp-server.exe"];
/// Executables sit at the top of the portable TES3MP-NG folder.
const EXE_SEARCH_DEPTH: u32 = 2;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn configure_tes3mp_command(command: &mut Command, working_dir: &Path) {
    command
        .current_dir(working_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Console-subsystem children otherwise get a blank terminal when launched from
    // our GUI app. Output is still captured via the pipes above.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
}

pub fn find_executable(root: &Path, names: &[&str], max_depth: u32) -> Option<PathBuf> {
    if max_depth == 0 {
        return None;
    }

    let entries = std::fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            let file_name = path.file_name().and_then(|n| n.to_str())?;
            if names.iter().any(|name| file_name.eq_ignore_ascii_case(name)) {
                return Some(path);
            }
        }
    }

    if max_depth > 1 {
        let entries = std::fs::read_dir(root).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = find_executable(&path, names, max_depth - 1) {
                    return Some(found);
                }
            }
        }
    }

    None
}

fn pipe_process_output(
    app: AppHandle,
    instance_id: String,
    role: ProcessRole,
    stream: ProcessStream,
    reader: impl BufRead + Send + 'static,
) {
    thread::spawn(move || {
        for line in reader.lines().map_while(Result::ok) {
            let _ = app.emit(
                "process-output",
                ProcessOutputEvent {
                    instance_id: instance_id.clone(),
                    role: role.as_str().to_string(),
                    stream: stream.clone(),
                    line,
                },
            );
        }
    });
}

/// Launch the TES3MP-NG client of an instance. Its portable `config/` folder is written first:
/// `openmw.cfg` from the load order, and seeded settings and `client_files` (the host's, for synced
/// instances).
pub fn launch_tes3mp_client(
    app: AppHandle,
    manager: Arc<ProcessManager>,
    instance_id: &str,
    instance_root: &Path,
    data_dir: &Path,
    client_files: &[ClientConfigFile],
) -> Result<(), String> {
    let tes3mp_dir = instance_tes3mp_dir(instance_root);
    let config_dir =
        crate::client_config::write_instance_client_config(&tes3mp_dir, data_dir, client_files)?;

    let exe = find_executable(&tes3mp_dir, CLIENT_EXE_NAMES, EXE_SEARCH_DEPTH).ok_or_else(|| {
        format!(
            "Could not find the TES3MP-NG client (openmw.exe) under {}",
            tes3mp_dir.display()
        )
    })?;

    let mut command = Command::new(&exe);
    configure_tes3mp_command(
        &mut command,
        exe.parent().unwrap_or(&tes3mp_dir),
    );

    tauri_plugin_log::log::info!(
        "Launching TES3MP-NG client with its config in {}",
        config_dir.display()
    );

    let mut child = command
        .spawn()
        .map_err(|err| format!("Failed to launch TES3MP client: {err}"))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    if let Some(out) = stdout {
        pipe_process_output(
            app.clone(),
            instance_id.to_string(),
            ProcessRole::Client,
            ProcessStream::Stdout,
            BufReader::new(out),
        );
    }
    if let Some(err) = stderr {
        pipe_process_output(
            app.clone(),
            instance_id.to_string(),
            ProcessRole::Client,
            ProcessStream::Stderr,
            BufReader::new(err),
        );
    }

    thread::sleep(Duration::from_millis(900));
    if let Ok(Some(status)) = child.try_wait() {
        return Err(format!(
            "TES3MP client exited immediately (code {:?}). Check the client output console below.",
            status.code()
        ));
    }

    let _ = app.emit(
        "process-status",
        ProcessStatusEvent {
            instance_id: instance_id.to_string(),
            role: ProcessRole::Client.as_str().to_string(),
            running: true,
            exit_code: None,
        },
    );

    let child_handle = manager.insert(instance_id, ProcessRole::Client, child)?;
    spawn_exit_watcher(
        app.clone(),
        manager.clone(),
        instance_id.to_string(),
        ProcessRole::Client,
        child_handle,
    );
    Ok(())
}

pub fn launch_tes3mp_server(
    app: AppHandle,
    manager: Arc<ProcessManager>,
    instance_id: &str,
    instance_root: &Path,
    data_dir: &Path,
) -> Result<(), String> {
    let tes3mp_dir = instance_tes3mp_dir(instance_root);
    let resolved = resolve_instance_openmw_config(data_dir)?;
    write_required_data_files_for_resolved(&tes3mp_dir, &resolved)?;
    let settings = crate::instance_settings::load_instance_settings(data_dir)?;
    crate::instance_settings::write_script_policy(&tes3mp_dir, &settings.lua_script_policy)?;
    let exe = find_executable(&tes3mp_dir, SERVER_EXE_NAMES, EXE_SEARCH_DEPTH).ok_or_else(|| {
        format!(
            "Could not find TES3MP server executable under {}",
            tes3mp_dir.display()
        )
    })?;

    let mut command = Command::new(&exe);
    configure_tes3mp_command(
        &mut command,
        exe.parent().unwrap_or(&tes3mp_dir),
    );

    let mut child = command
        .spawn()
        .map_err(|e| format!("Failed to launch TES3MP server: {e}"))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    if let Some(out) = stdout {
        pipe_process_output(
            app.clone(),
            instance_id.to_string(),
            ProcessRole::Server,
            ProcessStream::Stdout,
            BufReader::new(out),
        );
    }
    if let Some(err) = stderr {
        pipe_process_output(
            app.clone(),
            instance_id.to_string(),
            ProcessRole::Server,
            ProcessStream::Stderr,
            BufReader::new(err),
        );
    }

    let _ = app.emit(
        "process-status",
        ProcessStatusEvent {
            instance_id: instance_id.to_string(),
            role: ProcessRole::Server.as_str().to_string(),
            running: true,
            exit_code: None,
        },
    );

    let child_handle = manager.insert(instance_id, ProcessRole::Server, child)?;
    spawn_exit_watcher(
        app.clone(),
        manager.clone(),
        instance_id.to_string(),
        ProcessRole::Server,
        child_handle,
    );
    Ok(())
}

pub fn stop_tes3mp_process(
    app: AppHandle,
    manager: &ProcessManager,
    instance_id: &str,
    role: ProcessRole,
) -> Result<bool, String> {
    manager.stop(Some(app), instance_id, role)
}
