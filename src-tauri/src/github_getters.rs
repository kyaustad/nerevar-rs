use crate::data::{GithubAssetResponse, GithubReleaseResponse};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{copy, Cursor};
use std::path::{Component, Path, PathBuf};
use tauri_plugin_log::log::{error, info};

/// TES3MP-NG: TES3MP rebased on OpenMW 0.51. Its Windows releases are portable zips
/// (`TES3MP-NG-<version>-win64.zip`) with a `SHA256SUMS.txt` asset.
pub const TES3MP_NG_REPO: &str = "kyaustad/TES3MP-NG";
const CLIENT_ASSET_PREFIX: &str = "TES3MP-NG-";
const CLIENT_ASSET_SUFFIX: &str = "-win64.zip";
const CHECKSUMS_ASSET: &str = "SHA256SUMS.txt";

/// Folders of an installed release that belong to the player or the server, not the release:
/// the portable `config/` and `userdata/` folders, and the server's world data.
const PRESERVED_ON_UPDATE: &[&str] = &["config", "userdata"];
const SERVER_DIR: &str = "server";
const SERVER_DATA_DIR: &str = "data";

/// TES3MP-NG releases that have a Windows client package, newest first.
pub async fn get_all_releases() -> Result<Vec<GithubReleaseResponse>, String> {
    let releases = fetch_github_releases(TES3MP_NG_REPO, "TES3MP-NG").await?;
    Ok(releases
        .into_iter()
        .filter(|release| !release.draft && find_client_asset(release).is_some())
        .collect())
}

pub async fn get_nerevar_releases() -> Result<Vec<GithubReleaseResponse>, String> {
    fetch_github_releases(crate::app_update::NEREVAR_REPO, "Nerevar").await
}

async fn fetch_github_releases(
    repo: &str,
    label: &str,
) -> Result<Vec<GithubReleaseResponse>, String> {
    info!("Attempting to fetch all {label} releases from GitHub...");
    let client = Client::new();
    let url = format!("https://api.github.com/repos/{repo}/releases");

    let response = match client
        .get(url)
        .header("User-Agent", user_agent())
        .send()
        .await
    {
        Ok(res) => res,
        Err(e) => {
            error!("fetch_github_releases request failed: {e}");
            return Err(e.to_string());
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        error!("fetch_github_releases HTTP {status}: {body}");
        return Err(format!("GitHub API returned HTTP {status}: {body}"));
    }

    match response.json::<Vec<GithubReleaseResponse>>().await {
        Ok(releases) => {
            info!("Successfully fetched all {label} releases from GitHub");
            Ok(releases)
        }
        Err(e) => {
            error!("fetch_github_releases JSON parsing failed: {e}");
            Err(e.to_string())
        }
    }
}

fn user_agent() -> String {
    format!("Nerevar-{}", env!("CARGO_PKG_VERSION"))
}

/// The client package of a release: `TES3MP-NG-<version>-win64.zip`, not the master server or
/// symbols packages.
pub fn find_client_asset(release: &GithubReleaseResponse) -> Option<&GithubAssetResponse> {
    release.assets.iter().find(|asset| is_client_asset_name(&asset.name))
}

fn is_client_asset_name(name: &str) -> bool {
    name.starts_with(CLIENT_ASSET_PREFIX)
        && name.ends_with(CLIENT_ASSET_SUFFIX)
        && !name.contains("MasterServer")
        && !name.contains("symbols")
}

pub async fn find_release_by_id(release_id: &str) -> Result<GithubReleaseResponse, String> {
    let release_id_u64: u64 = release_id
        .parse()
        .map_err(|_| format!("Invalid release id: {release_id}"))?;
    get_all_releases()
        .await?
        .into_iter()
        .find(|release| release.id == release_id_u64)
        .ok_or_else(|| format!("TES3MP-NG release with id {release_id} not found"))
}

pub async fn find_release_by_tag(tag: &str) -> Result<GithubReleaseResponse, String> {
    get_all_releases()
        .await?
        .into_iter()
        .find(|release| release.tag_name == tag)
        .ok_or_else(|| format!("TES3MP-NG release {tag} not found on GitHub"))
}

pub async fn download_and_extract_release_zip_by_id_to_path(
    release_id: String,
    path: String,
) -> Result<GithubReleaseResponse, String> {
    let release = find_release_by_id(&release_id).await?;
    install_release(&release, Path::new(&path)).await?;
    Ok(release)
}

/// Download a release's client package, verify it against `SHA256SUMS.txt` and install it into
/// `dest`, which becomes the portable TES3MP-NG folder (`openmw.exe`, `config/`, `server/`...).
/// Installing over an existing release keeps its `config/`, `userdata/` and `server/data/`.
pub async fn install_release(release: &GithubReleaseResponse, dest: &Path) -> Result<(), String> {
    let asset = find_client_asset(release).ok_or_else(|| {
        format!(
            "No TES3MP-NG Windows package found in release {}",
            release.tag_name
        )
    })?;
    info!(
        "Installing TES3MP-NG {} ({}) to {}",
        release.tag_name,
        asset.name,
        dest.display()
    );

    let client = Client::new();
    let bytes = download_bytes(&client, &asset.browser_download_url).await?;
    verify_checksum(&client, release, &asset.name, &bytes).await?;

    let parent = dest
        .parent()
        .ok_or_else(|| format!("Invalid install path: {}", dest.display()))?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let staging = parent.join(format!(
        ".{}-download",
        dest.file_name().and_then(|n| n.to_str()).unwrap_or("tes3mp")
    ));
    if staging.exists() {
        std::fs::remove_dir_all(&staging).map_err(|e| e.to_string())?;
    }

    let result = (|| {
        extract_zip_stripping_root(&bytes, &staging)?;
        install_release_files(&staging, dest)
    })();
    let _ = std::fs::remove_dir_all(&staging);
    result?;

    write_installed_release(
        dest,
        &InstalledRelease {
            tag: release.tag_name.clone(),
            id: release.id,
        },
    )?;
    info!("Installed TES3MP-NG {} to {}", release.tag_name, dest.display());
    Ok(())
}

/// The TES3MP-NG release installed in a folder, recorded by `install_release`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledRelease {
    pub tag: String,
    pub id: u64,
}

const RELEASE_MARKER: &str = ".nerevar-release.json";

pub fn read_installed_release(tes3mp_dir: &Path) -> Option<InstalledRelease> {
    let contents = std::fs::read_to_string(tes3mp_dir.join(RELEASE_MARKER)).ok()?;
    serde_json::from_str(&contents).ok()
}

fn write_installed_release(tes3mp_dir: &Path, release: &InstalledRelease) -> Result<(), String> {
    let json = serde_json::to_string_pretty(release).map_err(|e| e.to_string())?;
    std::fs::write(tes3mp_dir.join(RELEASE_MARKER), json)
        .map_err(|e| format!("Failed to record the installed TES3MP-NG release: {e}"))
}

async fn download_bytes(client: &Client, url: &str) -> Result<Vec<u8>, String> {
    let response = client
        .get(url)
        .header("User-Agent", user_agent())
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Download of {url} failed: HTTP {}", response.status()));
    }
    Ok(response.bytes().await.map_err(|e| e.to_string())?.to_vec())
}

async fn verify_checksum(
    client: &Client,
    release: &GithubReleaseResponse,
    asset_name: &str,
    bytes: &[u8],
) -> Result<(), String> {
    let Some(sums_asset) = release.assets.iter().find(|a| a.name == CHECKSUMS_ASSET) else {
        info!("Release {} has no {CHECKSUMS_ASSET}; skipping verification", release.tag_name);
        return Ok(());
    };
    let sums = download_bytes(client, &sums_asset.browser_download_url).await?;
    let sums = String::from_utf8_lossy(&sums);
    let expected = expected_sha256(&sums, asset_name).ok_or_else(|| {
        format!("{CHECKSUMS_ASSET} of release {} doesn't list {asset_name}", release.tag_name)
    })?;
    let actual = format!("{:x}", Sha256::digest(bytes));
    if !actual.eq_ignore_ascii_case(&expected) {
        return Err(format!(
            "Downloaded {asset_name} doesn't match its SHA-256 checksum (expected {expected}, got {actual})"
        ));
    }
    Ok(())
}

/// Find a file's hash in `sha256sum` format: `<hash>  <file name>`.
fn expected_sha256(sums: &str, file_name: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let (hash, name) = line.trim().split_once(char::is_whitespace)?;
        (name.trim().trim_start_matches('*') == file_name).then(|| hash.to_string())
    })
}

/// Extract a zip into `dest`. When every entry sits under one top-level folder
/// (`TES3MP-NG-0.51.0-win64/...`), that folder is dropped.
fn extract_zip_stripping_root(bytes: &[u8], dest: &Path) -> Result<(), String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let mut paths = Vec::with_capacity(archive.len());
    for i in 0..archive.len() {
        let file = archive.by_index(i).map_err(|e| e.to_string())?;
        paths.push(file.enclosed_name());
    }
    let root = common_root(paths.iter().flatten());

    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for (i, enclosed) in paths.into_iter().enumerate() {
        let Some(enclosed) = enclosed else {
            continue;
        };
        let relative = match &root {
            Some(root) => match enclosed.strip_prefix(root) {
                Ok(rest) if rest.as_os_str().is_empty() => continue,
                Ok(rest) => rest.to_path_buf(),
                Err(_) => enclosed.clone(),
            },
            None => enclosed.clone(),
        };
        let outpath = dest.join(relative);

        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        if file.is_dir() {
            std::fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut outfile = File::create(&outpath).map_err(|e| e.to_string())?;
            copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn common_root<'a>(paths: impl Iterator<Item = &'a PathBuf>) -> Option<PathBuf> {
    let mut root: Option<PathBuf> = None;
    let mut has_nested = false;
    for path in paths {
        let mut components = path.components();
        let first = match components.next() {
            Some(Component::Normal(first)) => PathBuf::from(first),
            _ => return None,
        };
        if components.next().is_some() {
            has_nested = true;
        }
        match &root {
            None => root = Some(first),
            Some(existing) if *existing == first => {}
            Some(_) => return None,
        }
    }
    root.filter(|_| has_nested)
}

/// Move a freshly extracted release from `staging` into `dest`, keeping what the player and the
/// server own.
fn install_release_files(staging: &Path, dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    let entries = std::fs::read_dir(staging).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        let target = dest.join(&name);

        if PRESERVED_ON_UPDATE.iter().any(|kept| name_str.eq_ignore_ascii_case(kept)) && target.exists() {
            continue;
        }

        if name_str.eq_ignore_ascii_case(SERVER_DIR) && target.join(SERVER_DATA_DIR).is_dir() {
            replace_server_dir(&entry.path(), &target)?;
            continue;
        }

        remove_path(&target)?;
        std::fs::rename(entry.path(), &target).map_err(|e| {
            format!("Failed to move {} into {}: {e}", entry.path().display(), dest.display())
        })?;
    }
    Ok(())
}

/// Replace the server's scripts with the release's, keeping the existing `server/data/` (players,
/// cells, world state).
fn replace_server_dir(new_server: &Path, server: &Path) -> Result<(), String> {
    let kept_data = server.with_file_name(".server-data-kept");
    remove_path(&kept_data)?;
    std::fs::rename(server.join(SERVER_DATA_DIR), &kept_data).map_err(|e| e.to_string())?;
    remove_path(server)?;
    std::fs::rename(new_server, server).map_err(|e| e.to_string())?;
    remove_path(&server.join(SERVER_DATA_DIR))?;
    std::fs::rename(&kept_data, server.join(SERVER_DATA_DIR)).map_err(|e| e.to_string())?;
    Ok(())
}

fn remove_path(path: &Path) -> Result<(), String> {
    if path.is_dir() {
        std::fs::remove_dir_all(path).map_err(|e| format!("Failed to remove {}: {e}", path.display()))
    } else if path.exists() {
        std::fs::remove_file(path).map_err(|e| format!("Failed to remove {}: {e}", path.display()))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn zip_with(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut buffer = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            let options = zip::write::SimpleFileOptions::default();
            for (name, contents) in entries {
                writer.start_file(*name, options).unwrap();
                writer.write_all(contents.as_bytes()).unwrap();
            }
            writer.finish().unwrap();
        }
        buffer.into_inner()
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nerevar-gh-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn picks_the_client_package() {
        assert!(is_client_asset_name("TES3MP-NG-0.51.0-win64.zip"));
        assert!(!is_client_asset_name("TES3MP-NG-MasterServer-0.51.0-win64.zip"));
        assert!(!is_client_asset_name("TES3MP-NG-0.51.0-win64-symbols.zip"));
        assert!(!is_client_asset_name("SHA256SUMS.txt"));
    }

    #[test]
    fn reads_sha256sums() {
        let sums = "79911bbe  TES3MP-NG-0.51.0-win64.zip\na825b39f  TES3MP-NG-MasterServer-0.51.0-win64.zip\n";
        assert_eq!(
            expected_sha256(sums, "TES3MP-NG-0.51.0-win64.zip").as_deref(),
            Some("79911bbe")
        );
        assert_eq!(expected_sha256(sums, "missing.zip"), None);
    }

    #[test]
    fn strips_the_package_folder() {
        let dir = temp_dir("strip");
        let bytes = zip_with(&[
            ("TES3MP-NG-0.51.0-win64/openmw.exe", "exe"),
            ("TES3MP-NG-0.51.0-win64/server/scripts/config.lua", "lua"),
        ]);
        extract_zip_stripping_root(&bytes, &dir).unwrap();
        assert!(dir.join("openmw.exe").is_file());
        assert!(dir.join("server/scripts/config.lua").is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn update_keeps_player_and_server_data() {
        let root = temp_dir("update");
        let dest = root.join("tes3mp");
        std::fs::create_dir_all(dest.join("config")).unwrap();
        std::fs::create_dir_all(dest.join("server/data/player")).unwrap();
        std::fs::write(dest.join("config/settings.cfg"), "player").unwrap();
        std::fs::write(dest.join("server/data/player/me.json"), "me").unwrap();
        std::fs::write(dest.join("openmw.exe"), "old").unwrap();

        let staging = root.join("staging");
        extract_zip_stripping_root(
            &zip_with(&[
                ("TES3MP-NG-0.52.0-win64/openmw.exe", "new"),
                ("TES3MP-NG-0.52.0-win64/server/scripts/serverCore.lua", "core"),
                ("TES3MP-NG-0.52.0-win64/server/data/requiredDataFiles.json", "[]"),
            ]),
            &staging,
        )
        .unwrap();
        install_release_files(&staging, &dest).unwrap();

        assert_eq!(std::fs::read_to_string(dest.join("openmw.exe")).unwrap(), "new");
        assert_eq!(std::fs::read_to_string(dest.join("config/settings.cfg")).unwrap(), "player");
        assert_eq!(std::fs::read_to_string(dest.join("server/data/player/me.json")).unwrap(), "me");
        assert!(dest.join("server/scripts/serverCore.lua").is_file());
        let _ = std::fs::remove_dir_all(&root);
    }
}
