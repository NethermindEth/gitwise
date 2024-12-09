use anyhow::Result;
use git2::{Diff, Repository};

pub fn get_branch_diff<'a>(repo: &'a Repository, source: &str, target: &str) -> Result<Diff<'a>> {
    let source_branch = repo.find_branch(source, git2::BranchType::Local)?;
    let target_branch = repo.find_branch(target, git2::BranchType::Local)?;
    
    let source_tree = source_branch.get().peel_to_tree()?;
    let target_tree = target_branch.get().peel_to_tree()?;
    
    let diff = repo.diff_tree_to_tree(
        Some(&source_tree),
        Some(&target_tree),
        None,
    )?;
    
    Ok(diff)
}
