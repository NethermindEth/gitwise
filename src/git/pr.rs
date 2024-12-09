use std::process::Command;
use anyhow::{Result, anyhow};
use git2::Repository;
use crate::ai::AiEngine;

pub struct PullRequest {
    pub title: Option<String>,
    pub body: Option<String>,
    pub base: Option<String>,
}

impl PullRequest {
    pub fn new() -> Self {
        Self {
            title: None,
            body: None,
            base: None,
        }
    }

    pub fn with_title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }

    pub fn with_body(mut self, body: String) -> Self {
        self.body = Some(body);
        self
    }

    pub fn with_base(mut self, base: String) -> Self {
        self.base = Some(base);
        self
    }

    pub async fn create(&self) -> Result<()> {
        let repo = Repository::open_from_env()?;
        let ai = AiEngine::new()?;

        // Get the diff between the current branch and the base branch
        let head = repo.head()?.peel_to_commit()?;
        let base_branch = self.base.as_deref().unwrap_or("main");
        
        let base_commit = if let Ok(branch) = repo.find_branch(base_branch, git2::BranchType::Local) {
            branch.get().peel_to_commit()?
        } else if let Ok(branch) = repo.find_branch(&format!("origin/{}", base_branch), git2::BranchType::Remote) {
            branch.get().peel_to_commit()?
        } else {
            return Err(anyhow!("Base branch '{}' not found", base_branch));
        };

        let diff = repo.diff_tree_to_tree(
            Some(&base_commit.tree()?),
            Some(&head.tree()?),
            None,
        )?;

        // Generate PR title and description using AI if not provided
        let title = match &self.title {
            Some(t) => t.clone(),
            None => {
                let commit_msg = ai.generate_commit_message(&diff).await?;
                // Extract first line as title
                commit_msg.lines().next()
                    .ok_or_else(|| anyhow!("Failed to generate PR title"))?
                    .to_string()
            }
        };

        let body = match &self.body {
            Some(b) => b.clone(),
            None => {
                ai.summarize_diff(&diff, Some("Generate a detailed pull request description that explains the changes, their purpose, and any important implementation details. Include a high-level summary at the start.")).await?
            }
        };

        let mut command = Command::new("gh");
        command.arg("pr").arg("create");
        
        command.arg("--title").arg(&title);
        command.arg("--body").arg(&body);
        
        if let Some(base) = &self.base {
            command.arg("--base").arg(base);
        }

        let output = command.output()?;
        
        if !output.status.success() {
            return Err(anyhow!(
                "Failed to create PR: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(())
    }
}
