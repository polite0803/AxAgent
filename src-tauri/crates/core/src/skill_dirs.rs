use std::path::PathBuf;

const SKILL_DIR_PRIORITY: &[&str] = &[
    "axagent",
    "claude",
    "trae",
    "codebuddy",
    "workbuddy",
    "agents",
];

pub fn skill_dirs() -> Vec<(&'static str, PathBuf)> {
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
}

pub fn all_skills_dirs() -> Vec<PathBuf> {
    skill_dirs().into_iter().map(|(_, dir)| dir).collect()
}
