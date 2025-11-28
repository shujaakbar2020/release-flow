mod git;
mod version;
mod github;

use anyhow::{Context, Result};
use clap::Parser;
use dotenv::dotenv;
use log::info;
use semver::Version;
use std::env;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the git repository
    #[arg(short, long, default_value = ".")]
    path: String,

    /// GitHub Token (required for creating releases)
    #[arg(long, env = "GITHUB_TOKEN")]
    token: Option<String>,

    /// Dry run mode (calculate version but don't create release)
    #[arg(long, default_value = "false")]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    let args = Args::parse();

    info!("Starting release-flow...");

    // 1. Open Git Repo
    let repo = git::GitRepo::open(&args.path)?;
    
    // 2. Get latest tag
    let (current_version, last_tag) = match repo.get_latest_tag()? {
        Some((v, t)) => (v, Some(t)),
        None => (Version::parse("0.0.0")?, None),
    };
    
    info!("Current version: {}", current_version);

    // 3. Get commits since last tag
    let commits = repo.get_commits_since(last_tag.as_deref())?;
    if commits.is_empty() {
        info!("No new commits found.");
        return Ok(());
    }
    info!("Found {} new commits.", commits.len());

    // 4. Calculate next version
    let (next_version, bump) = version::calculate_next_version(&current_version, &commits)?;
    
    if bump == version::BumpType::None {
        info!("No changes requiring a version bump.");
        return Ok(());
    }

    info!("Next version: {}", next_version);

    // 5. Create Release
    if args.dry_run {
        info!("Dry run enabled. Skipping release creation.");
        let changelog = github::generate_changelog(&commits);
        println!("--- Changelog ---\n{}", changelog);
    } else {
        let token = args.token.context("GITHUB_TOKEN is required for release creation")?;
        
        // Try to get owner/repo from env vars (GitHub Actions standard)
        let github_repo_env = env::var("GITHUB_REPOSITORY").unwrap_or_else(|_| "owner/repo".to_string());
        let parts: Vec<&str> = github_repo_env.split('/').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid GITHUB_REPOSITORY format. Expected 'owner/repo'");
        }
        let owner = parts[0].to_string();
        let repo_name = parts[1].to_string();

        info!("Creating release on {}/{}...", owner, repo_name);
        
        let client = github::GitHubClient::new(token, owner, repo_name)?;
        let tag_name = format!("v{}", next_version);
        let changelog = github::generate_changelog(&commits);
        
        let url = client.create_release(&tag_name, &changelog, false).await?;
        info!("Release created successfully: {}", url);
        
        // Output for GitHub Actions
        println!("::set-output name=version::{}", next_version);
        println!("::set-output name=tag::{}", tag_name);
    }

    Ok(())
}

