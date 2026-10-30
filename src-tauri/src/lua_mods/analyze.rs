//! Suggests where a TES3MP-NG global Lua script should run, from what its code does.
//!
//! In TES3MP-NG, a global script in `client` mode runs on every player's game, like mwscript
//! globals: events (activation, item use, a player's own events) reach the game of the player
//! who caused them, so a change made in response happens once and is synced. A script that changes
//! the shared world on its own (from `onUpdate`, timers, objects becoming active) would make the
//! same change once per player in `client` mode, so it should run once, on the `authority`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::instance_settings::ScriptMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SuggestionConfidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ScriptModeSuggestion {
    pub mode: ScriptMode,
    pub confidence: SuggestionConfidence,
    /// What the code does that led to the suggestion.
    pub reasons: Vec<String>,
    /// Things that don't work the same in multiplayer.
    pub warnings: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// Changes the shared world
    WorldChange,
    /// Runs without a player causing it
    Autonomous,
    /// Reacts to something a player did
    PlayerHandler,
    Warning,
}

struct Signal {
    patterns: &'static [&'static str],
    kind: Kind,
    description: &'static str,
}

const SIGNALS: &[Signal] = &[
    Signal { patterns: &["world.createObject"], kind: Kind::WorldChange, description: "Places objects (world.createObject)" },
    Signal { patterns: &[":teleport("], kind: Kind::WorldChange, description: "Moves objects or actors (teleport)" },
    Signal { patterns: &[":remove("], kind: Kind::WorldChange, description: "Removes objects (remove)" },
    Signal { patterns: &[":moveInto("], kind: Kind::WorldChange, description: "Moves items into containers (moveInto)" },
    Signal { patterns: &[":setScale("], kind: Kind::WorldChange, description: "Rescales objects (setScale)" },
    Signal { patterns: &[".enabled =", ".enabled="], kind: Kind::WorldChange, description: "Enables or disables objects" },
    Signal { patterns: &["world.setGameTime", "world.setGameTimeScale", "world.setSimulationTimeScale"], kind: Kind::WorldChange, description: "Changes the game time" },
    Signal { patterns: &["getGlobalVariables"], kind: Kind::WorldChange, description: "Changes mwscript global variables" },
    Signal { patterns: &["Lockable.lock", "Lockable.unlock", "setTrapSpell"], kind: Kind::WorldChange, description: "Locks, unlocks or traps objects" },
    Signal { patterns: &["setBarterGold"], kind: Kind::WorldChange, description: "Changes merchants' gold" },
    Signal { patterns: &["onUpdate"], kind: Kind::Autonomous, description: "Runs every frame (onUpdate)" },
    Signal { patterns: &["newSimulationTimer", "newGameTimer", "newUnsavableSimulationTimer", "newUnsavableGameTimer"], kind: Kind::Autonomous, description: "Uses timers" },
    Signal { patterns: &["onObjectActive", "onActorActive"], kind: Kind::Autonomous, description: "Reacts to objects becoming active" },
    Signal { patterns: &["world.activeActors"], kind: Kind::Autonomous, description: "Goes through all active actors" },
    Signal { patterns: &["I.Activation", "addHandlerForType", "addHandlerForObject"], kind: Kind::PlayerHandler, description: "Handles activation or item use" },
    Signal { patterns: &["I.ItemUsage"], kind: Kind::PlayerHandler, description: "Handles item use" },
    Signal { patterns: &["onActivate"], kind: Kind::PlayerHandler, description: "Reacts to activation (onActivate)" },
    Signal { patterns: &["onPlayerAdded"], kind: Kind::PlayerHandler, description: "Reacts to a player joining (onPlayerAdded)" },
    Signal { patterns: &["world.createRecord"], kind: Kind::Warning, description: "Creates records (world.createRecord): objects made from them aren't synced to other players" },
    Signal { patterns: &["world.players"], kind: Kind::Warning, description: "Uses world.players, which only lists the local player in TES3MP-NG" },
];

/// Most files read through `require()` for one script.
const MAX_FILES: usize = 25;

/// Suggest a mode for the global script at `script` (a VFS path such as `scripts/mod/world.lua`).
/// `data_paths` are the instance's data folders in load order; later ones override earlier ones.
pub fn suggest_script_mode(script: &str, data_paths: &[PathBuf]) -> Option<ScriptModeSuggestion> {
    let code = read_script_with_requires(script, data_paths)?;
    Some(suggest_from_code(&code))
}

fn suggest_from_code(code: &str) -> ScriptModeSuggestion {
    let code = strip_comments(code);
    let mut found: Vec<&Signal> = SIGNALS
        .iter()
        .filter(|signal| signal.patterns.iter().any(|pattern| code.contains(pattern)))
        .collect();
    // Writes to shared storage: a global section and a set() call
    let storage_write = code.contains("globalSection(") && code.contains(":set(");
    let has = |found: &[&Signal], kind: Kind| found.iter().any(|signal| signal.kind == kind);

    let world_change = has(&found, Kind::WorldChange) || storage_write;
    let autonomous = has(&found, Kind::Autonomous);
    let player_handler = has(&found, Kind::PlayerHandler);

    let mut reasons: Vec<String> = Vec::new();
    for kind in [Kind::WorldChange, Kind::Autonomous, Kind::PlayerHandler] {
        for signal in found.iter().filter(|signal| signal.kind == kind) {
            reasons.push(signal.description.to_string());
        }
        if kind == Kind::WorldChange && storage_write {
            reasons.push("Writes shared global storage".to_string());
        }
    }
    found.retain(|signal| signal.kind == Kind::Warning);
    let warnings = found.iter().map(|signal| signal.description.to_string()).collect();

    let (mode, confidence) = match (world_change, autonomous) {
        (true, true) => {
            reasons.push("Changes the shared world on its own: run it once, on the authority".into());
            let confidence = if player_handler { SuggestionConfidence::Medium } else { SuggestionConfidence::High };
            (ScriptMode::Authority, confidence)
        }
        (true, false) => {
            reasons.push("Only changes the world in response to events: the game of the player who caused them makes the change once".into());
            (ScriptMode::Client, SuggestionConfidence::Medium)
        }
        (false, true) => {
            reasons.push("Runs on its own, but no changes to the shared world were found".into());
            (ScriptMode::Client, SuggestionConfidence::Low)
        }
        (false, false) => {
            reasons.push("No changes to the shared world were found".into());
            let confidence = if player_handler { SuggestionConfidence::High } else { SuggestionConfidence::Medium };
            (ScriptMode::Client, confidence)
        }
    };

    ScriptModeSuggestion { mode, confidence, reasons, warnings }
}

/// The script's code, followed by the modules it requires from data folders.
fn read_script_with_requires(script: &str, data_paths: &[PathBuf]) -> Option<String> {
    let first = resolve_vfs_path(script, data_paths)?;
    let mut code = String::new();
    let mut queue = vec![first];
    let mut seen: HashSet<PathBuf> = HashSet::new();

    while let Some(path) = queue.pop() {
        if seen.len() >= MAX_FILES || !seen.insert(path.clone()) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for module in required_modules(&text) {
            let module_path = format!("{}.lua", module.replace('.', "/"));
            if let Some(resolved) = resolve_vfs_path(&module_path, data_paths) {
                queue.push(resolved);
            }
        }
        code.push_str(&text);
        code.push('\n');
    }
    Some(code)
}

/// A VFS path in the data folders, the last folder that has it winning.
pub fn resolve_vfs_path(vfs_path: &str, data_paths: &[PathBuf]) -> Option<PathBuf> {
    let relative = vfs_path.trim().trim_start_matches(['/', '\\']).replace('\\', "/");
    data_paths
        .iter()
        .rev()
        .map(|dir| dir.join(&relative))
        .find(|path| path.is_file())
}

/// Module names passed to `require`, except OpenMW's own (`openmw.*`, `openmw_aux.*`).
fn required_modules(code: &str) -> Vec<String> {
    let mut modules = Vec::new();
    let mut rest = code;
    while let Some(index) = rest.find("require") {
        rest = &rest[index + "require".len()..];
        let trimmed = rest.trim_start_matches(|c: char| c == '(' || c.is_whitespace());
        let Some(quote) = trimmed.chars().next().filter(|c| *c == '\'' || *c == '"') else {
            continue;
        };
        let body = &trimmed[1..];
        let Some(end) = body.find(quote) else {
            continue;
        };
        let module = &body[..end];
        if !module.is_empty() && !module.starts_with("openmw") {
            modules.push(module.to_string());
        }
    }
    modules
}

/// Drop Lua comments (`-- line` and `--[[ block ]]`) so commented-out code doesn't count.
fn strip_comments(code: &str) -> String {
    let mut out = String::with_capacity(code.len());
    let mut rest = code;
    while let Some(index) = rest.find("--") {
        out.push_str(&rest[..index]);
        let after = &rest[index + 2..];
        if let Some(block) = after.strip_prefix("[[") {
            rest = block.find("]]").map(|end| &block[end + 2..]).unwrap_or("");
        } else {
            rest = after.find('\n').map(|end| &after[end..]).unwrap_or("");
        }
    }
    out.push_str(rest);
    out
}

pub fn strip_quotes(path: &str) -> &str {
    let trimmed = path.trim();
    trimmed
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .unwrap_or(trimmed)
}

pub fn data_path_buf(path: &str) -> PathBuf {
    Path::new(strip_quotes(path)).to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawners_should_run_on_the_authority() {
        let code = r#"
            local world = require('openmw.world')
            local time = require('openmw_aux.time')
            time.runRepeatedly(function() end, 60)
            return { engineHandlers = { onUpdate = function(dt)
                local guard = world.createObject('imperial guard', 1)
                guard:teleport('Balmora', util.vector3(0, 0, 0))
            end } }
        "#;
        let suggestion = suggest_from_code(code);
        assert_eq!(suggestion.mode, ScriptMode::Authority);
        assert_eq!(suggestion.confidence, SuggestionConfidence::High);
        assert!(suggestion.reasons.iter().any(|r| r.contains("world.createObject")));
    }

    #[test]
    fn activation_handlers_should_run_on_clients() {
        let code = r#"
            local I = require('openmw.interfaces')
            I.Activation.addHandlerForType(types.Container, function(object, actor)
                object:remove(1)
            end)
        "#;
        let suggestion = suggest_from_code(code);
        assert_eq!(suggestion.mode, ScriptMode::Client);
        assert!(suggestion.reasons.iter().any(|r| r.contains("in response to events")));
    }

    #[test]
    fn commented_out_code_and_warnings() {
        let code = "-- world.createObject('x')\n--[[ onUpdate ]]\nlocal p = world.players\nworld.createRecord(r)";
        let suggestion = suggest_from_code(code);
        assert_eq!(suggestion.mode, ScriptMode::Client);
        assert!(!suggestion.reasons.iter().any(|r| r.contains("Places objects")));
        assert_eq!(suggestion.warnings.len(), 2);
    }

    #[test]
    fn shared_storage_writes_from_timers_go_to_the_authority() {
        let code = "local section = storage.globalSection('Economy')\nasync:newSimulationTimer(5, cb)\nsection:set('prices', prices)";
        assert_eq!(suggest_from_code(code).mode, ScriptMode::Authority);
    }

    #[test]
    fn follows_requires_into_mod_modules() {
        let dir = std::env::temp_dir().join(format!("nerevar-lua-requires-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("scripts/mod")).unwrap();
        std::fs::write(
            dir.join("scripts/mod/world.lua"),
            "local spawn = require('scripts.mod.spawn')\nlocal core = require('openmw.core')\nreturn { engineHandlers = { onUpdate = spawn.tick } }",
        )
        .unwrap();
        std::fs::write(dir.join("scripts/mod/spawn.lua"), "return { tick = function() world.createObject('x') end }").unwrap();

        let suggestion = suggest_script_mode("scripts/mod/world.lua", &[dir.clone()]).unwrap();
        assert_eq!(suggestion.mode, ScriptMode::Authority);
        assert!(suggest_script_mode("scripts/missing.lua", &[dir.clone()]).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
