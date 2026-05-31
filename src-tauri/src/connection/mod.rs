pub mod commands;
pub mod instance_delete;
pub mod instance_edit;

pub use instance_delete::delete_instance;
pub use instance_edit::{get_instance_connection_settings, update_instance};
