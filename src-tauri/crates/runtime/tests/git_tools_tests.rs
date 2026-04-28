use axagent_runtime::GitContext;
use std::process::Command as StdCommand;

fn init_temp_git_repo(path: &std::path::Path) {
    StdCommand::new("git")
        .args(["init", "--initial-branch=main"])
        .current_dir(path)
        .output()
        .expect("git init failed");

    StdCommand::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(path)
        .output()
        .ok();

    StdCommand::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(path)
        .output()
        .ok();

    std::fs::write(path.join("test.txt"), "initial content\n").ok();

    StdCommand::new("git")
        .args(["add", "test.txt"])
        .current_dir(path)
        .output()
        .expect("git add failed");

    StdCommand::new("git")
        .args(["commit", "-m", "initial commit"])
        .current_dir(path)
        .output()
        .expect("git commit failed");
}

fn create_temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("failed to create temp dir")
}

#[test]
fn test_git_context_detects_repo() {
    let dir = create_temp_dir();
    init_temp_git_repo(dir.path());
    let ctx = GitContext::detect(dir.path());
    assert!(ctx.is_some());
    assert!(!ctx.unwrap().render().is_empty());
}

#[test]
fn test_git_context_non_repo() {
    let dir = create_temp_dir();
    let ctx = GitContext::detect(dir.path());
    assert!(ctx.is_none());
}

#[test]
fn test_git_context_detects_branch() {
    let dir = create_temp_dir();
    init_temp_git_repo(dir.path());

    let ctx = GitContext::detect(dir.path()).expect("should detect git repo");
    assert_eq!(ctx.branch.as_deref(), Some("main"));
}

#[test]
fn test_git_context_finds_commits() {
    let dir = create_temp_dir();
    init_temp_git_repo(dir.path());

    let ctx = GitContext::detect(dir.path()).expect("should detect git repo");
    assert!(!ctx.recent_commits.is_empty());
    assert!(ctx.recent_commits.len() <= 5);
}

#[test]
fn test_git_context_render_contains_branch() {
    let dir = create_temp_dir();
    init_temp_git_repo(dir.path());

    let ctx = GitContext::detect(dir.path()).expect("should detect git repo");
    let rendered = ctx.render();
    assert!(rendered.contains("main"));
}

#[test]
fn test_git_context_empty_dir() {
    let dir = create_temp_dir();
    let ctx = GitContext::detect(dir.path());
    assert!(ctx.is_none());
}

#[test]
fn test_git_context_has_staged_files() {
    let dir = create_temp_dir();
    init_temp_git_repo(dir.path());

    std::fs::write(dir.path().join("staged.txt"), "staged content\n").ok();
    StdCommand::new("git")
        .args(["add", "staged.txt"])
        .current_dir(dir.path())
        .output()
        .expect("git add failed");

    let ctx = GitContext::detect(dir.path()).expect("should detect git repo");
    assert!(ctx.staged_files.contains(&"staged.txt".to_string()));
}
