use super::types::{
    InstanceSettings, SettingCategory, SettingDefinition, SettingValue, SettingValueType,
    Tes3mpGameSettingEntry, INSTANCE_SETTINGS_VERSION,
};

fn bool_def(
    key: &str,
    category: SettingCategory,
    label: &str,
    description: &str,
    default: bool,
) -> SettingDefinition {
    SettingDefinition {
        key: key.to_string(),
        category,
        value_type: SettingValueType::Boolean,
        label: label.to_string(),
        description: description.to_string(),
        default_value: SettingValue::Boolean(default),
        min_integer: None,
        max_integer: None,
    }
}

fn int_def(
    key: &str,
    category: SettingCategory,
    label: &str,
    description: &str,
    default: i64,
    min: i64,
    max: i64,
) -> SettingDefinition {
    SettingDefinition {
        key: key.to_string(),
        category,
        value_type: SettingValueType::Integer,
        label: label.to_string(),
        description: description.to_string(),
        default_value: SettingValue::Integer(default),
        min_integer: Some(min),
        max_integer: Some(max),
    }
}

fn string_def(
    key: &str,
    category: SettingCategory,
    label: &str,
    description: &str,
    default: &str,
) -> SettingDefinition {
    SettingDefinition {
        key: key.to_string(),
        category,
        value_type: SettingValueType::String,
        label: label.to_string(),
        description: description.to_string(),
        default_value: SettingValue::String(default.to_string()),
        min_integer: None,
        max_integer: None,
    }
}

/// Curated settings exposed in the server management UI.
///
/// Gameplay mechanics are written to TES3MP `config.lua` (`config.gameSettings`) and enforced
/// by the game server when players connect. Client graphics/shaders settings are written to
/// an instance launch overlay for `settings.cfg` and applied at TES3MP client launch.
pub fn setting_definitions() -> Vec<SettingDefinition> {
    let tes3mp = SettingCategory::Tes3mpGameplay;
    let graphics = SettingCategory::OpenMwGraphics;
    let shaders = SettingCategory::OpenMwShaders;

    vec![
        // TES3MP-enforced gameplay (OpenMW Game category)
        bool_def(
            "best attack",
            tes3mp,
            "Always use best attack",
            "Use the strongest attack type (chop/slash/thrust) automatically.",
            false,
        ),
        bool_def(
            "prevent merchant equipping",
            tes3mp,
            "Prevent merchant equipping",
            "Stop merchants from equipping items sold to them.",
            false,
        ),
        bool_def(
            "enchanted weapons are magical",
            tes3mp,
            "Enchanted weapons are magical",
            "Enchanted weapons bypass normal weapon resistance on certain creatures.",
            true,
        ),
        bool_def(
            "barter disposition change is permanent",
            tes3mp,
            "Permanent barter disposition",
            "Disposition changes from trading persist after dialogue ends.",
            false,
        ),
        int_def(
            "strength influences hand to hand",
            tes3mp,
            "Strength influences hand to hand",
            "0 = ignore Strength, 1 = all actors, 2 = all except werewolves.",
            0,
            0,
            2,
        ),
        bool_def(
            "normalise race speed",
            tes3mp,
            "Normalise race speed",
            "Ignore race weight when calculating movement speed.",
            false,
        ),
        bool_def(
            "uncapped damage fatigue",
            tes3mp,
            "Uncapped damage fatigue",
            "Allow Damage Fatigue effects to reduce fatigue below zero.",
            false,
        ),
        bool_def(
            "NPCs avoid collisions",
            tes3mp,
            "NPCs avoid collisions",
            "NPCs attempt to avoid colliding with each other.",
            false,
        ),
        bool_def(
            "swim upward correction",
            tes3mp,
            "Swim upward correction",
            "Third-person swimming bias toward the surface.",
            false,
        ),
        bool_def(
            "trainers training skills based on base skill",
            tes3mp,
            "Trainers use base skill",
            "Trainer skill limits use base skill instead of fortified/drained values.",
            true,
        ),
        bool_def(
            "always allow stealing from knocked out actors",
            tes3mp,
            "Steal from knocked-out actors",
            "Allow pickpocketing actors knocked out during combat.",
            false,
        ),
        bool_def(
            "use magic item animations",
            tes3mp,
            "Magic item animations",
            "Play casting animations when using enchanted items and scrolls.",
            false,
        ),
        bool_def(
            "use additional anim sources",
            tes3mp,
            "Additional animation sources",
            "Load extra KF animation files from Animations/ folders (common for animation replacers).",
            false,
        ),
        bool_def(
            "shield sheathing",
            tes3mp,
            "Shield sheathing",
            "Display holstered shields when using compatible _sh assets.",
            false,
        ),
        bool_def(
            "weapon sheathing",
            tes3mp,
            "Weapon sheathing",
            "Display holstered weapons when using compatible _sh assets.",
            false,
        ),
        bool_def(
            "graphic herbalism",
            tes3mp,
            "Graphic herbalism",
            "Harvest plants visually when mods provide harvestable container models.",
            true,
        ),
        // Client graphics — synced via settings.cfg overlay
        bool_def(
            "auto use object normal maps",
            shaders,
            "Auto object normal maps",
            "Detect normal maps from filename patterns (e.g. foo_n.dds).",
            false,
        ),
        bool_def(
            "auto use object specular maps",
            shaders,
            "Auto object specular maps",
            "Detect specular maps from filename patterns (e.g. foo_spec.dds).",
            false,
        ),
        bool_def(
            "auto use terrain normal maps",
            shaders,
            "Auto terrain normal maps",
            "Auto-detect terrain normal maps from filename patterns.",
            false,
        ),
        bool_def(
            "auto use terrain specular maps",
            shaders,
            "Auto terrain specular maps",
            "Auto-detect terrain specular/diffuse-spec maps.",
            false,
        ),
        bool_def(
            "force shaders",
            shaders,
            "Force shaders",
            "Render with shaders on all objects (not just bump-mapped).",
            false,
        ),
        bool_def(
            "clamp lighting",
            shaders,
            "Clamp lighting",
            "Cap per-object lighting to (1,1,1) for Morrowind-like lighting.",
            true,
        ),
        bool_def(
            "radial fog",
            shaders,
            "Radial fog",
            "Use eye-distance fog instead of clip-plane fog (wide FOV).",
            false,
        ),
        string_def(
            "lighting method",
            shaders,
            "Lighting method",
            "legacy, shaders compatibility, or shaders.",
            "default",
        ),
        string_def(
            "normal map pattern",
            shaders,
            "Normal map pattern",
            "Filename suffix for auto-detected normal maps.",
            "_n",
        ),
        string_def(
            "specular map pattern",
            shaders,
            "Specular map pattern",
            "Filename suffix for auto-detected specular maps.",
            "_spec",
        ),
        bool_def(
            "apply lighting to environment maps",
            shaders,
            "Lighting on environment maps",
            "Environment-mapped objects receive scene lighting.",
            false,
        ),
        // Graphics section
        bool_def(
            "distant terrain",
            graphics,
            "Distant terrain",
            "Enable OpenMW distant terrain rendering.",
            false,
        ),
        bool_def(
            "distant statics",
            graphics,
            "Distant statics",
            "Render static objects in distant terrain cells.",
            false,
        ),
        bool_def(
            "antialias",
            graphics,
            "Anti-aliasing",
            "Enable MSAA anti-aliasing.",
            false,
        ),
    ]
}

pub fn default_instance_settings() -> InstanceSettings {
    let definitions = setting_definitions();
    let mut tes3mp_game_settings = Vec::new();
    let mut openmw_settings: std::collections::BTreeMap<
        String,
        std::collections::BTreeMap<String, SettingValue>,
    > = std::collections::BTreeMap::new();

    for definition in definitions {
        match definition.category {
            SettingCategory::Tes3mpGameplay => {
                tes3mp_game_settings.push(Tes3mpGameSettingEntry {
                    name: definition.key.clone(),
                    value: definition.default_value.clone(),
                });
            }
            SettingCategory::OpenMwGraphics => {
                openmw_settings
                    .entry("Graphics".to_string())
                    .or_default()
                    .insert(definition.key.clone(), definition.default_value.clone());
            }
            SettingCategory::OpenMwShaders => {
                openmw_settings
                    .entry("Shaders".to_string())
                    .or_default()
                    .insert(definition.key.clone(), definition.default_value.clone());
            }
        }
    }

    InstanceSettings {
        version: INSTANCE_SETTINGS_VERSION,
        tes3mp_game_settings,
        openmw_settings,
        openmw_cfg_overrides: Vec::new(),
        openmw_settings_cfg_overrides: String::new(),
    }
}

pub fn normalize_instance_settings(mut settings: InstanceSettings) -> InstanceSettings {
    let defaults = default_instance_settings();

    settings.openmw_cfg_overrides = settings
        .openmw_cfg_overrides
        .iter()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();

    for default_entry in &defaults.tes3mp_game_settings {
        if !settings
            .tes3mp_game_settings
            .iter()
            .any(|entry| entry.name == default_entry.name)
        {
            settings.tes3mp_game_settings.push(default_entry.clone());
        }
    }

    for (section, entries) in defaults.openmw_settings {
        let target = settings.openmw_settings.entry(section).or_default();
        for (key, value) in entries {
            target.entry(key).or_insert(value);
        }
    }

    settings
}
