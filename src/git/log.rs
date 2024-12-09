use anyhow::Result;
use git2::{Repository, Commit, DiffOptions};

/// Get commits in a branch with their diffs
pub fn get_log<'a>(repo: &'a Repository, branch_name: Option<&str>, limit: Option<u32>) -> Result<Vec<Commit<'a>>> {
    let mut revwalk = repo.revwalk()?;
    
    // Start from HEAD or specified branch
    if let Some(branch) = branch_name {
        let branch_id = repo.find_branch(branch, git2::BranchType::Local)?.get().peel_to_commit()?.id();
        revwalk.push(branch_id)?;
    } else {
        revwalk.push_head()?;
    }

    // Limit number of commits if specified
    let limit = limit.unwrap_or(10);
    
    let mut commits = Vec::new();
    for (i, oid) in revwalk.enumerate() {
        if i >= limit as usize {
            break;
        }
        let commit = repo.find_commit(oid?)?;
        commits.push(commit);
    }
    
    Ok(commits)
}

/// Get the diff for a commit
pub fn get_commit_diff<'a>(repo: &'a Repository, commit: &Commit<'a>) -> Result<git2::Diff<'a>> {
    let parent = commit.parent(0).ok();
    let tree = commit.tree()?;
    let parent_tree = parent.and_then(|p| p.tree().ok());

    let mut opts = DiffOptions::new();
    opts.context_lines(3)
        .patience(true)
        .minimal(true);

    let diff = match parent_tree {
        Some(parent_tree) => repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), Some(&mut opts))?,
        None => repo.diff_tree_to_tree(None, Some(&tree), Some(&mut opts))?,
    };

    Ok(diff)
}
