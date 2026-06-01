use std::cmp::Ordering;
use std::path::PathBuf;
use std::process::Command;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_log::log::info;
#[cfg(not(windows))]
use tauri_plugin_log::log::error;
use ts_rs::TS;

use crate::data::{GithubAssetResponse, GithubReleaseResponse};
use crate::github_getters;

pub const NEREVAR_REPO: &str = "kyaustad/nerevar-rs";

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppUpdateRelease {
    pub id: u64,
    pub tag_name: String,
    pub name: String,
    pub version: String,
    pub body: String,
    pub published_at: String,
    pub html_url: String,
    pub installer_asset_name: String,
    pub installer_download_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppUpdateStatus {
    pub current_version: String,
    pub update_available: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_release: Option<AppUpdateRelease>,
}

pub fn current_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

pub async fn check_for_update() -> Result<AppUpdateStatus, String> {
    let current_version = current_app_version();
    let releases = github_getters::get_nerevar_releases().await?;

    let Some(latest) = find_latest_release(&releases) else {
        return Ok(AppUpdateStatus {
            current_version,
            update_available: false,
            latest_release: None,
        });
    };

    let latest_version = release_version(latest)
        .ok_or_else(|| format!("Could not parse version for release {}", latest.tag_name))?;
    let update_available = version_compare(&latest_version, &current_version) == Ordering::Greater;

    let latest_release = if update_available {
        Some(to_update_release(latest)?)
    } else {
        None
    };

    Ok(AppUpdateStatus {
        current_version,
        update_available,
        latest_release,
    })
}

pub async fn download_and_run_installer(release_id: u64, app: AppHandle) -> Result<(), String> {
    let releases = github_getters::get_nerevar_releases().await?;
    let release = releases
        .iter()
        .find(|release| release.id == release_id)
        .ok_or_else(|| format!("Release {release_id} not found"))?;

    let asset = find_installer_asset(release)
        .ok_or_else(|| format!("No Windows installer (.exe) found for release {release_id}"))?;

    info!(
        "Downloading Nerevar update {} ({})",
        release.tag_name, asset.name
    );

    let client = Client::new();
    let bytes = client
        .get(&asset.browser_download_url)
        .header(
            "User-Agent",
            format!("Nerevar-{}", current_app_version()),
        )
        .send()
        .await
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;

    let temp_dir = std::env::temp_dir().join(format!("nerevar-update-{release_id}"));
    std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
    let installer_path: PathBuf = temp_dir.join(&asset.name);
    std::fs::write(&installer_path, bytes).map_err(|e| e.to_string())?;

    spawn_installer_detached(&installer_path)?;
    info!(
        "Launched installer at {} and exiting Nerevar",
        installer_path.display()
    );
    app.exit(0);
    Ok(())
}

fn to_update_release(release: &GithubReleaseResponse) -> Result<AppUpdateRelease, String> {
    let version = release_version(release)
        .ok_or_else(|| format!("Could not parse version for release {}", release.tag_name))?;
    let asset = find_installer_asset(release)
        .ok_or_else(|| format!("No Windows installer found for release {}", release.tag_name))?;

    Ok(AppUpdateRelease {
        id: release.id,
        tag_name: release.tag_name.clone(),
        name: release.name.clone(),
        version,
        body: release.body.clone(),
        published_at: release.published_at.clone(),
        html_url: release.html_url.clone(),
        installer_asset_name: asset.name.clone(),
        installer_download_url: asset.browser_download_url.clone(),
    })
}

fn find_latest_release(releases: &[GithubReleaseResponse]) -> Option<&GithubReleaseResponse> {
    releases
        .iter()
        .filter(|release| !release.draft && !release.prerelease)
        .filter_map(|release| release_version(release).map(|version| (release, version)))
        .max_by(|(_, left), (_, right)| version_compare(left, right))
        .map(|(release, _)| release)
}

fn find_installer_asset(release: &GithubReleaseResponse) -> Option<&GithubAssetResponse> {
    release.assets.iter().find(|asset| {
        let name = asset.name.to_ascii_lowercase();
        name.ends_with(".exe")
            && (name.contains("setup") || name.contains("installer") || name.contains("x64"))
    })
}

fn release_version(release: &GithubReleaseResponse) -> Option<String> {
    parse_semver_from_tag(&release.tag_name).or_else(|| parse_semver_from_name(&release.name))
}

fn parse_semver_from_tag(tag: &str) -> Option<String> {
    let trimmed = tag.trim().trim_start_matches(['v', 'V']);
    let core = trimmed.split('-').next()?.trim();
    parse_version(core).map(|_| core.to_string())
}

fn parse_semver_from_name(name: &str) -> Option<String> {
    for token in name.split_whitespace() {
        if let Some(version) = parse_semver_from_tag(token) {
            return Some(version);
        }
    }
    None
}

fn parse_version(value: &str) -> Option<(u64, u64, u64)> {
    let mut parts = value.split('.');
    Some((
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    ))
}

fn version_compare(left: &str, right: &str) -> Ordering {
    match (parse_version(left), parse_version(right)) {
        (Some(left), Some(right)) => left.cmp(&right),
        _ => Ordering::Equal,
    }
}

fn spawn_installer_detached(path: &PathBuf) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x00000008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;

        Command::new(path)
            .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
            .spawn()
            .map_err(|e| format!("Failed to launch installer: {e}"))?;
        return Ok(());
    }

    #[cfg(not(windows))]
    {
        error!("Nerevar in-app updates are only supported on Windows");
        Err("In-app updates are only supported on Windows".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_semver_versions() {
        assert_eq!(version_compare("0.1.4", "0.1.3"), Ordering::Greater);
        assert_eq!(version_compare("0.1.4", "0.1.4"), Ordering::Equal);
        assert_eq!(version_compare("0.1.3", "0.2.0"), Ordering::Less);
    }

    #[test]
    fn parses_version_from_tag_and_name() {
        assert_eq!(parse_semver_from_tag("v0.1.4"), Some("0.1.4".into()));
        assert_eq!(
            parse_semver_from_name("Nerevar v0.1.4"),
            Some("0.1.4".into())
        );
    }
}
