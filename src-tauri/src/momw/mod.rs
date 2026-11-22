//! Modding-OpenMW.com modlists for instances, like the umo command line tool
//! (https://modding-openmw.gitlab.io/umo/): the list's mods are downloaded (Nexus Mods with a Premium account,
//! Modding-OpenMW's GitLab projects, direct links), extracted into the instance's data folder the way umo lays them
//! out (`<category>/<mod folder>`), and the load order, content files and settings come from the site's config
//! generator, which momw-configurator uses too.

pub mod actions;
pub mod archive;
pub mod commands;
pub mod install;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const MOMW_HOST: &str = "https://modding-openmw.com";
pub const MOMW_BETA_HOST: &str = "https://beta.modding-openmw.com";
/// Mod folder base path in the config generator's output, which momw-configurator replaces with the real one.
pub const CFG_BASE_DIR: &str = r"C:\games\OpenMWMods";

pub fn host(beta: bool) -> &'static str {
    if beta { MOMW_BETA_HOST } else { MOMW_HOST }
}

/// A file action of a mod's download, run after extracting it (umo's `actions`). Paths are relative to the mod's
/// category folder.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "kebab-case")]
pub enum ModAction {
    Remove {
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        paths: Option<Vec<String>>,
    },
    Rename {
        src: String,
        dst: String,
    },
    Copy {
        #[serde(default)]
        src: Option<String>,
        #[serde(default)]
        srcs: Option<Vec<String>>,
        dst: String,
        #[serde(default)]
        force: Option<bool>,
    },
    CopyRename {
        files: Vec<CopyRenameFile>,
        #[serde(default)]
        strip_prefix: Option<String>,
        dst: String,
        #[serde(default)]
        force: Option<bool>,
    },
    /// tes3cmd cleaning of a plugin; Nerevar doesn't run tes3cmd
    Clean {
        path: String,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyRenameFile {
    pub src: String,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadInfo {
    #[serde(default)]
    pub direct_download: Option<String>,
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub extract_to: Option<String>,
    #[serde(default)]
    pub nexus_file_id: Option<u64>,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub actions: Vec<ModAction>,
}

/// A mod of a list, from `/api/lists/{slug}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MomwMod {
    pub name: String,
    pub url: String,
    pub category: String,
    #[serde(default)]
    pub dl_url: String,
    pub dir: String,
    pub slug: String,
    #[serde(default)]
    pub usage_notes: Option<String>,
    #[serde(default)]
    pub download_info: Vec<DownloadInfo>,
    #[serde(default)]
    pub data_paths: Vec<String>,
    #[serde(default)]
    pub plugins: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GeneratedOpenMwCfg {
    #[serde(default, rename = "fallback values")]
    pub fallback_values: String,
    #[serde(default, rename = "data paths")]
    pub data_paths: String,
    #[serde(default, rename = "fallback archives")]
    pub fallback_archives: String,
    #[serde(default, rename = "content files")]
    pub content_files: String,
    #[serde(default, rename = "groundcover files")]
    pub groundcover_files: String,
}

/// The config of a list, from `/api/cfg-generator/{slug}`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct GeneratedConfig {
    pub openmw_cfg: GeneratedOpenMwCfg,
    #[serde(default)]
    pub settings_cfg: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MomwModlistSummary {
    pub slug: String,
    pub beta: bool,
}

fn http() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(crate::nexus::user_agent())
        .build()
        .map_err(|e| e.to_string())
}

async fn get_json<T: for<'de> Deserialize<'de>>(url: &str) -> Result<T, String> {
    let response = http()?.get(url).send().await.map_err(|e| format!("{url}: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("{url} returned {}", response.status()));
    }
    response.json::<T>().await.map_err(|e| format!("Unexpected answer from {url}: {e}"))
}

pub async fn fetch_modlists(beta: bool) -> Result<Vec<String>, String> {
    get_json(&format!("{}/api/lists", host(beta))).await
}

pub async fn fetch_modlist(slug: &str, beta: bool) -> Result<Vec<MomwMod>, String> {
    get_json(&format!("{}/api/lists/{}", host(beta), urlencoding::encode(slug))).await
}

pub async fn fetch_generated_config(slug: &str, beta: bool) -> Result<GeneratedConfig, String> {
    get_json(&format!("{}/api/cfg-generator/{}", host(beta), urlencoding::encode(slug))).await
}

/// Values of `key=` lines of a config generator block, unquoted.
pub fn cfg_values<'a>(block: &'a str, key: &str) -> Vec<&'a str> {
    block
        .lines()
        .filter_map(|line| line.trim().strip_prefix(key)?.strip_prefix('='))
        .map(|value| value.trim().trim_matches('"'))
        .filter(|value| !value.is_empty())
        .collect()
}

/// A data path of the config generator relative to the mod folder base, with forward slashes, or None when it
/// isn't under the base (Morrowind's Data Files are handled separately).
pub fn relative_data_path(path: &str) -> Option<String> {
    let normalized = path.replace('/', "\\");
    let rest = normalized
        .get(..CFG_BASE_DIR.len())
        .filter(|prefix| prefix.eq_ignore_ascii_case(CFG_BASE_DIR))
        .map(|_| &normalized[CFG_BASE_DIR.len()..])?;
    let rest = rest.trim_start_matches('\\');
    if rest.is_empty() {
        return None;
    }
    Some(rest.replace('\\', "/"))
}

/// The config generator's stand-in for Morrowind's own Data Files folder.
pub fn is_base_game_data_path(path: &str) -> bool {
    relative_data_path(path).is_some_and(|rel| rel.eq_ignore_ascii_case("ModdingResources/Morrowind/Data Files"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_generated_data_paths() {
        assert_eq!(
            relative_data_path(r"C:\games\OpenMWMods\OfficialPlugins\UMOPP\08 UMOPP Merged\00 AIO Core").as_deref(),
            Some("OfficialPlugins/UMOPP/08 UMOPP Merged/00 AIO Core")
        );
        assert!(is_base_game_data_path(r"C:\games\OpenMWMods\ModdingResources\Morrowind\Data Files"));
        assert_eq!(relative_data_path(r"D:\Other\Mod"), None);
        let block = "data=C:\\games\\OpenMWMods\\A\\B\ndata=\"C:\\games\\OpenMWMods\\C\"\ncontent=x.esp";
        assert_eq!(cfg_values(block, "data"), vec!["C:\\games\\OpenMWMods\\A\\B", "C:\\games\\OpenMWMods\\C"]);
        assert_eq!(cfg_values(block, "content"), vec!["x.esp"]);
    }

    #[test]
    fn parses_list_entries() {
        let json = r#"[{"name":"M","url":"https://www.nexusmods.com/morrowind/mods/1","category":"Cat","dl_url":"",
            "dir":"MDir","slug":"m","usage_notes":null,"data_paths":["MDir"],"plugins":["m.esp"],
            "download_info":[{"direct_download":null,"file_name":"Main","extract_to":"MDir","nexus_file_id":5,
            "pinned":false,"actions":[{"action":"remove","path":"MDir/x"},{"action":"copy","src":"a","dst":"b"},
            {"action":"clean","path":"MDir/m.esp"},{"action":"something-new"}]}]}]"#;
        let mods: Vec<MomwMod> = serde_json::from_str(json).unwrap();
        assert_eq!(mods[0].download_info[0].nexus_file_id, Some(5));
        assert_eq!(mods[0].download_info[0].actions.len(), 4);
        assert!(matches!(mods[0].download_info[0].actions[3], ModAction::Unknown));
    }
}
