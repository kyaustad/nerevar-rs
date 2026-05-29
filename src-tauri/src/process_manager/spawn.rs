use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Emitter};

use crate::instance_data::{
    resolve_instance_openmw_config, write_tes3mp_launch_openmw_cfg,
};
use crate::instance_setup::{instance_tes3mp_dir, write_required_data_files_for_resolved};
use crate::openmw_ini_importer::begin_global_openmw_launch_patch;
use crate::sync_client::types::{ProcessOutputEvent, ProcessStatusEvent, ProcessStream};

use super::state::{spawn_exit_watcher, ProcessManager};
use super::types::ProcessRole;

const CLIENT_EXE_NAMES: &[&str] = &["tes3mp.exe", "TES3MP.exe", "openmw.exe", "OpenMW.exe"];
const SERVER_EXE_NAMES: &[&str] = &["tes3mp-server.exe", "TES3MP-server.exe"];

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

fn prepare_launch_cfg(instance_root: &Path, data_dir: &Path) -> Result<PathBuf, String> {
    let resolved = resolve_instance_openmw_config(data_dir)?;
    let tes3mp_dir = instance_tes3mp_dir(instance_root);
    write_tes3mp_launch_openmw_cfg(&tes3mp_dir, data_dir, &resolved)
}

pub fn launch_tes3mp_client(
    app: AppHandle,
    manager: std::sync::Arc<ProcessManager>,
    instance_id: &str,
    instance_root: &Path,
    data_dir: &Path,
) -> Result<(), String> {
    let global_patch = begin_global_openmw_launch_patch()?;
    if let Some(patch) = global_patch {
        manager.store_global_openmw_patch(patch)?;
    }

    let cfg_path = match prepare_launch_cfg(instance_root, data_dir) {
        Ok(path) => path,
        Err(err) => {
            manager.restore_global_openmw_patch_if_any();
            return Err(err);
        }
    };
    let tes3mp_dir = instance_tes3mp_dir(instance_root);
    let exe = find_executable(&tes3mp_dir, CLIENT_EXE_NAMES, 5).ok_or_else(|| {
        manager.restore_global_openmw_patch_if_any();
        format!(
            "Could not find TES3MP client executable under {}",
            tes3mp_dir.display()
        )
    })?;

    let mut command = Command::new(&exe);
    command
        .current_dir(exe.parent().unwrap_or(&tes3mp_dir))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    tauri_plugin_log::log::info!(
        "Launching TES3MP client with openmw.cfg at {}",
        cfg_path.display()
    );

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => {
            manager.restore_global_openmw_patch_if_any();
            return Err(format!("Failed to launch TES3MP client: {err}"));
        }
    };

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
        manager.restore_global_openmw_patch_if_any();
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
    manager: std::sync::Arc<ProcessManager>,
    instance_id: &str,
    instance_root: &Path,
    data_dir: &Path,
) -> Result<(), String> {
    let _cfg_path = prepare_launch_cfg(instance_root, data_dir)?;
    let tes3mp_dir = instance_tes3mp_dir(instance_root);
    let resolved = resolve_instance_openmw_config(data_dir)?;
    write_required_data_files_for_resolved(&tes3mp_dir, &resolved)?;
    let exe = find_executable(&tes3mp_dir, SERVER_EXE_NAMES, 5).ok_or_else(|| {
        format!(
            "Could not find TES3MP server executable under {}",
            tes3mp_dir.display()
        )
    })?;

    let mut command = Command::new(&exe);
    command
        .current_dir(exe.parent().unwrap_or(&tes3mp_dir))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

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
