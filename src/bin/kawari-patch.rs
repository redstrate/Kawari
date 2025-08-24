use std::cmp::Ordering;

use axum::extract::Path;
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Router, routing::get};
use kawari::config::get_config;
use kawari::patch::{Version, list_patch_files};
use kawari::{SUPPORTED_BOOT_VERSION, SUPPORTED_GAME_VERSION, get_supported_expac_versions};
use physis::patchlist::{PatchEntry, PatchList, PatchListType};
use reqwest::header::USER_AGENT;

/// Check if it's a valid patch client connecting
fn check_valid_patch_client(headers: &HeaderMap) -> bool {
    let Some(user_agent) = headers.get(USER_AGENT) else {
        return false;
    };

    // FFXIV_Patch is used by sqexPatch.dll
    user_agent == "FFXIV PATCH CLIENT" || user_agent == "FFXIV_Patch"
}

async fn verify_session(
    headers: HeaderMap,
    Path((platform, channel, game_version, sid)): Path<(String, String, String, String)>,
    body: String,
) -> impl IntoResponse {
    if !check_valid_patch_client(&headers) {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let config = get_config();
    if !config.patch.supports_platform(&platform) {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    // TODO: these are all very useful and should be documented somewhere
    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Location",
        "ffxivpatch/4e9232b/vercheck.dat".parse().unwrap(),
    );
    headers.insert(
        "X-Repository",
        "ffxivneo/win32/release/game".parse().unwrap(),
    );
    headers.insert("X-Patch-Module", "ZiPatch".parse().unwrap());
    headers.insert("X-Protocol", "http".parse().unwrap());
    headers.insert("X-Latest-Version", game_version.parse().unwrap());

    if config.enforce_validity_checks {
        tracing::info!(
            "Verifying game components for {platform} {channel} {game_version} {body}..."
        );

        let body_parts: Vec<&str> = body.split('\n').collect();

        let _hashes = body_parts[0];
        let expansion_versions = &body_parts[1..body_parts.len() - 1]; // last part is empty

        let game_version = Version(&game_version);

        let supported_expac_versions = get_supported_expac_versions();

        for expansion_version in expansion_versions {
            let expac_version_parts: Vec<&str> = expansion_version.split('\t').collect();
            let expansion_name = expac_version_parts[0]; // e.g. ex1
            let expansion_version = expac_version_parts[1];

            if Version(expansion_version) > supported_expac_versions[expansion_name] {
                tracing::warn!(
                    "{expansion_name} {expansion_version} is above supported version {}!",
                    supported_expac_versions[expansion_name]
                );
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        }

        // Their version is too new
        if game_version > SUPPORTED_GAME_VERSION {
            tracing::warn!(
                "{game_version} is above supported game version {SUPPORTED_GAME_VERSION}!"
            );
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }

        if game_version < SUPPORTED_GAME_VERSION {
            tracing::warn!(
                "{game_version} is below supported game version {SUPPORTED_GAME_VERSION}!"
            );
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }

        // If we are up to date, yay!
        if game_version == SUPPORTED_GAME_VERSION {
            let mut headers = HeaderMap::new();
            headers.insert("X-Patch-Unique-Id", sid.parse().unwrap());

            return (headers).into_response();
        }
    }

    let mut headers = HeaderMap::new();
    headers.insert("X-Patch-Unique-Id", sid.parse().unwrap());

    (headers).into_response()
}

async fn verify_boot(
    headers: HeaderMap,
    Path((platform, channel, boot_version)): Path<(String, String, String)>,
) -> impl IntoResponse {
    if !check_valid_patch_client(&headers) {
        tracing::warn!("Invalid patch client! {headers:#?}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let config = get_config();
    if !config.patch.supports_platform(&platform) {
        tracing::warn!("Invalid platform! {platform}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    // TODO: these are all very useful and should be documented somewhere
    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Location",
        "ffxivpatch/2b5cbc63/vercheck.dat".parse().unwrap(),
    );
    headers.insert(
        "X-Repository",
        "ffxivneo/win32/release/boot".parse().unwrap(),
    );
    headers.insert("X-Patch-Module", "ZiPatch".parse().unwrap());
    headers.insert("X-Protocol", "http".parse().unwrap());
    headers.insert("X-Latest-Version", boot_version.parse().unwrap());

    if config.enforce_validity_checks {
        tracing::info!("Verifying boot components for {platform} {channel} {boot_version}...");

        let actual_boot_version = boot_version.split("?time").collect::<Vec<&str>>()[0];
        let boot_version = Version(actual_boot_version);

        // If we are up to date, yay!
        if boot_version == SUPPORTED_BOOT_VERSION {
            return (headers).into_response();
        }

        // Their version is too new
        if boot_version > SUPPORTED_BOOT_VERSION {
            tracing::warn!(
                "{boot_version} is above supported boot version {SUPPORTED_BOOT_VERSION}!"
            );
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }

        // check if we need any patching
        let mut send_patches = Vec::new();
        let patches = list_patch_files(&format!("{}/boot", &config.patch.patches_location));
        let mut patch_length = 0;
        for patch in patches {
            let patch_str: &str = &patch;
            if actual_boot_version.partial_cmp(patch_str).unwrap() == Ordering::Less {
                let file = std::fs::File::open(&*format!(
                    "{}/boot/{}.patch",
                    &config.patch.patches_location, patch_str
                ))
                .unwrap();
                let metadata = file.metadata().unwrap();

                send_patches.push(PatchEntry {
                    url: format!("http://{}/boot/{}.patch", config.patch.patch_dl_url, patch)
                        .to_string(),
                    version: patch_str.to_string(),
                    hash_block_size: 0,
                    length: metadata.len() as i64,
                    size_on_disk: metadata.len() as i64, // NOTE: wrong but it should be fine to lie
                    hashes: vec![],
                    unknown_a: 19,
                    unknown_b: 18,
                });
                patch_length += metadata.len();
            }
        }

        if !send_patches.is_empty() {
            headers.insert(
                "Content-Type",
                "multipart/mixed; boundary=477D80B1_38BC_41d4_8B48_5273ADB89CAC"
                    .parse()
                    .unwrap(),
            );

            let patch_list = PatchList {
                id: "477D80B1_38BC_41d4_8B48_5273ADB89CAC".to_string(),
                requested_version: boot_version.to_string().clone(),
                content_location: format!("ffxivpatch/2b5cbc63/metainfo/{}.http", boot_version.0), // FIXME: i think this is actually supposed to be the target version
                patch_length,
                patches: send_patches,
            };
            let patch_list_str = patch_list.to_string(PatchListType::Boot);
            dbg!(&patch_list_str);
            return (headers, patch_list_str).into_response();
        }
    }

    (headers).into_response()
}

async fn fallback(uri: Uri) -> (StatusCode, String) {
    tracing::warn!("{}", uri);
    (StatusCode::NOT_FOUND, format!("No route for {uri}"))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route(
            "/http/{platform}/{channel}/{game_version}/{sid}",
            post(verify_session),
        )
        .route(
            "/http/{platform}/{channel}/{boot_version}/",
            get(verify_boot),
        )
        .fallback(fallback);

    let config = get_config();

    let addr = config.patch.get_socketaddr();
    tracing::info!("Server started on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
