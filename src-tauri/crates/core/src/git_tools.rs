use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiffSummary {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
    pub file_diffs: Vec<FileDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub status: String,
    pub insertions: usize,
    pub deletions: usize,
    pub hunks: Vec<Hunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewComment {
    pub file: String,
    pub line: u32,
    pub severity: ReviewSeverity,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReviewSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
    pub last_commit: String,
    pub last_commit_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLogEntry {
    pub hash: String,
    pub author: String,
    pub date: String,
    pub subject: String,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatusEntry {
    pub path: String,
    pub status: String,
    pub staged: bool,
}

pub struct GitTools;

impl GitTools {
    pub fn get_staged_diff(repo_path: &str) -> Result<GitDiffSummary, String> {
        let stat_output = run_git(repo_path, &["diff", "--staged", "--numstat"])?;

        let mut file_diffs = Vec::new();
        let mut total_insertions = 0usize;
        let mut total_deletions = 0usize;

        for line in stat_output.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                let ins: usize = parts[0].parse().unwrap_or(0);
                let del: usize = parts[1].parse().unwrap_or(0);
                let path = parts[2].to_string();
                total_insertions += ins;
                total_deletions += del;
                file_diffs.push(FileDiff {
                    path,
                    status: classify_change(ins, del),
                    insertions: ins,
                    deletions: del,
                    hunks: Vec::new(),
                });
            }
        }

        Ok(GitDiffSummary {
            files_changed: file_diffs.len(),
            insertions: total_insertions,
            deletions: total_deletions,
            file_diffs,
        })
    }

    pub fn get_branch_diff(
        repo_path: &str,
        base_branch: &str,
    ) -> Result<GitDiffSummary, String> {
        let stat_output = run_git(
            repo_path,
            &["diff", &format!("{}...HEAD", base_branch), "--numstat"],
        )?;

        let mut file_diffs = Vec::new();
        let mut total_insertions = 0usize;
        let mut total_deletions = 0usize;

        for line in stat_output.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                let ins: usize = parts[0].parse().unwrap_or(0);
                let del: usize = parts[1].parse().unwrap_or(0);
                let path = parts[2].to_string();
                total_insertions += ins;
                total_deletions += del;
                file_diffs.push(FileDiff {
                    path,
                    status: classify_change(ins, del),
                    insertions: ins,
                    deletions: del,
                    hunks: Vec::new(),
                });
            }
        }

        Ok(GitDiffSummary {
            files_changed: file_diffs.len(),
            insertions: total_insertions,
            deletions: total_deletions,
            file_diffs,
        })
    }

    pub fn get_branch_commits(
        repo_path: &str,
        base_branch: &str,
    ) -> Result<Vec<GitLogEntry>, String> {
        let output = run_git(
            repo_path,
            &[
                "log",
                &format!("{}..HEAD", base_branch),
                "--format=%H|%an|%ai|%s",
            ],
        )?;

        Ok(output
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(4, '|').collect();
                if parts.len() >= 4 {
                    Some(GitLogEntry {
                        hash: parts[0].to_string(),
                        author: parts[1].to_string(),
                        date: parts[2].to_string(),
                        subject: parts[3].to_string(),
                        body: None,
                    })
                } else {
                    None
                }
            })
            .collect())
    }

    pub fn commit(repo_path: &str, message: &str) -> Result<String, String> {
        let output = run_git(repo_path, &["commit", "-m", message])?;
        Ok(output)
    }

    pub fn stage_all(repo_path: &str) -> Result<String, String> {
        let output = run_git(repo_path, &["add", "-A"])?;
        Ok(output)
    }

    pub fn stage_files(repo_path: &str, files: &[&str]) -> Result<String, String> {
        let mut args = vec!["add"];
        args.extend(files);
        let output = run_git(repo_path, &args)?;
        Ok(output)
    }

    pub fn get_status(repo_path: &str) -> Result<Vec<GitStatusEntry>, String> {
        let output = run_git(repo_path, &["status", "--porcelain=v1"])?;

        Ok(output
            .lines()
            .filter_map(|line| {
                if line.len() < 4 {
                    return None;
                }
                let index_status = line.chars().next()?;
                let worktree_status = line.chars().nth(1)?;
                let path = line[3..].to_string();
                let staged = index_status != ' ' && index_status != '?';
                let status = match (index_status, worktree_status) {
                    ('?', '?') => "untracked".to_string(),
                    ('A', _) => "added".to_string(),
                    ('M', 'M') => "modified_staged_and_unstaged".to_string(),
                    ('M', _) => "modified_staged".to_string(),
                    (_, 'M') => "modified_unstaged".to_string(),
                    ('D', _) => "deleted_staged".to_string(),
                    (_, 'D') => "deleted_unstaged".to_string(),
                    ('R', _) => "renamed".to_string(),
                    ('C', _) => "copied".to_string(),
                    _ => format!("{}{}", index_status, worktree_status),
                };
                Some(GitStatusEntry {
                    path,
                    status,
                    staged,
                })
            })
            .collect())
    }

    pub fn list_branches(repo_path: &str) -> Result<Vec<BranchInfo>, String> {
        let output = run_git(
            repo_path,
            &["branch", "-a", "--format=%(refname:short)|%(HEAD)|%(upstream:short)|%(creatordate:short)"],
        )?;

        let current_branch = run_git(repo_path, &["rev-parse", "--abbrev-ref", "HEAD"])
            .ok()
            .unwrap_or_default();

        Ok(output
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.is_empty() {
                    return None;
                }
                let name = parts[0].trim().to_string();
                let is_current = name == current_branch.trim();
                let is_remote = name.starts_with("remotes/");
                let last_commit = parts
                    .get(2)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();
                let last_commit_date = parts
                    .get(3)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();
                Some(BranchInfo {
                    name,
                    is_current,
                    is_remote,
                    last_commit,
                    last_commit_date,
                })
            })
            .collect())
    }

    pub fn get_log(
        repo_path: &str,
        max_count: usize,
    ) -> Result<Vec<GitLogEntry>, String> {
        let output = run_git(
            repo_path,
            &[
                "log",
                &format!("-n{}", max_count),
                "--format=%H|%an|%ai|%s",
            ],
        )?;

        Ok(output
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(4, '|').collect();
                if parts.len() >= 4 {
                    Some(GitLogEntry {
                        hash: parts[0].to_string(),
                        author: parts[1].to_string(),
                        date: parts[2].to_string(),
                        subject: parts[3].to_string(),
                        body: None,
                    })
                } else {
                    None
                }
            })
            .collect())
    }

    pub fn create_branch(repo_path: &str, name: &str) -> Result<String, String> {
        let output = run_git(repo_path, &["checkout", "-b", name])?;
        Ok(output)
    }

    pub fn switch_branch(repo_path: &str, name: &str) -> Result<String, String> {
        let output = run_git(repo_path, &["checkout", name])?;
        Ok(output)
    }

    pub fn generate_commit_context(repo_path: &str) -> Result<String, String> {
        let diff = Self::get_staged_diff(repo_path)?;
        let status = Self::get_status(repo_path)?;

        let mut context = String::new();
        context.push_str(&format!(
            "Files changed: {}, Insertions: {}, Deletions: {}\n\n",
            diff.files_changed, diff.insertions, diff.deletions
        ));

        if !status.is_empty() {
            context.push_str("Working tree status:\n");
            for entry in &status {
                let staged_marker = if entry.staged { "[staged]" } else { "[unstaged]" };
                context.push_str(&format!("  {} {} {}\n", staged_marker, entry.status, entry.path));
            }
            context.push('\n');
        }

        if !diff.file_diffs.is_empty() {
            context.push_str("Changed files:\n");
            for fd in &diff.file_diffs {
                context.push_str(&format!(
                    "  {} (+{} -{})\n",
                    fd.path, fd.insertions, fd.deletions
                ));
            }
        }

        Ok(context)
    }

    pub fn generate_pr_context(
        repo_path: &str,
        base_branch: &str,
    ) -> Result<String, String> {
        let diff = Self::get_branch_diff(repo_path, base_branch)?;
        let commits = Self::get_branch_commits(repo_path, base_branch)?;

        let mut context = String::new();

        if !commits.is_empty() {
            context.push_str("Commits in this branch:\n");
            for c in &commits {
                context.push_str(&format!("  {} {}\n", &c.hash[..7.min(c.hash.len())], c.subject));
            }
            context.push('\n');
        }

        context.push_str(&format!(
            "Total changes: {} files, +{} -{}\n\n",
            diff.files_changed, diff.insertions, diff.deletions
        ));

        if !diff.file_diffs.is_empty() {
            context.push_str("Changed files:\n");
            for fd in &diff.file_diffs {
                context.push_str(&format!(
                    "  {} (+{} -{})\n",
                    fd.path, fd.insertions, fd.deletions
                ));
            }
        }

        Ok(context)
    }
}

fn run_git(cwd: &str, args: &[&str]) -> Result<String, String> {
    if !Path::new(cwd).exists() {
        return Err(format!("Directory does not exist: {}", cwd));
    }

    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("Failed to execute git: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git {} failed: {}", args.join(" "), stderr.trim()));
    }

    String::from_utf8(output.stdout).map_err(|e| format!("Invalid UTF-8 output: {}", e))
}

fn classify_change(insertions: usize, deletions: usize) -> String {
    match (insertions, deletions) {
        (0, 0) => "modified".to_string(),
        (_, 0) => "added".to_string(),
        (0, _) => "deleted".to_string(),
        _ => "modified".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_change() {
        assert_eq!(classify_change(10, 0), "added");
        assert_eq!(classify_change(0, 5), "deleted");
        assert_eq!(classify_change(5, 3), "modified");
        assert_eq!(classify_change(0, 0), "modified");
    }
}
