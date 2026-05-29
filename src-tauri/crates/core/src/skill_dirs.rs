use std::path::PathBuf;
use std::sync::LazyLock;

const SKILL_DIR_PRIORITY: &[&str] = &[
    "axagent",
    "claude",
    "trae",
    "codebuddy",
    "workbuddy",
    "agents",
];

static SKILL_DIRS: LazyLock<Vec<(&'static str, PathBuf)>> = LazyLock::new(|| {
    let home = dirs::home_dir().unwrap_or_default();
    SKILL_DIR_PRIORITY
        .iter()
        .map(|name| {
            let dir = if *name == "axagent" {
                home.join(".axagent").join("skills")
            } else {
                home.join(format!(".{}", name)).join("skills")
            };
            (*name, dir)
        })
        .collect()
});

pub fn skill_dirs() -> Vec<(&'static str, PathBuf)> {
    SKILL_DIRS.clone()
}

pub fn all_skills_dirs() -> Vec<PathBuf> {
    SKILL_DIRS.iter().map(|(_, dir)| dir.clone()).collect()
}
