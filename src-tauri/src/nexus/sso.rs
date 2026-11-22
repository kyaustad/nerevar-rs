//! Nexus Mods single sign-on (protocol 2): Nerevar opens a websocket to sso.nexusmods.com, the player approves
//! Nerevar on the Nexus website, and Nexus sends the account's API key over the socket.
//!
//! SSO needs an application id that Nexus Mods registers for Nerevar. It is set at build time with the
//! `NEREVAR_NEXUS_APP_SLUG` environment variable (or at run time with the same variable); without one, players
//! paste a personal API key instead.

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use tokio_tungstenite::tungstenite::Message;

const SSO_SOCKET: &str = "wss://sso.nexusmods.com";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(300);
const PING_INTERVAL: Duration = Duration::from_secs(30);

pub fn app_slug() -> Option<String> {
    std::env::var("NEREVAR_NEXUS_APP_SLUG")
        .ok()
        .or_else(|| option_env!("NEREVAR_NEXUS_APP_SLUG").map(str::to_string))
        .filter(|slug| !slug.trim().is_empty())
}

#[derive(Debug, Deserialize)]
struct SsoReply {
    success: bool,
    #[serde(default)]
    data: Option<SsoData>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SsoData {
    #[serde(default)]
    connection_token: Option<String>,
    #[serde(default)]
    api_key: Option<String>,
}

pub async fn request_api_key() -> Result<String, String> {
    let slug = app_slug().ok_or_else(|| {
        "Nexus SSO isn't available in this build of Nerevar yet; paste a personal API key instead".to_string()
    })?;
    tokio::time::timeout(LOGIN_TIMEOUT, login(&slug))
        .await
        .map_err(|_| "Nexus login timed out; try again".to_string())?
}

async fn login(slug: &str) -> Result<String, String> {
    let (mut socket, _) = tokio_tungstenite::connect_async(SSO_SOCKET)
        .await
        .map_err(|e| format!("Couldn't reach Nexus SSO: {e}"))?;

    let request_id = uuid::Uuid::new_v4().to_string();
    socket
        .send(Message::text(json!({ "id": request_id, "token": null, "protocol": 2 }).to_string()))
        .await
        .map_err(|e| format!("Nexus SSO failed: {e}"))?;

    let mut opened_browser = false;
    let mut ping = tokio::time::interval(PING_INTERVAL);
    ping.tick().await;
    loop {
        tokio::select! {
            _ = ping.tick() => {
                let _ = socket.send(Message::Ping(Vec::new().into())).await;
            }
            message = socket.next() => {
                let message = match message {
                    Some(Ok(message)) => message,
                    Some(Err(e)) => return Err(format!("Nexus SSO failed: {e}")),
                    None => return Err("Nexus SSO closed the connection".into()),
                };
                let Message::Text(text) = message else { continue };
                let reply: SsoReply = serde_json::from_str(&text)
                    .map_err(|e| format!("Unexpected Nexus SSO reply: {e}"))?;
                if !reply.success {
                    return Err(format!("Nexus SSO refused: {}", reply.error.unwrap_or_default()));
                }
                let data = reply.data.unwrap_or(SsoData { connection_token: None, api_key: None });
                if let Some(key) = data.api_key {
                    let _ = socket.close(None).await;
                    return Ok(key);
                }
                if data.connection_token.is_some() && !opened_browser {
                    opened_browser = true;
                    let url = format!("https://www.nexusmods.com/sso?id={request_id}&application={slug}");
                    opener::open_browser(&url).map_err(|e| format!("Couldn't open {url}: {e}"))?;
                }
            }
        }
    }
}
