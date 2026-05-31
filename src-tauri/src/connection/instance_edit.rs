use std::path::Path;
use std::sync::Mutex;

use tauri::{AppHandle, Emitter, State};

use crate::config::nerevar_config::{update_owned_instance, update_synced_instance};
use crate::data::{InstanceConnectionSettings, InstanceEditPayload};
use crate::instance_data::find_instance_by_id;
use crate::instance_setup::{
    instance_tes3mp_dir, read_tes3mp_client_settings, read_tes3mp_server_settings,
    update_server_connection_settings, write_owned_client_connection,
    write_tes3mp_client_connection,
};
use crate::AppState;

#[tauri::command]
pub fn get_instance_connection_settings(
    state: State<'_, Mutex<AppState>>,
    instance_id: String,
) -> Result<InstanceConnectionSettings, String> {
    let instance = {
        let guard = state.lock().map_err(|_| "App state lock poisoned".to_string())?;
        find_instance_by_id(&guard.nerevar_config, &instance_id)
            .ok_or_else(|| format!("Instance not found: {instance_id}"))?
            .clone()
    };

    let tes3mp_dir = instance_tes3mp_dir(Path::new(&instance.path));
    let is_synced = instance.remote_host.is_some();

    if is_synced {
        let client = read_tes3mp_client_settings(&tes3mp_dir)?;
        Ok(InstanceConnectionSettings {
            name: instance.name,
            description: instance.description,
            host: instance
                .remote_host
                .clone()
                .unwrap_or(client.destination_address),
            port: instance.remote_sync_port.unwrap_or(25567),
            password: instance
                .sync_password
                .clone()
                .unwrap_or(client.password),
            is_synced: true,
            sync_port: instance.remote_sync_port,
        })
    } else {
        let server = read_tes3mp_server_settings(&tes3mp_dir)?;
        Ok(InstanceConnectionSettings {
            name: instance.name,
            description: instance.description,
            host: server.hostname,
            port: server.port,
            password: server.password,
            is_synced: false,
            sync_port: None,
        })
    }
}

#[tauri::command]
pub fn update_instance(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    edit: InstanceEditPayload,
) -> Result<InstanceConnectionSettings, String> {
    let mut instance = {
        let guard = state.lock().map_err(|_| "App state lock poisoned".to_string())?;
        find_instance_by_id(&guard.nerevar_config, &edit.id)
            .ok_or_else(|| format!("Instance not found: {}", edit.id))?
            .clone()
    };

    let tes3mp_dir = instance_tes3mp_dir(Path::new(&instance.path));
    let is_synced = instance.remote_host.is_some();

    instance.name = edit.name.trim().to_string();
    instance.description = edit.description;

    if is_synced {
        instance.remote_host = Some(edit.host.trim().to_string());
        instance.remote_sync_port = Some(edit.port);
        instance.sync_password = Some(edit.password.clone());

        let game_port = instance.tes3mp_server_port.or_else(|| {
            read_tes3mp_client_settings(&tes3mp_dir)
                .ok()
                .map(|client| client.port)
        });
        if let Some(game_port) = game_port {
            write_tes3mp_client_connection(
                &tes3mp_dir,
                edit.host.trim(),
                game_port,
                &edit.password,
            )?;
            instance.tes3mp_server_port = Some(game_port);
        }
        let config = update_synced_instance(&state, instance)?;
        let _ = app.emit("on_config_change", config);
    } else {
        update_server_connection_settings(
            &tes3mp_dir,
            edit.host.trim(),
            edit.port,
            &edit.password,
        )?;
        write_owned_client_connection(&tes3mp_dir)?;
        instance.tes3mp_server_port = Some(edit.port);
        let config = update_owned_instance(&state, instance)?;
        let _ = app.emit("on_config_change", config);
    }

    get_instance_connection_settings(state, edit.id)
}
