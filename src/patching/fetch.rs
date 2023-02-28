//! Fetch the default patch set

use std::{
    fs::create_dir_all,
    io::Write,
    process::{Command, Stdio},
};

use anyhow::{anyhow, Result};
use reqwest::{get, Client};
use serde::{Deserialize, Serialize};

use crate::config::Config;

static API_REPO_URL: &str = "https://api.github.com/repos/dscottboggs/nu-completions";

/// The relevant fields from the "assets" part of the GitHub API releases
/// response.
#[derive(Debug, Serialize, Deserialize)]
struct AssetResponse {
    name: String,
    browser_download_url: String,
}

/// The relevant fields from the GitHub releases response
#[derive(Debug, Serialize, Deserialize)]
struct ReleaseResponse {
    url: String,
    tag_name: String,
    assets: Vec<AssetResponse>,
}

/// Unpack the latest patches from the tarball in the latest release on GitHub,
/// into [`Config::patch_dir()`].
pub(crate) async fn fetch_latest_patch_set() -> Result<()> {
    let client: Client = Client::new();
    let latest_release: ReleaseResponse = client
        .get(format!("{API_REPO_URL}/releases/latest"))
        .header(
            "User-Agent",
            "nu-completions script (reqwest) <scott+cargo@tams.tech>",
        )
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let target_dir = Config::patch_dir()
        .parent()
        .expect("patch dir to have a parent");
    create_dir_all(target_dir)?;
    for asset in &latest_release.assets {
        if asset.name == "patches.tar.gz" {
            let mut asset_response = get(&asset.browser_download_url).await?.error_for_status()?;
            let mut p = Command::new("tar")
                .arg("xz")
                .current_dir(target_dir)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;
            let mut stdin = p.stdin.take().expect("stdin was requested");
            while let Some(chunk) = asset_response.chunk().await? {
                stdin.write_all(&chunk)?;
            }
            drop(stdin);
            p.wait()?.exit_ok()?;
            return Ok(());
        }
    }
    Err(anyhow!(
        "asset `patches.tar.gz` not found in release {} assets: {}",
        latest_release.tag_name,
        latest_release
            .assets
            .iter()
            .map(|asset| asset.name.to_owned())
            .collect::<Vec<String>>()
            .join(", ")
    ))
}
