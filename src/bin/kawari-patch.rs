use std::cmp::Ordering;
use std::fs::read_dir;
use std::net::SocketAddr;

use axum::{Form, Json, Router, routing::get};
use axum::extract::Query;
use axum::response::Html;
use axum::routing::post;
use serde::{Deserialize, Serialize};
use kawari::config::{Config, get_config};
use axum::extract::Path;
use axum::response::IntoResponse;
use axum::http::{HeaderMap, StatusCode};
use minijinja::filters::list;

fn list_patch_files(dir_path: &str) -> Vec<String> {
    let mut entries: Vec<_> = read_dir(dir_path).unwrap().flatten().collect();
    entries.sort_by_key(|dir| dir.path());
    let mut game_patches: Vec<_> = entries
        .into_iter()
        .flat_map(|entry| {
            let Ok(meta) = entry.metadata() else {
                return vec![];
            };
            if meta.is_dir() {
                return vec![];
            }
            if meta.is_file() && entry.file_name().to_str().unwrap().contains(".patch") {
                return vec![entry.path()];
            }
            vec![]
        })
        .collect();
    game_patches.sort_by(|a, b| {
        // Ignore H/D in front of filenames
        let mut a_path = a.as_path().file_name().unwrap().to_str().unwrap().to_string();
        if a_path.starts_with("H") {
            return Ordering::Less;
        }
        let mut b_path = b.as_path().file_name().unwrap().to_str().unwrap().to_string();
        /*if b_path.starts_with("H") {
            return Ordering::Greater;
        }*/
        a_path.partial_cmp(&b_path).unwrap()
    }); // ensure we're actually installing them in the correct order
    game_patches.iter().map(|x| x.file_stem().unwrap().to_str().unwrap().to_string() ).collect()
}

async fn verify_session(Path((platform, game_version, sid)): Path<(String, String, String)>) -> impl IntoResponse {
    let config = get_config();
    if !config.supports_platform(&platform) {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let mut headers = HeaderMap::new();
    headers.insert("X-Patch-Unique-Id", sid.parse().unwrap());

    (headers).into_response()
}


async fn verify_boot(Path((platform, boot_version)): Path<(String, String)>) -> impl IntoResponse {
    tracing::info!("Verifying boot components...");

    let config = get_config();
    if !config.supports_platform(&platform) {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    // Turns 2019.03.12.0000.0001/?time=2024-06-29-18-30 into just 2019.03.12.0000.0001
    let actual_boot_version = boot_version.split("?time").collect::<Vec<&str>>()[0];

    // check if we need any patching
    let patches = list_patch_files(&config.boot_patches_location);
    for patch in patches {
        let patch_str: &str = &patch;
        if actual_boot_version.partial_cmp(patch_str).unwrap() == Ordering::Less {
            // not up to date!
            // TODO: serve patchlist
        }
    }

    let mut headers = HeaderMap::new();
    (headers).into_response()
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/http/:platform/ffxivneo_release_game/:game_version/:sid", post(verify_session))
        .route("/http/:platform/ffxivneo_release_boot/*boot_version", get(verify_boot)); // NOTE: for future programmers, this is a wildcard because axum hates the /version/?time=blah format.

    let addr = SocketAddr::from(([127, 0, 0, 1], 6900));
    tracing::info!("Patch server started on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}