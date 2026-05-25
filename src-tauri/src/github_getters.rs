use crate::data::GithubReleaseResponse;
use reqwest::Client;
use std::fs::File;
use std::io::{copy, Cursor};
use std::path::Path;
use tauri_plugin_log::log::{error, info};

pub async fn get_all_releases() -> Result<Vec<GithubReleaseResponse>, String> {
    info!("Attempting to fetch all Tes3MP releases from Github...");
    let client = Client::new();
    let url = "https://api.github.com/repos/tes3mp/tes3mp/releases";

    let response = match client
        .get(url)
        .header("User-Agent", "Nerevar-0.1.0")
        .send()
        .await
    {
        Ok(res) => res,
        Err(e) => {
            error!("get_all_releases request failed: {e}");
            return Err(e.to_string());
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        error!("get_all_releases HTTP {status}: {body}");
        return Err(format!("Github API returnedHTTP {status}: {body}"));
    }

    match response.json::<Vec<GithubReleaseResponse>>().await {
        Ok(releases) => {
            info!("Successfully fetched all Tes3MP releases from Github");
            Ok(releases)
        }
        Err(e) => {
            error!("get_all_releases JSON parsing failed: {e}");
            return Err(e.to_string());
        }
    }
}

pub async fn download_and_extract_release_zip_by_id_to_path(
    release_id: String,
    path: String,
) -> Result<(), String> {
    info!("Downloading release {release_id} to {path}");

    let release_id_u64: u64 = release_id
        .parse()
        .map_err(|_| format!("Invalid release id: {release_id}"))?;

    let all_releases = get_all_releases().await?;
    let desired_release = all_releases
        .iter()
        .find(|release| release.id == release_id_u64)
        .ok_or_else(|| format!("Release with id {release_id} not found"))?;

    let zip_asset = desired_release
        .assets
        .iter()
        .find(|asset| asset.name.contains("Win64") && asset.name.ends_with(".zip"))
        .ok_or_else(|| format!("No Windows zip asset found for release {release_id}"))?;

    let dest = Path::new(&path);
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;

    let client = Client::new();
    let bytes = client
        .get(&zip_asset.browser_download_url)
        .header("User-Agent", "Nerevar-0.1.0")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;

    let reader = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(reader).map_err(|e| e.to_string())?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let Some(relative_path) = file.enclosed_name() else {
            continue;
        };
        let outpath = dest.join(relative_path);

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut outfile = File::create(&outpath).map_err(|e| e.to_string())?;
            copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
        }
    }

    info!("Extracted release {release_id} to {}", dest.display());
    Ok(())
}
