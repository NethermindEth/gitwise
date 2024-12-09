use anyhow::Result;
use git2::{Repository, Diff, Status, StatusOptions};

pub fn get_staged_changes<'a>(repo: &'a Repository) -> Result<Diff<'a>> {
    let head_tree = repo.head()?.peel_to_tree()?;
    
    let diff = repo.diff_tree_to_index(
        Some(&head_tree),
        None,
        None,
    )?;
    
    Ok(diff)
}

pub fn get_unstaged_changes<'a>(repo: &'a Repository) -> Result<Diff<'a>> {
    let mut opts = git2::DiffOptions::new();
    opts.include_untracked(true);
    
    let diff = repo.diff_index_to_workdir(
        None,
        Some(&mut opts),
    )?;
    
    Ok(diff)
}

pub fn stage_file(repo: &Repository, path: &str) -> Result<()> {
    let mut index = repo.index()?;
    index.add_path(path.as_ref())?;
    index.write()?;
    Ok(())
}

pub fn get_status(repo: &Repository) -> Result<Vec<(String, Status)>> {
    let mut status_opts = StatusOptions::new();
    status_opts
        .include_untracked(true)
        .recurse_untracked_dirs(true);
    
    let statuses = repo.statuses(Some(&mut status_opts))?;
    let mut result = Vec::new();
    
    for entry in statuses.iter() {
        if let Some(path) = entry.path() {
            result.push((path.to_string(), entry.status()));
        }
    }
    
    Ok(result)
}

/// Group changes by their status (staged/unstaged) and file path
pub fn get_change_groups(repo: &Repository) -> Result<(Vec<String>, Vec<String>)> {
    let mut staged = Vec::new();
    let mut unstaged = Vec::new();
    
    let statuses = get_status(repo)?;
    for (path, status) in statuses {
        if status.is_index_new() || status.is_index_modified() || status.is_index_deleted() {
            staged.push(path.clone());
        }
        if status.is_wt_new() || status.is_wt_modified() || status.is_wt_deleted() {
            unstaged.push(path);
        }
    }
    
    Ok((staged, unstaged))
}
