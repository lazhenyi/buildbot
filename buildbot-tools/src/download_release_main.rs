use anyhow::{Context, Result};
use clap::Parser;
use reqwest::Client;
use serde::Deserialize;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

#[derive(Parser)]
#[command(name = "download-release")]
#[command(about = "Download buildbot release assets", long_about = None)]
struct Cli {
    /// Release name (tag)
    #[arg(short, long)]
    name: Option<String>,
}

#[derive(Deserialize)]
struct HubConfig {
    #[serde(rename = "github.com")]
    github_com: Vec<GitHubEntry>,
}

#[derive(Deserialize)]
struct GitHubEntry {
    #[serde(rename = "oauth_token")]
    oauth_token: String,
}

#[derive(Deserialize)]
struct Release {
    id: u64,
    name: String,
    #[serde(rename = "upload_url")]
    upload_url: String,
}

#[derive(Deserialize)]
struct ReleaseAsset {
    #[serde(rename = "browser_download_url")]
    browser_download_url: String,
    #[allow(dead_code)]
    name: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let token = get_github_token()?;
    let client = Client::new();

    let tag = match &cli.name {
        Some(t) => t.clone(),
        None => get_current_tag()?,
    };

    let release = find_release_by_name(&client, &tag, &token).await?;
    
    let upload_url = release.upload_url.split('{').next().unwrap().to_string();

    let assets = fetch_release_assets(&client, release.id, &token).await?;

    tokio::fs::create_dir_all("dist").await?;

    for asset in assets {
        let url = &asset.browser_download_url;
        
        if url.contains("gitarchive") {
            anyhow::bail!("The git archive has already been uploaded. Are you trying to fix broken upload? If this is the case, delete the asset in the GitHub UI and retry this command");
        }
        
        if url.ends_with(".whl") || url.ends_with(".tar.gz") {
            let filename = url.split('/').last().unwrap();
            let path = Path::new("dist").join(filename);
            download_file(&client, url, &path, &token).await?;
        }
    }

    let archive_url = format!("https://github.com/buildbot/buildbot/archive/{}.tar.gz", tag);
    let archive_path = Path::new("dist").join(format!("buildbot-{}.gitarchive.tar.gz", tag));
    download_file(&client, &archive_url, &archive_path, &token).await?;

    let sig_path_str = format!("{}.asc", archive_path.display());
    let sig_path = Path::new(&sig_path_str);
    if sig_path.exists() {
        tokio::fs::remove_file(sig_path).await?;
    }

    let gpg_output = tokio::process::Command::new("gpg")
        .args(["--armor", "--detach-sign", "--output", &sig_path_str, &archive_path.to_string_lossy()])
        .output()
        .await
        .context("gpg signing failed")?;
    
    if !gpg_output.status.success() {
        anyhow::bail!("gpg failed: {}", String::from_utf8_lossy(&gpg_output.stderr));
    }

    upload_asset(&client, &upload_url, sig_path, "application/pgp-signature", &token).await?;
    upload_asset(&client, &upload_url, &archive_path, "application/gzip", &token).await?;

    tokio::fs::remove_file(sig_path).await?;
    tokio::fs::remove_file(&archive_path).await?;

    Ok(())
}

fn get_github_token() -> Result<String> {
    let home = std::env::var("HOME").context("HOME not set")?;
    let config_path = format!("{}/.config/hub", home);
    let content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read {}", config_path))?;
    let config: HubConfig = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse {}", config_path))?;
    config.github_com.first()
        .map(|e| e.oauth_token.clone())
        .context("No GitHub token found")
}

fn get_current_tag() -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["tag", "--points-at", "HEAD"])
        .output()
        .context("git tag failed")?;
    
    let out = String::from_utf8_lossy(&output.stdout).trim().to_string();
    
    if out.is_empty() {
        anyhow::bail!("Could not find any tags pointing to current release");
    }
    
    let tags: Vec<&str> = out.split_whitespace().collect();
    if tags.len() > 1 {
        anyhow::bail!("More than one tag points to HEAD: {:?}", tags);
    }
    
    Ok(tags[0].to_string())
}

async fn find_release_by_name(client: &Client, name: &str, token: &str) -> Result<Release> {
    let url = "https://api.github.com/repos/buildbot/buildbot/releases";
    let response = client
        .get(url)
        .header("Authorization", format!("token {}", token))
        .header("User-Agent", "buildbot-tools")
        .send()
        .await
        .context("Failed to fetch releases")?;

    let releases: Vec<Release> = response
        .json()
        .await
        .context("Failed to parse releases response")?;

    releases
        .into_iter()
        .find(|r| r.name == name)
        .with_context(|| format!("Could not find release for name {}", name))
}

async fn fetch_release_assets(client: &Client, release_id: u64, token: &str) -> Result<Vec<ReleaseAsset>> {
    let url = format!("https://api.github.com/repos/buildbot/buildbot/releases/{}/assets", release_id);
    let response = client
        .get(&url)
        .header("Authorization", format!("token {}", token))
        .header("User-Agent", "buildbot-tools")
        .send()
        .await
        .context("Failed to fetch assets")?;

    let assets: Vec<ReleaseAsset> = response
        .json()
        .await
        .context("Failed to parse assets response")?;
    Ok(assets)
}

async fn download_file(client: &Client, url: &str, path: &Path, token: &str) -> Result<()> {
    if path.exists() {
        println!("Removing old file {}", path.display());
        tokio::fs::remove_file(path).await?;
    }

    println!("Downloading {} from {}", path.display(), url);

    let response = client
        .get(url)
        .header("Authorization", format!("token {}", token))
        .header("User-Agent", "buildbot-tools")
        .send()
        .await
        .context("Failed to download file")?;

    let bytes = response
        .bytes()
        .await
        .context("Failed to read response body")?;

    let mut file = File::create(path).await?;
    file.write_all(&bytes).await?;

    Ok(())
}

async fn upload_asset(
    client: &Client,
    upload_url: &str,
    path: &Path,
    content_type: &str,
    token: &str,
) -> Result<()> {
    let filename = path.file_name()
        .unwrap()
        .to_str()
        .unwrap();
    
    let body = tokio::fs::read(path).await?;
    
    let response = client
        .post(upload_url)
        .header("Authorization", format!("token {}", token))
        .header("User-Agent", "buildbot-tools")
        .header("Content-Type", content_type)
        .query(&[("name", filename)])
        .body(body)
        .send()
        .await
        .context("Failed to upload asset")?;

    println!("Upload response: {:?}", response.text().await);
    Ok(())
}