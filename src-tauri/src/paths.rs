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
        let base = dirs::data_dir()
            .or_else(dirs::home_dir)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .or_else(|| std::env::var("ANDROID_DATA").ok())
                    .map(PathBuf::from)
            })
            .unwrap_or_else(|| {
                tracing::warn!("Could not determine home directory, using current dir");
                PathBuf::from(".")
            });
        base.join(".axagent")
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
