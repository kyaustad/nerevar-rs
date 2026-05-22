use crate::data::GithubReleaseResponse;
use reqwest::Client;

pub async fn get_all_releases() -> Result<Vec<GithubReleaseResponse>, String> {
    let client = Client::new();
    let url = "https://api.github.com/repos/tes3mp/tes3mp/releases";

    let response = match client.get(url).header("User-Agent", "Nerevar-0.1.0").send().await {
        Ok(res) => res,
        Err(e) => {
            eprintln!("get_all_releases request failed: {e}");
            return Err(e.to_string())
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        eprintln!("get_all_releases HTTP {status}: {body}");
        return Err(format!("Github API returnedHTTP {status}: {body}"));
    }

    match response.json::<Vec<GithubReleaseResponse>>().await {
        Ok(releases) => Ok(releases),
        Err(e) => {
            eprintln!("get_all_releases JSON parsing failed: {e}");
            return Err(e.to_string())
        }
    }
}
