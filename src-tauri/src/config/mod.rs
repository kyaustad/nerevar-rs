pub mod nerevar_config;
pub use nerevar_config::{
    add_instance, complete_onboarding, generate_default_global_openmw_config,
    load_or_create_nerevar_config, nerevar_config_file_path, set_root_path, set_sync_port,
    update_synced_instance, validate_global_openmw_config,
};
