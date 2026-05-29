use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ProcessRole {
    Client,
    Server,
}

impl ProcessRole {
    pub fn as_str(self) -> &'static str {
        match self {
            ProcessRole::Client => "client",
            ProcessRole::Server => "server",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "client" => Some(ProcessRole::Client),
            "server" => Some(ProcessRole::Server),
            _ => None,
        }
    }
}
