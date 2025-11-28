use anyhow::{Context, Result};
use git2::{Repository, Sort, ObjectType};
use semver::Version;
use std::path::Path;

pub struct GitRepo {
    repo: Repository,
}

impl GitRepo {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let repo = Repository::open(path).context("Failed to open git repository")?;
        Ok(Self { repo })
    }

    pub fn get_latest_tag(&self) -> Result<Option<(Version, String)>> {
        let tags = self.repo.tag_names(None)?;
        let mut max_version: Option<Version> = None;
        let mut max_tag_name = String::new();

        for tag_name in tags.iter().flatten() {
            // Check if tag starts with 'v' and parse
            let version_str = tag_name.strip_prefix('v').unwrap_or(tag_name);
            if let Ok(version) = Version::parse(version_str) {
                if max_version.is_none() || version > max_version.clone().unwrap() {
                    max_version = Some(version);
                    max_tag_name = tag_name.to_string();
                }
            }
        }

        Ok(max_version.map(|v| (v, max_tag_name)))
    }

    pub fn get_commits_since(&self, tag_name: Option<&str>) -> Result<Vec<String>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.set_sorting(Sort::TOPOLOGICAL)?;

        if let Some(tag) = tag_name {
            // Range: tag..HEAD
            // We need to find the commit object for the tag
            let obj = self.repo.revparse_single(tag)?;
            let commit_id = obj.peel(ObjectType::Commit)?.id();
            
            revwalk.push_head()?;
            revwalk.hide(commit_id)?;
        } else {
            revwalk.push_head()?;
        }

        let mut commits = Vec::new();
        for oid in revwalk {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            if let Some(message) = commit.message() {
                commits.push(message.to_string());
            }
        }

        Ok(commits)
    }

}
