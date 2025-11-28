use anyhow::Result;
use octocrab::Octocrab;

pub struct GitHubClient {
    octocrab: Octocrab,
    owner: String,
    repo: String,
}

impl GitHubClient {
    pub fn new(token: String, owner: String, repo: String) -> Result<Self> {
        let octocrab = Octocrab::builder().personal_token(token).build()?;
        Ok(Self { octocrab, owner, repo })
    }

    pub async fn create_release(&self, tag: &str, body: &str, prerelease: bool) -> Result<String> {
        let release = self.octocrab
            .repos(&self.owner, &self.repo)
            .releases()
            .create(tag)
            .name(tag)
            .body(body)
            .prerelease(prerelease)
            .send()
            .await?;

        Ok(release.html_url.to_string())
    }
}

pub fn generate_changelog(commits: &[String]) -> String {
    let mut changelog = String::from("## Changes\n\n");
    for msg in commits {
        // Take the first line of the commit message
        let title = msg.lines().next().unwrap_or("").trim();
        if !title.is_empty() {
            changelog.push_str(&format!("- {}\n", title));
        }
    }
    changelog
}
