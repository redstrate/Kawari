use std::cmp::Ordering;
use std::fs::read_dir;

use axum::extract::Path;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Router, routing::get};
use kawari::config::get_config;
use physis::patchlist::{PatchEntry, PatchList, PatchListType};

fn list_patch_files(dir_path: &str) -> Vec<String> {
    // If the dir doesn't exist, pretend there is no patch files
    let Ok(dir) = read_dir(dir_path) else {
        return Vec::new();
    };
    let mut entries: Vec<_> = dir.flatten().collect();
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
        let a_path = a
            .as_path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        if a_path.starts_with("H") {
            return Ordering::Less;
        }
        let b_path = b
            .as_path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        /*if b_path.starts_with("H") {
            return Ordering::Greater;
        }*/
        a_path.partial_cmp(&b_path).unwrap()
    }); // ensure we're actually installing them in the correct order
    game_patches
        .iter()
        .map(|x| x.file_stem().unwrap().to_str().unwrap().to_string())
        .collect()
}

async fn verify_session(
    Path((platform, _, sid)): Path<(String, String, String)>,
) -> impl IntoResponse {
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
            let patch_list = PatchList {
                id: "477D80B1_38BC_41d4_8B48_5273ADB89CAC".to_string(),
                requested_version: boot_version.clone(),
                patch_length: todo!(),
                content_location: todo!(),
                patches: vec![PatchEntry {
                    url: format!("http://{}", patch).to_string(),
                    version: "2023.09.15.0000.0000".to_string(),
                    hash_block_size: 50000000,
                    length: 1479062470,
                    size_on_disk: 0,
                    hashes: vec![],
                    unknown_a: 0,
                    unknown_b: 0,
                }],
            };
            let patch_list_str = patch_list.to_string(PatchListType::Boot);
            return patch_list_str.into_response();
        }
    }

    let headers = HeaderMap::new();
    (headers).into_response()
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route(
            "/http/:platform/ffxivneo_release_game/:game_version/:sid",
            post(verify_session),
        )
        .route(
            "/http/:platform/ffxivneo_release_boot/*boot_version",
            get(verify_boot),
        ); // NOTE: for future programmers, this is a wildcard because axum hates the /version/?time=blah format.

    let config = get_config();

    let addr = config.patch.get_socketaddr();
    tracing::info!("Patch server started on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
