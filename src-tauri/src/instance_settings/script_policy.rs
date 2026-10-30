//! TES3MP-NG's `data/lua/scriptPolicy.json`: whether each global Lua script runs on every
//! player's game (`client`) or once, on the Lua world authority (`authority`).

use std::path::{Path, PathBuf};

use super::types::LuaScriptPolicy;

const SCRIPT_POLICY_FILE: &str = "scriptPolicy.json";
const SERVER_LUA_DATA_DIR: &str = "server/data/lua";

pub fn script_policy_path(tes3mp_dir: &Path) -> PathBuf {
    tes3mp_dir.join(SERVER_LUA_DATA_DIR).join(SCRIPT_POLICY_FILE)
}

pub fn write_script_policy(tes3mp_dir: &Path, policy: &LuaScriptPolicy) -> Result<PathBuf, String> {
    let path = script_policy_path(tes3mp_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create {}: {e}", parent.display()))?;
    }
    std::fs::write(&path, format_script_policy(policy))
        .map_err(|e| format!("Failed to write {}: {e}", path.display()))?;
    Ok(path)
}

fn format_script_policy(policy: &LuaScriptPolicy) -> String {
    let scripts: serde_json::Map<String, serde_json::Value> = policy
        .scripts
        .iter()
        .map(|(script, mode)| (script.clone(), serde_json::Value::from(mode.as_str())))
        .collect();
    let json = serde_json::json!({
        "default": policy.default_mode.as_str(),
        "scripts": scripts,
    });
    let mut text = serde_json::to_string_pretty(&json).expect("script policy is valid JSON");
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance_settings::ScriptMode;

    #[test]
    fn writes_tes3mp_ng_policy_format() {
        let dir = std::env::temp_dir().join(format!("nerevar-policy-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut policy = LuaScriptPolicy::default();
        policy
            .scripts
            .insert("scripts/spawner/world.lua".into(), ScriptMode::Authority);

        let path = write_script_policy(&dir, &policy).unwrap();
        assert_eq!(path, dir.join("server/data/lua/scriptPolicy.json"));
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(json["default"], "client");
        assert_eq!(json["scripts"]["scripts/spawner/world.lua"], "authority");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
