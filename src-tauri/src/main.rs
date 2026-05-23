
// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tokio::main]
async fn main() {
    tokio::spawn(async {
        if let Err(e) = nerevar_rs_lib::start_web_server().await {
            eprintln!("Failed to start web server: {e}");
        }
    });

    nerevar_rs_lib::run();
}
