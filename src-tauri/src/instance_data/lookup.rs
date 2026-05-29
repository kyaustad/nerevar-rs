use crate::data::{InstanceConfig, NerevarConfig};

pub fn find_instance_by_id<'a>(
    config: &'a NerevarConfig,
    instance_id: &str,
) -> Option<&'a InstanceConfig> {
    if let Some(owned) = &config.owned_instances {
        if let Some(found) = owned.iter().find(|i| i.id == instance_id) {
            return Some(found);
        }
    }
    if let Some(synced) = &config.synced_instances {
        if let Some(found) = synced.iter().find(|i| i.id == instance_id) {
            return Some(found);
        }
    }
    None
}
