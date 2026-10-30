use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const INSTANCE_SETTINGS_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SettingCategory {
    Tes3mpGameplay,
    OpenMwGraphics,
    OpenMwShaders,
    /// `settings.cfg` `[Post Processing]`: post-processing shaders and their chain.
    OpenMwPostProcessing,
}

impl SettingCategory {
    /// The `settings.cfg` section of OpenMW categories.
    pub fn openmw_section(self) -> Option<&'static str> {
        match self {
            SettingCategory::Tes3mpGameplay => None,
            SettingCategory::OpenMwGraphics => Some("Graphics"),
            SettingCategory::OpenMwShaders => Some("Shaders"),
            SettingCategory::OpenMwPostProcessing => Some(POST_PROCESSING_SECTION),
        }
    }
}

pub const POST_PROCESSING_SECTION: &str = "Post Processing";
/// Comma-separated post-processing shaders, in order (file names without `.omwfx`).
pub const SHADER_CHAIN_KEY: &str = "chain";

/// Where a TES3MP-NG global Lua script runs (`data/lua/scriptPolicy.json` of the server).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ScriptMode {
    /// Every player's game runs its own copy, as in single player.
    #[default]
    Client,
    /// One player's game, the Lua world authority, runs it for everyone.
    Authority,
}

impl ScriptMode {
    pub fn as_str(self) -> &'static str {
        match self {
            ScriptMode::Client => "client",
            ScriptMode::Authority => "authority",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LuaScriptPolicy {
    #[serde(default)]
    pub default_mode: ScriptMode,
    /// Modes of global scripts by path (`scripts/mod/world.lua`), as listed in `.omwscripts` files.
    #[serde(default)]
    pub scripts: BTreeMap<String, ScriptMode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SettingValueType {
    Boolean,
    Integer,
    Float,
    String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SettingValue {
    Boolean(bool),
    Integer(#[ts(type = "number")] i64),
    Float(f64),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Tes3mpGameSettingEntry {
    pub name: String,
    pub value: SettingValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct InstanceSettings {
    pub version: u32,
    pub tes3mp_game_settings: Vec<Tes3mpGameSettingEntry>,
    #[serde(default)]
    pub openmw_settings: BTreeMap<String, BTreeMap<String, SettingValue>>,
    /// Extra `key=value` lines seeded into the client's `openmw.cfg` (e.g. `groundcover=SomeMod.esp`).
    #[serde(default)]
    pub openmw_cfg_overrides: Vec<String>,
    /// Raw INI text (`[Section]` blocks) seeded into the client's `settings.cfg`.
    #[serde(default)]
    pub openmw_settings_cfg_overrides: String,
    /// TES3MP-NG global Lua script modes, written to the server's `scriptPolicy.json`.
    #[serde(default)]
    pub lua_script_policy: LuaScriptPolicy,
    /// Post-processing shader values (OpenMW's `shaders.yaml`), seeded into players' configs.
    /// When unset, hosts share their own game's `shaders.yaml`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shaders_yaml: Option<String>,
}

impl Default for InstanceSettings {
    fn default() -> Self {
        crate::instance_settings::defaults::default_instance_settings()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SettingDefinition {
    pub key: String,
    pub category: SettingCategory,
    pub value_type: SettingValueType,
    pub label: String,
    pub description: String,
    pub default_value: SettingValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(type = "number")]
    pub min_integer: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(type = "number")]
    pub max_integer: Option<i64>,
}

impl SettingValue {
    pub fn as_lua_literal(&self) -> String {
        match self {
            SettingValue::Boolean(value) => value.to_string(),
            SettingValue::Integer(value) => value.to_string(),
            SettingValue::Float(value) => value.to_string(),
            SettingValue::String(value) => format!("\"{}\"", escape_lua_string(value)),
        }
    }

    pub fn as_settings_cfg_value(&self) -> String {
        match self {
            SettingValue::Boolean(value) => {
                if *value {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            SettingValue::Integer(value) => value.to_string(),
            SettingValue::Float(value) => value.to_string(),
            SettingValue::String(value) => value.clone(),
        }
    }
}

fn escape_lua_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
