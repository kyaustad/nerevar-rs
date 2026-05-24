use crate::data::GithubReleaseResponse;
use reqwest::Client;
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
