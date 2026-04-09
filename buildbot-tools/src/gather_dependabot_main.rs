use anyhow::{Context, Result};
use clap::Parser;
use reqwest::Client;
use serde::Deserialize;
use std::process::Command;

#[derive(Parser)]
#[command(name = "gather-dependabot")]
#[command(about = "Gather dependabot PRs into a single PR", long_about = None)]
struct Cli {
    /// GitHub repository to process (owner/repo)
    #[arg(short, long, default_value = "buildbot/buildbot")]
    repo: String,
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
struct GitHubPR {
    number: u64,
    title: String,
    user: GitHubUser,
}

#[derive(Deserialize)]
struct GitHubUser {
    login: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let token = get_github_token()?;
    let client = Client::new();

    git_fetch(&cli.repo, "master")?;
    git_checkout("FETCH_HEAD", "gather_dependabot")?;

    let prs = fetch_prs(&client, &cli.repo, &token).await?;

    let mut pr_text = String::from("This PR collects dependabot PRs:\n\n");
    let commit_before = get_current_commit()?;

    for pr in prs {
        if !pr.user.login.contains("dependabot") {
            continue;
        }

        print!("{} {}\n", pr.number, pr.title);

        if let Err(e) = process_pr(&cli.repo, pr.number) {
            eprintln!("GOT ERROR, skipping PR: {}", e);
            git_reset_hard(&commit_before)?;
        } else {
            pr_text.push_str(&format!("#{}: {}\n", pr.number, pr.title));
        }
    }

    println!("===========");
    println!("{}", pr_text);

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

fn get_current_commit() -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .context("git rev-parse failed")?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn git_fetch(repo: &str, branch: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["fetch", &format!("https://github.com/{}", repo), branch])
        .output()
        .context("git fetch failed")?;
    if !output.status.success() {
        anyhow::bail!("git fetch failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}

fn git_checkout(commit: &str, branch: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["checkout", commit, "-B", branch])
        .output()
        .context("git checkout failed")?;
    if !output.status.success() {
        anyhow::bail!("git checkout failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}

fn git_reset_hard(commit: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["reset", "--hard", commit])
        .output()
        .context("git reset --hard failed")?;
    if !output.status.success() {
        anyhow::bail!("git reset --hard failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}

fn git_cherry_pick(commit: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["cherry-pick", commit])
        .output()
        .context("git cherry-pick failed")?;
    if !output.status.success() {
        anyhow::bail!("git cherry-pick failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}

async fn fetch_prs(client: &Client, repo: &str, token: &str) -> Result<Vec<GitHubPR>> {
    let url = format!("https://api.github.com/repos/{}/pulls", repo);
    let response = client
        .get(&url)
        .header("Authorization", format!("token {}", token))
        .header("User-Agent", "buildbot-tools")
        .send()
        .await
        .context("Failed to fetch PRs")?;
    
    let prs: Vec<GitHubPR> = response
        .json()
        .await
        .context("Failed to parse PRs response")?;
    Ok(prs)
}

fn process_pr(repo: &str, pr_number: u64) -> Result<()> {
    git_fetch(repo, &format!("refs/pull/{}/head", pr_number))?;
    
    git_cherry_pick("master..FETCH_HEAD")?;
    
    Ok(())
}