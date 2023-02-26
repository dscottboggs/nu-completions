//! Fetch the default patch set

use std::{
    io::Write,
    process::{Command, Stdio},
};

use anyhow::{anyhow, Result};
use reqwest::get;
use serde::{Deserialize, Serialize};

use crate::config::Config;

static API_REPO_URL: &str = "https://api.github.com/repos/dscottboggs/nu-completions";

#[derive(Debug, Serialize, Deserialize)]
struct AssetResponse {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReleaseResponse {
    url: String,
    tag_name: String,
    assets: Vec<AssetResponse>,
}

pub(crate) async fn fetch_latest_patch_set() -> Result<()> {
    let latest_release: ReleaseResponse = get(format!("{API_REPO_URL}/releases/latest"))
        .await?
        .error_for_status()?
        .json()
        .await?;
    for asset in &latest_release.assets {
        if asset.name == "patches.tar.gz" {
            let mut asset_response = get(&asset.browser_download_url).await?.error_for_status()?;
            let mut p = Command::new("tar")
                .arg("xz")
                .current_dir(
                    Config::patch_dir()
                        .parent()
                        .expect("patch dir to have a parent"),
                )
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;
            let mut stdin = p.stdin.take().expect("stdin was requested");
            while let Some(chunk) = asset_response.chunk().await? {
                stdin.write_all(&chunk)?;
            }
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
