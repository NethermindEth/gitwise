use anyhow::Result;
use git2::Diff;

pub fn summarize_diff(diff: &Diff) -> Result<String> {
    let stats = diff.stats()?;
    Ok(format!(
        "Changes: {} files changed, {} insertions(+), {} deletions(-)",
        stats.files_changed(),
        stats.insertions(),
        stats.deletions()
    ))
}
