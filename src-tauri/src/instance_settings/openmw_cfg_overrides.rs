use crate::openmw_ini_importer::MultiStrMap;

pub fn apply_openmw_cfg_override_lines(cfg: &mut MultiStrMap, lines: &[String]) {
    for line in lines {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        cfg.entry(key.trim().to_string())
            .or_default()
            .push(value.trim().to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_key_value_lines_to_cfg_map() {
        let mut cfg = MultiStrMap::new();
        apply_openmw_cfg_override_lines(
            &mut cfg,
            &[
                "groundcover=Mod.esp".into(),
                "# comment".into(),
                "  content=Extra.esp  ".into(),
            ],
        );
        assert_eq!(
            cfg.get("groundcover").map(|values| values.as_slice()),
            Some(["Mod.esp".to_string()].as_slice())
        );
        assert_eq!(
            cfg.get("content").map(|values| values.as_slice()),
            Some(["Extra.esp".to_string()].as_slice())
        );
    }
}
