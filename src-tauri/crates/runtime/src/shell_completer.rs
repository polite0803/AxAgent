use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct CompletionCandidate {
    pub text: String,
    pub display_text: Option<String>,
    pub description: Option<String>,
    pub score: f32,
    pub category: CompletionCategory,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionCategory {
    Command,
    Flag,
    Option,
    Argument,
    File,
    Directory,
    Variable,
    Alias,
    Custom,
}

#[derive(Debug, Clone)]
pub struct CompletionContext {
    pub command: Option<String>,
    pub args: Vec<String>,
    pub current_word: String,
    pub cursor_position: usize,
    pub shell_type: ShellType,
    pub working_directory: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    Pwsh,
    Cmd,
    Unknown,
}

impl ShellType {
    pub fn detect() -> Self {
        #[cfg(windows)]
        {
            if std::env::var("PSModulePath").is_ok() {
                return ShellType::Pwsh;
            }
            if std::env::var("CMDER_ROOT").is_ok() {
                return ShellType::Cmd;
            }
        }

        if let Ok(shell) = std::env::var("SHELL") {
            if shell.contains("zsh") {
                return ShellType::Zsh;
            }
            if shell.contains("fish") {
                return ShellType::Fish;
            }
            if shell.contains("bash") {
                return ShellType::Bash;
            }
        }

        ShellType::Bash
    }
}

pub struct ShellCompleter {
    history: Arc<RwLock<Vec<HistoryEntry>>>,
    aliases: Arc<RwLock<HashMap<String, String>>>,
    custom_providers: Arc<RwLock<Vec<Box<dyn CompletionProvider + Send + Sync>>>>,
    config: CompleterConfig,
}

#[derive(Debug, Clone)]
struct HistoryEntry {
    command: String,
    timestamp: i64,
    exit_code: Option<i32>,
    count: usize,
}

#[derive(Debug, Clone)]
pub struct CompleterConfig {
    pub max_history_entries: usize,
    pub cache_ttl_ms: u64,
    pub max_candidates: usize,
    pub enable_learning: bool,
    pub history_weight: f32,
    pub frequency_weight: f32,
    pub recency_weight: f32,
}

impl Default for CompleterConfig {
    fn default() -> Self {
        Self {
            max_history_entries: 10000,
            cache_ttl_ms: 5000,
            max_candidates: 20,
            enable_learning: true,
            history_weight: 0.4,
            frequency_weight: 0.3,
            recency_weight: 0.3,
        }
    }
}

pub trait CompletionProvider: Send + Sync {
    fn name(&self) -> &str;
    fn provide(&self, ctx: &CompletionContext) -> Vec<CompletionCandidate>;
}

impl ShellCompleter {
    pub fn new() -> Self {
        Self {
            history: Arc::new(RwLock::new(Vec::new())),
            aliases: Arc::new(RwLock::new(HashMap::new())),
            custom_providers: Arc::new(RwLock::new(Vec::new())),
            config: CompleterConfig::default(),
        }
    }

    pub fn with_config(config: CompleterConfig) -> Self {
        Self {
            history: Arc::new(RwLock::new(Vec::new())),
            aliases: Arc::new(RwLock::new(HashMap::new())),
            custom_providers: Arc::new(RwLock::new(Vec::new())),
            config,
        }
    }

    pub async fn add_provider(&self, provider: Box<dyn CompletionProvider + Send + Sync>) {
        let mut providers = self.custom_providers.write().await;
        providers.push(provider);
    }

    pub async fn learn_command(&self, command: &str, exit_code: Option<i32>) {
        if !self.config.enable_learning {
            return;
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let mut history = self.history.write().await;

        if let Some(entry) = history.iter_mut().find(|e| e.command == command) {
            entry.count += 1;
            entry.timestamp = timestamp;
            entry.exit_code = exit_code;
        } else {
            history.push(HistoryEntry {
                command: command.to_string(),
                timestamp,
                exit_code,
                count: 1,
            });

            if history.len() > self.config.max_history_entries {
                history.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                history.truncate(self.config.max_history_entries);
            }
        }
    }

    pub async fn get_completions(&self, ctx: &CompletionContext) -> Vec<CompletionCandidate> {
        let candidates = self.provide_completions(ctx).await;

        let mut sorted = candidates;
        sorted.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        sorted
            .into_iter()
            .take(self.config.max_candidates)
            .collect()
    }

    async fn provide_completions(&self, ctx: &CompletionContext) -> Vec<CompletionCandidate> {
        let mut candidates = Vec::new();

        if ctx.command.is_none() {
            candidates.extend(self.get_command_completions(ctx));
        } else {
            candidates.extend(self.get_argument_completions(ctx));
        }

        candidates.extend(self.get_history_completions(ctx).await);
        candidates.extend(self.get_alias_completions().await);

        let providers = self.custom_providers.read().await;
        for provider in providers.iter() {
            candidates.extend(provider.provide(ctx));
        }

        candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        candidates
    }

    fn get_command_completions(&self, ctx: &CompletionContext) -> Vec<CompletionCandidate> {
        let prefix = ctx.current_word.to_lowercase();

        let common_commands = vec![
            ("git", "Distributed version control", "🔀"),
            ("ls", "List directory contents", "📁"),
            ("cd", "Change directory", "📂"),
            ("pwd", "Print working directory", "📍"),
            ("mkdir", "Create directory", "📂"),
            ("rm", "Remove files/directories", "🗑️"),
            ("cp", "Copy files/directories", "📋"),
            ("mv", "Move files/directories", "📎"),
            ("cat", "Concatenate and display", "📄"),
            ("grep", "Search text patterns", "🔍"),
            ("find", "Find files", "🔎"),
            ("chmod", "Change permissions", "🔐"),
            ("chown", "Change ownership", "👤"),
            ("ps", "Process status", "⚙️"),
            ("top", "Task manager", "📊"),
            ("docker", "Container platform", "🐳"),
            ("npm", "Node package manager", "📦"),
            ("cargo", "Rust package manager", "🔧"),
            ("python", "Python interpreter", "🐍"),
            ("node", "Node.js runtime", "🟢"),
        ];

        common_commands
            .into_iter()
            .filter(|(cmd, _, _)| cmd.starts_with(&prefix) || prefix.is_empty())
            .map(|(cmd, desc, icon)| CompletionCandidate {
                text: cmd.to_string(),
                display_text: Some(format!("{} - {}", cmd, desc)),
                description: Some(desc.to_string()),
                score: 1.0,
                category: CompletionCategory::Command,
                icon: Some(icon.to_string()),
            })
            .collect()
    }

    fn get_argument_completions(&self, ctx: &CompletionContext) -> Vec<CompletionCandidate> {
        let mut candidates = Vec::new();

        if let Some(cmd) = &ctx.command {
            let flag_candidates = self.get_command_flags(cmd);
            candidates.extend(flag_candidates);
        }

        candidates
    }

    fn get_command_flags(&self, command: &str) -> Vec<CompletionCandidate> {
        let flags = match command {
            "git" => vec![
                ("status", "Show working tree status"),
                ("add", "Stage changes"),
                ("commit", "Record changes"),
                ("push", "Push to remote"),
                ("pull", "Pull from remote"),
                ("branch", "List branches"),
                ("checkout", "Switch branches"),
                ("merge", "Merge branches"),
                ("rebase", "Rebase onto another branch"),
                ("log", "Show commit logs"),
                ("diff", "Show changes"),
                ("stash", "Stash changes"),
            ],
            "ls" => vec![
                ("-l", "Long format"),
                ("-a", "Show hidden"),
                ("-h", "Human readable"),
                ("-R", "Recursive"),
                ("-t", "Sort by time"),
            ],
            "docker" => vec![
                ("ps", "List containers"),
                ("images", "List images"),
                ("run", "Run container"),
                ("stop", "Stop container"),
                ("rm", "Remove container"),
                ("exec", "Execute command"),
                ("logs", "View logs"),
            ],
            _ => vec![],
        };

        flags
            .into_iter()
            .map(|(flag, desc)| CompletionCandidate {
                text: flag.to_string(),
                display_text: None,
                description: Some(desc.to_string()),
                score: 0.8,
                category: CompletionCategory::Flag,
                icon: None,
            })
            .collect()
    }

    async fn get_history_completions(&self, ctx: &CompletionContext) -> Vec<CompletionCandidate> {
        let history = self.history.read().await;
        let prefix = ctx.current_word.to_lowercase();

        let mut scores: HashMap<String, f32> = HashMap::new();

        for entry in history.iter().rev() {
            if entry.command.to_lowercase().starts_with(&prefix) {
                let recency_score = self.calculate_recency_score(entry.timestamp);
                let score = scores.entry(entry.command.clone()).or_insert(0.0);
                *score += recency_score * self.config.recency_weight
                    + entry.count as f32 * self.config.frequency_weight;
            }
        }

        scores
            .into_iter()
            .map(|(command, score)| CompletionCandidate {
                text: command.clone(),
                display_text: Some(command.clone()),
                description: Some("From history".to_string()),
                score,
                category: CompletionCategory::Command,
                icon: Some("📜".to_string()),
            })
            .collect()
    }

    async fn get_alias_completions(&self) -> Vec<CompletionCandidate> {
        let aliases = self.aliases.read().await;
        let prefix = self.current_word_prefix();

        aliases
            .iter()
            .filter(|(alias, _)| alias.to_lowercase().starts_with(&prefix))
            .map(|(alias, expansion)| CompletionCandidate {
                text: alias.clone(),
                display_text: Some(format!("{} -> {}", alias, expansion)),
                description: Some(format!("Alias: {}", expansion)),
                score: 0.9,
                category: CompletionCategory::Alias,
                icon: Some("🔗".to_string()),
            })
            .collect()
    }

    fn current_word_prefix(&self) -> String {
        String::new()
    }

    fn calculate_recency_score(&self, timestamp: i64) -> f32 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let age_hours = (now - timestamp) / 3600;
        let decay = (-age_hours as f32 / 720.0).exp();
        decay.max(0.1)
    }

    pub async fn load_history_from_file(&self, path: &PathBuf) -> std::io::Result<()> {
        let content = std::fs::read_to_string(path)?;
        let lines: Vec<&str> = content.lines().collect();

        let mut history = self.history.write().await;

        for line in lines {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                history.push(HistoryEntry {
                    command: trimmed.to_string(),
                    timestamp: 0,
                    exit_code: None,
                    count: 1,
                });
            }
        }

        Ok(())
    }

    pub async fn add_alias(&self, alias: &str, expansion: &str) {
        let mut aliases = self.aliases.write().await;
        aliases.insert(alias.to_string(), expansion.to_string());
    }
}

impl Default for ShellCompleter {
    fn default() -> Self {
        Self::new()
    }
}
