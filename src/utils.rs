use anyhow::Result;
use git2::Repository;

/// Get the current git repository
fn get_current_repo() -> Result<Repository> {
    Ok(Repository::open_from_env()?)
}
