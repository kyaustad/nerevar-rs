use std::path::Path;

use crate::data::InstanceConfig;
use crate::instance_data::NerevarManifest;

pub fn apply_manifest_metadata(instance: &mut InstanceConfig, manifest: &NerevarManifest) {
    instance.tes3mp_server_port = Some(manifest.tes3mp_server_port);
}

pub fn write_synced_client_connection(
    instance: &InstanceConfig,
    manifest: &NerevarManifest,
) -> Result<(), String> {
    let host = instance
        .remote_host
        .as_deref()
        .ok_or_else(|| "Synced instance has no remote host".to_string())?;
    let port = manifest.tes3mp_server_port;
    let password = instance
        .sync_password
        .as_deref()
        .filter(|value| !value.is_empty())
        .unwrap_or(manifest.tes3mp_server_password.as_str());
    crate::instance_setup::write_tes3mp_client_connection(
        &crate::instance_setup::instance_tes3mp_dir(Path::new(&instance.path)),
        host,
        port,
        password,
    )
    .map(|_| ())
}
