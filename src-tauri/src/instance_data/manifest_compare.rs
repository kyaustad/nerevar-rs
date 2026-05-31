use super::types::NerevarManifest;

pub fn manifests_differ(local: &NerevarManifest, remote: &NerevarManifest) -> bool {
    if local.generated_at != remote.generated_at {
        return true;
    }
    if local.total_download_bytes != remote.total_download_bytes {
        return true;
    }
    if local.packages.len() != remote.packages.len() {
        return true;
    }

    let mut local_packages: Vec<_> = local.packages.iter().collect();
    let mut remote_packages: Vec<_> = remote.packages.iter().collect();
    local_packages.sort_by_key(|p| &p.id);
    remote_packages.sort_by_key(|p| &p.id);

    for (left, right) in local_packages.iter().zip(remote_packages.iter()) {
        if left.id != right.id
            || left.tree_checksum != right.tree_checksum
            || left.file_count != right.file_count
            || left.total_size_bytes != right.total_size_bytes
        {
            return true;
        }
    }

    if local.resolved.content != remote.resolved.content {
        return true;
    }

    if local.required_data_files != remote.required_data_files {
        return true;
    }

    if local.instance_settings != remote.instance_settings {
        return true;
    }

    false
}
