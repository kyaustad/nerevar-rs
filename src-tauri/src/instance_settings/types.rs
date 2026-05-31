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
