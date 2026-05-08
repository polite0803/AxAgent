use std::path::PathBuf;

/// Returns the canonical AxAgent home directory and ensures it exists.
///
/// - macOS / Linux: `~/.axagent/`
/// - Windows:       `%USERPROFILE%\.axagent\`
/// - Mobile (iOS):  App's sandboxed container (data directory)
/// - Mobile (Android): App's sandboxed container (data directory)
///
/// Panics if the home directory cannot be determined.
pub fn axagent_home() -> PathBuf {
    #[cfg(mobile)]
    {
        dirs::data_dir()
            .or_else(|| dirs::home_dir())
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".axagent")
    }
    #[cfg(not(mobile))]
    {
        #[cfg(not(windows))]
        let home = std::env::var("HOME").expect("HOME env var not set");
        #[cfg(windows)]
        let home = std::env::var("USERPROFILE").expect("USERPROFILE env var not set");

        PathBuf::from(home).join(".axagent")
    }
}
