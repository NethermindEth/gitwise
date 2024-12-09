use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand, ValueEnum};
use git2::{Repository, Oid};
use tracing::info;
use tracing_subscriber::fmt;

mod ai;
mod utils;
mod git;

use git::staging;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long, help = "Enable verbose logging")]
    verbose: bool,

    /// Force a specific AI model provider
    #[arg(long, value_enum, help = "Force a specific AI model provider (e.g., 'anthropic' or 'openai')")]
    model: Option<ModelProvider>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Intelligently stage changes by feature
    Add {
        /// Custom prompt for feature analysis
        #[arg(long, help = "Custom prompt for feature analysis (e.g., 'Focus on UI changes' or 'Look for security-related changes')")]
        prompt: Option<String>,
    },
    /// Create a pull request with AI-generated title and description
    Pr {
        /// Base branch for the PR
        #[arg(long, help = "Base branch for the PR (e.g., 'main' or 'develop')")]
        base: Option<String>,
        /// Custom PR title
        #[arg(long, help = "Custom PR title (if not provided, will be AI-generated)")]
        title: Option<String>,
        /// Custom PR description
        #[arg(long, help = "Custom PR description (if not provided, will be AI-generated)")]
        body: Option<String>,
    },
    /// Summarize changes between git references
    Diff {
        /// First git reference (branch, commit, or tag)
        #[arg(default_value = "HEAD")]
        from: String,
        /// Second git reference (branch, commit, or tag)
        #[arg()]
        to: Option<String>,
        /// Show staged changes instead
        #[arg(short, long)]
        staged: bool,
        /// Custom prompt for AI summarization
        #[arg(long, help = "Custom prompt for AI summarization (e.g., 'Focus on security changes' or 'List only modified functions')")]
        prompt: Option<String>,
    },
    /// Generate a commit message for staged changes
    Commit,
    /// Summarize git history
    History {
        /// Git reference to start from (branch, commit, or tag)
        #[arg(default_value = "HEAD")]
        reference: String,
        /// Number of commits to summarize
        #[arg(short, long, default_value_t = 5)]
        count: u32,
        /// Custom prompt for AI summarization
        #[arg(long, help = "Custom prompt for AI summarization (e.g., 'Focus on API changes' or 'Summarize in bullet points')")]
        prompt: Option<String>,
    },
    /// Show commit history with AI-generated summaries
    Log {
        /// Show commits from this branch
        #[arg(short, long)]
        branch: Option<String>,
        /// Limit the number of commits shown
        #[arg(short, long, default_value = "10")]
        limit: u32,
    },
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum ModelProvider {
    /// Use Anthropic's Claude model
    Anthropic,
    /// Use OpenAI's GPT model
    OpenAI,
}

/// Resolve a git reference (branch, tag, or commit hash) to a commit
fn resolve_reference(repo: &Repository, reference: &str) -> Result<Oid> {
    // Try as a direct reference first (branch or tag)
    if let Ok(reference) = repo.find_reference(reference) {
        return Ok(reference.peel_to_commit()?.id());
    }

    // Try as a revision (commit hash, HEAD~1, etc)
    if let Ok(revspec) = repo.revparse_single(reference) {
        return Ok(revspec.peel_to_commit()?.id());
    }

    // Try as a short commit hash
    let partial_hash = reference.to_string();
    if partial_hash.len() >= 4 {
        let mut found_oid = None;
        repo.odb()?.foreach(|oid| {
            if oid.to_string().starts_with(&partial_hash) {
                found_oid = Some(*oid);
                false // Stop iteration
            } else {
                true // Continue iteration
            }
        })?;
        
        if let Some(oid) = found_oid {
            return Ok(oid);
        }
    }

    Err(anyhow::anyhow!("Could not resolve git reference: {}", reference))
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    let cli = Cli::parse();

    // Initialize logging
    if cli.verbose {
        fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    } else {
        fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
    }

    let mut engine = ai::AiEngine::new()?;
    
    // Apply model provider if specified
    if let Some(provider) = cli.model {
        info!("Using enforced model provider: {:?}", provider);
        engine = engine.with_provider(match provider {
            ModelProvider::Anthropic => ai::ModelProvider::Anthropic,
            ModelProvider::OpenAI => ai::ModelProvider::OpenAI,
        });
    } else {
        info!("Using default model provider selection");
    }

    match &cli.command {
        Commands::Add { prompt } => {
            let repo = Repository::open_from_env()?;
            
            // Get staged and unstaged changes
            let staged_diff = staging::get_staged_changes(&repo)?;
            let unstaged_diff = staging::get_unstaged_changes(&repo)?;
            
            // Get current status for all files
            let (_staged_files, unstaged_files) = staging::get_change_groups(&repo)?;
            
            // Skip if no changes
            if unstaged_files.is_empty() {
                println!("No changes to stage.");
                return Ok(());
            }
            
            // Analyze changes and group them by feature
            let groups = engine.analyze_changes(&staged_diff, &unstaged_diff, prompt.as_deref()).await?;
            
            if groups.is_empty() {
                println!("No changes to stage.");
                return Ok(());
            }

            // Take the first group as our suggestion
            let selected_group = &groups[0];
            
            println!("\nStaging files for feature:");
            for file in selected_group {
                println!("  {}", file);
                staging::stage_file(&repo, file)?;
            }

            // Get fresh diff after staging
            let new_staged_diff = staging::get_staged_changes(&repo)?;
            let commit_msg = engine.generate_commit_message(&new_staged_diff).await?;
            
            println!("\nSuggested commit message:\n{}", commit_msg);
        }
        Commands::Pr { base, title, body } => {
            let mut pr = git::pr::PullRequest::new();
            
            if let Some(t) = title {
                pr = pr.with_title(t.clone());
            }
            if let Some(b) = body {
                pr = pr.with_body(b.clone());
            }
            if let Some(base_branch) = base {
                pr = pr.with_base(base_branch.clone());
            }
            
            pr.create().await?;
            println!("✨ Pull request created successfully!");
        }
        Commands::Diff { from, to, staged, prompt } => {
            let repo = Repository::open_from_env()?;
            let diff = if *staged {
                // Get diff of staged changes
                let mut opts = git2::DiffOptions::new();
                let head_tree = repo.head()?.peel_to_tree()?;
                repo.diff_tree_to_index(Some(&head_tree), None, Some(&mut opts))?
            } else {
                // Get diff between references
                let from_commit = repo.find_commit(resolve_reference(&repo, &from)?)?;
                let from_tree = from_commit.tree()?;

                let to_tree = if let Some(to) = to {
                    let to_commit = repo.find_commit(resolve_reference(&repo, &to)?)?;
                    to_commit.tree()?
                } else {
                    // If no 'to' reference is provided, use the working directory
                    repo.head()?.peel_to_tree()?
                };

                repo.diff_tree_to_tree(Some(&from_tree), Some(&to_tree), None)?
            };

            let summary = engine.summarize_diff(&diff, prompt.as_deref()).await?;
            println!("Changes Summary:\n{}", summary);
        }
        Commands::Commit => {
            let repo = Repository::open_from_env()?;
            
            // Check if there are staged changes
            let mut index = repo.index()?;
            if index.is_empty() {
                println!("No changes to commit");
                return Ok(());
            }
            
            // Get the diff of staged changes
            let mut opts = git2::DiffOptions::new();
            let head_tree = repo.head()?.peel_to_tree()?;
            let diff = repo.diff_tree_to_index(Some(&head_tree), None, Some(&mut opts))?;
            
            let message = engine.generate_commit_message(&diff).await?;
            
            // Create the commit
            let signature = repo.signature()?;
            let tree_id = index.write_tree()?;
            let tree = repo.find_tree(tree_id)?;
            let parent = repo.head()?.peel_to_commit()?;
            
            repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                &message,
                &tree,
                &[&parent],
            )?;
            
            println!("Created commit with message:\n{}", message);
        }
        Commands::History { reference, count, prompt } => {
            let repo = Repository::open_from_env()?;
            let branch = if reference == "HEAD" {
                None
            } else {
                Some(reference.as_str())
            };
            
            let commits = git::get_log(&repo, branch, Some(*count))?;
            
            let mut revwalk = repo.revwalk()?;
            revwalk.push(repo.head()?.target().ok_or_else(|| anyhow!("Invalid HEAD reference"))?)?;
            revwalk.set_sorting(git2::Sort::TIME)?;

            let mut summaries = Vec::new();
            for (i, oid) in revwalk.take(*count as usize).enumerate() {
                let oid = oid?;
                let commit = repo.find_commit(oid)?;
                let tree = commit.tree()?;
                
                let parent = commit.parent(0).ok();
                let parent_tree = parent.as_ref().map(|c| c.tree()).transpose()?;
                
                let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;
                let summary = engine.summarize_diff(&diff, prompt.as_deref()).await?;
                
                summaries.push(format!(
                    "Commit {} - {}\n{}\n",
                    &oid.to_string()[..7],
                    commit.summary().unwrap_or("No summary"),
                    summary
                ));

                if i < *count as usize - 1 {
                    summaries.push(String::from("\n---\n\n"));
                }
            }

            println!("Git History Summary:\n");
            for summary in summaries {
                print!("{}", summary);
            }
        }
        Commands::Log { branch, limit } => {
            let repo = Repository::open_from_env()?;
            let commits = git::get_log(&repo, branch.as_deref(), Some(*limit))?;
            
            // Build the log output
            let mut output = String::new();
            
            for commit in commits {
                let hash = commit.id();
                let time = commit.time();
                let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(time.seconds(), 0)
                    .unwrap()
                    .format("%Y-%m-%d %H:%M:%S");
                
                // Commit header
                output.push_str(&format!("\n\x1b[33mcommit {}\x1b[0m\n", hash));
                output.push_str(&format!("Author: {}\n", commit.author()));
                output.push_str(&format!("Date:   {}\n\n", datetime));
                
                // AI Summary
                let diff = git::get_commit_diff(&repo, &commit)?;
                let summary = engine.generate_commit_message(&diff).await?;
                output.push_str("\x1b[36mAI Summary:\x1b[0m\n");
                output.push_str(&format!("{}\n", summary.replace("\n", "\n    ")));
                
                // Separator
                output.push_str("\n\x1b[90m----------------------------------------\x1b[0m\n");
                
                // Original message
                if let Some(msg) = commit.message() {
                    output.push_str("\x1b[32mOriginal Message:\x1b[0m\n");
                    output.push_str(&format!("{}\n", msg.trim().replace("\n", "\n    ")));
                }
                
                output.push_str("\n");
            }
            
            // Open in pager
            let mut child = std::process::Command::new("less")
                .arg("-R")  // Enable color codes
                .stdin(std::process::Stdio::piped())
                .spawn()?;
            
            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                stdin.write_all(output.as_bytes())?;
            }
            
            child.wait()?;
        }
    }

    Ok(())
}
