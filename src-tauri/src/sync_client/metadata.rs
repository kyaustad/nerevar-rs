use std::path::Path;

use crate::data::InstanceConfig;
use crate::instance_data::NerevarManifest;

/// Synced instances follow the host: its name, description, TES3MP-NG release and game server.
pub fn apply_manifest_metadata(instance: &mut InstanceConfig, manifest: &NerevarManifest) {
    instance.tes3mp_server_port = Some(manifest.tes3mp_server_port);
    if !manifest.instance_name.is_empty() {
        instance.name = manifest.instance_name.clone();
    }
    instance.description = manifest.instance_description.clone();
    if let Some(tag) = &manifest.tes3mp_release_tag {
        instance.release_tag = Some(tag.clone());
    }
}

/// Point the synced client at the host's game server, in its `config/tes3mp-client.cfg`.
pub fn write_synced_client_connection(
    instance: &InstanceConfig,
    manifest: &NerevarManifest,
) -> Result<(), String> {
    let address = match manifest.tes3mp_server_address.as_deref() {
        Some(address) if !address.is_empty() => address,
        _ => instance
            .remote_host
            .as_deref()
            .ok_or_else(|| "Synced instance has no remote host".to_string())?,
    };
    // The host's game server password; hosts use it as their sync password too
    let password = if manifest.tes3mp_server_password.is_empty() {
        instance.sync_password.as_deref().unwrap_or_default()
    } else {
        manifest.tes3mp_server_password.as_str()
    };
    crate::instance_setup::write_tes3mp_client_connection(
        &crate::instance_setup::instance_tes3mp_dir(Path::new(&instance.path)),
        address,
        manifest.tes3mp_server_port,
        password,
    )
    .map(|_| ())
}
