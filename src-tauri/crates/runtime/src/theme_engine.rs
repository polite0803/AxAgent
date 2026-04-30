use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    pub background: String,
    pub foreground: String,
    pub cursor: String,
    pub cursor_accent: Option<String>,
    pub selection_background: Option<String>,
    pub black: Option<String>,
    pub red: Option<String>,
    pub green: Option<String>,
    pub yellow: Option<String>,
    pub blue: Option<String>,
    pub magenta: Option<String>,
    pub cyan: Option<String>,
    pub white: Option<String>,
    pub bright_black: Option<String>,
    pub bright_red: Option<String>,
    pub bright_green: Option<String>,
    pub bright_yellow: Option<String>,
    pub bright_blue: Option<String>,
    pub bright_magenta: Option<String>,
    pub bright_cyan: Option<String>,
    pub bright_white: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeMetadata {
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub metadata: ThemeMetadata,
    pub colors: ThemeColors,
    pub ui: Option<UiTheme>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiTheme {
    pub primary: Option<String>,
    pub secondary: Option<String>,
    pub accent: Option<String>,
    pub error: Option<String>,
    pub warning: Option<String>,
    pub success: Option<String>,
    pub text_primary: Option<String>,
    pub text_secondary: Option<String>,
    pub border: Option<String>,
    pub background: Option<String>,
    pub surface: Option<String>,
}

impl Theme {
    pub fn from_json(json_content: &str) -> Result<Self, String> {
        serde_json::from_str(json_content).map_err(|e| format!("Failed to parse theme JSON: {}", e))
    }

    pub fn to_xterm_theme(&self) -> XTermTheme {
        XTermTheme {
            background: self.colors.background.clone(),
            foreground: self.colors.foreground.clone(),
            cursor: self.colors.cursor.clone(),
            cursor_accent: self
                .colors
                .cursor_accent
                .clone()
                .unwrap_or_else(|| self.colors.background.clone()),
            selection_background: self
                .colors
                .selection_background
                .clone()
                .unwrap_or_else(|| "#585b7066".to_string()),
            black: self
                .colors
                .black
                .clone()
                .unwrap_or_else(|| "#45475a".to_string()),
            red: self
                .colors
                .red
                .clone()
                .unwrap_or_else(|| "#f38ba8".to_string()),
            green: self
                .colors
                .green
                .clone()
                .unwrap_or_else(|| "#a6e3a1".to_string()),
            yellow: self
                .colors
                .yellow
                .clone()
                .unwrap_or_else(|| "#f9e2af".to_string()),
            blue: self
                .colors
                .blue
                .clone()
                .unwrap_or_else(|| "#89b4fa".to_string()),
            magenta: self
                .colors
                .magenta
                .clone()
                .unwrap_or_else(|| "#f5c2e7".to_string()),
            cyan: self
                .colors
                .cyan
                .clone()
                .unwrap_or_else(|| "#94e2d5".to_string()),
            white: self
                .colors
                .white
                .clone()
                .unwrap_or_else(|| "#bac2de".to_string()),
            bright_black: self
                .colors
                .bright_black
                .clone()
                .unwrap_or_else(|| "#585b70".to_string()),
            bright_red: self
                .colors
                .bright_red
                .clone()
                .unwrap_or_else(|| "#f38ba8".to_string()),
            bright_green: self
                .colors
                .bright_green
                .clone()
                .unwrap_or_else(|| "#a6e3a1".to_string()),
            bright_yellow: self
                .colors
                .bright_yellow
                .clone()
                .unwrap_or_else(|| "#f9e2af".to_string()),
            bright_blue: self
                .colors
                .bright_blue
                .clone()
                .unwrap_or_else(|| "#89b4fa".to_string()),
            bright_magenta: self
                .colors
                .bright_magenta
                .clone()
                .unwrap_or_else(|| "#f5c2e7".to_string()),
            bright_cyan: self
                .colors
                .bright_cyan
                .clone()
                .unwrap_or_else(|| "#94e2d5".to_string()),
            bright_white: self
                .colors
                .bright_white
                .clone()
                .unwrap_or_else(|| "#a6adc8".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XTermTheme {
    pub background: String,
    pub foreground: String,
    pub cursor: String,
    #[serde(rename = "cursorAccent")]
    pub cursor_accent: String,
    #[serde(rename = "selectionBackground")]
    pub selection_background: String,
    pub black: String,
    pub red: String,
    pub green: String,
    pub yellow: String,
    pub blue: String,
    pub magenta: String,
    pub cyan: String,
    pub white: String,
    #[serde(rename = "brightBlack")]
    pub bright_black: String,
    #[serde(rename = "brightRed")]
    pub bright_red: String,
    #[serde(rename = "brightGreen")]
    pub bright_green: String,
    #[serde(rename = "brightYellow")]
    pub bright_yellow: String,
    #[serde(rename = "brightBlue")]
    pub bright_blue: String,
    #[serde(rename = "brightMagenta")]
    pub bright_magenta: String,
    #[serde(rename = "brightCyan")]
    pub bright_cyan: String,
    #[serde(rename = "brightWhite")]
    pub bright_white: String,
}

pub struct ThemeEngine {
    themes_dir: PathBuf,
    built_in_themes: HashMap<String, Theme>,
}

impl ThemeEngine {
    pub fn new(skins_dir: PathBuf) -> Self {
        let mut engine = Self {
            themes_dir: skins_dir,
            built_in_themes: HashMap::new(),
        };
        engine.load_built_in_themes();
        engine
    }

    fn load_built_in_themes(&mut self) {
        self.built_in_themes.insert(
            "default".to_string(),
            Theme {
                metadata: ThemeMetadata {
                    name: "Default".to_string(),
                    version: "1.0.0".to_string(),
                    author: Some("AxAgent Team".to_string()),
                    description: Some("Default theme with Catppuccin Mocha colors".to_string()),
                },
                colors: ThemeColors {
                    background: "#1e1e2e".to_string(),
                    foreground: "#cdd6f4".to_string(),
                    cursor: "#f5e0dc".to_string(),
                    cursor_accent: Some("#1e1e2e".to_string()),
                    selection_background: Some("#585b7066".to_string()),
                    black: Some("#45475a".to_string()),
                    red: Some("#f38ba8".to_string()),
                    green: Some("#a6e3a1".to_string()),
                    yellow: Some("#f9e2af".to_string()),
                    blue: Some("#89b4fa".to_string()),
                    magenta: Some("#f5c2e7".to_string()),
                    cyan: Some("#94e2d5".to_string()),
                    white: Some("#bac2de".to_string()),
                    bright_black: Some("#585b70".to_string()),
                    bright_red: Some("#f38ba8".to_string()),
                    bright_green: Some("#a6e3a1".to_string()),
                    bright_yellow: Some("#f9e2af".to_string()),
                    bright_blue: Some("#89b4fa".to_string()),
                    bright_magenta: Some("#f5c2e7".to_string()),
                    bright_cyan: Some("#94e2d5".to_string()),
                    bright_white: Some("#a6adc8".to_string()),
                },
                ui: Some(UiTheme {
                    primary: Some("#89b4fa".to_string()),
                    secondary: Some("#cba6f7".to_string()),
                    accent: Some("#f5c2e7".to_string()),
                    error: Some("#f38ba8".to_string()),
                    warning: Some("#f9e2af".to_string()),
                    success: Some("#a6e3a1".to_string()),
                    text_primary: Some("#cdd6f4".to_string()),
                    text_secondary: Some("#6c7086".to_string()),
                    border: Some("#45475a".to_string()),
                    background: Some("#181825".to_string()),
                    surface: Some("#1e1e2e".to_string()),
                }),
            },
        );

        self.built_in_themes.insert(
            "catppuccin-mocha".to_string(),
            self.built_in_themes.get("default").cloned().unwrap(),
        );

        self.built_in_themes.insert(
            "monokai".to_string(),
            Theme {
                metadata: ThemeMetadata {
                    name: "Monokai".to_string(),
                    version: "1.0.0".to_string(),
                    author: Some("Wimer Hazenberg".to_string()),
                    description: Some("Monokai color scheme".to_string()),
                },
                colors: ThemeColors {
                    background: "#272822".to_string(),
                    foreground: "#f8f8f2".to_string(),
                    cursor: "#f8f8f0".to_string(),
                    cursor_accent: Some("#272822".to_string()),
                    selection_background: Some("#49483E".to_string()),
                    black: Some("#272822".to_string()),
                    red: Some("#f92672".to_string()),
                    green: Some("#a6e22e".to_string()),
                    yellow: Some("#f4bf75".to_string()),
                    blue: Some("#66d9ef".to_string()),
                    magenta: Some("#ae81ff".to_string()),
                    cyan: Some("#a1efe4".to_string()),
                    white: Some("#f8f8f2".to_string()),
                    bright_black: Some("#75715E".to_string()),
                    bright_red: Some("#f92672".to_string()),
                    bright_green: Some("#a6e22e".to_string()),
                    bright_yellow: Some("#f4bf75".to_string()),
                    bright_blue: Some("#66d9ef".to_string()),
                    bright_magenta: Some("#ae81ff".to_string()),
                    bright_cyan: Some("#a1efe4".to_string()),
                    bright_white: Some("#f9f8f5".to_string()),
                },
                ui: Some(UiTheme {
                    primary: Some("#66d9ef".to_string()),
                    secondary: Some("#ae81ff".to_string()),
                    accent: Some("#a6e22e".to_string()),
                    error: Some("#f92672".to_string()),
                    warning: Some("#f4bf75".to_string()),
                    success: Some("#a6e22e".to_string()),
                    text_primary: Some("#f8f8f2".to_string()),
                    text_secondary: Some("#75715E".to_string()),
                    border: Some("#49483E".to_string()),
                    background: Some("#1e1e1e".to_string()),
                    surface: Some("#272822".to_string()),
                }),
            },
        );

        self.built_in_themes.insert(
            "gruvbox".to_string(),
            Theme {
                metadata: ThemeMetadata {
                    name: "Gruvbox Dark".to_string(),
                    version: "1.0.0".to_string(),
                    author: Some("github.com/morhetz/gruvbox".to_string()),
                    description: Some("Gruvbox dark theme".to_string()),
                },
                colors: ThemeColors {
                    background: "#282828".to_string(),
                    foreground: "#ebdbb2".to_string(),
                    cursor: "#ebdbb2".to_string(),
                    cursor_accent: Some("#282828".to_string()),
                    selection_background: Some("#3c3836".to_string()),
                    black: Some("#282828".to_string()),
                    red: Some("#cc241d".to_string()),
                    green: Some("#98971a".to_string()),
                    yellow: Some("#d79921".to_string()),
                    blue: Some("#458588".to_string()),
                    magenta: Some("#b16286".to_string()),
                    cyan: Some("#689d6a".to_string()),
                    white: Some("#a89984".to_string()),
                    bright_black: Some("#928374".to_string()),
                    bright_red: Some("#fb4934".to_string()),
                    bright_green: Some("#b8bb26".to_string()),
                    bright_yellow: Some("#fabd2f".to_string()),
                    bright_blue: Some("#83a598".to_string()),
                    bright_magenta: Some("#d3869b".to_string()),
                    bright_cyan: Some("#8ec07c".to_string()),
                    bright_white: Some("#ebdbb2".to_string()),
                },
                ui: None,
            },
        );
    }

    pub fn get_theme(&self, name: &str) -> Option<Theme> {
        if let Some(theme) = self.built_in_themes.get(name) {
            return Some(theme.clone());
        }

        let custom_path = self.themes_dir.join(format!("{}.json", name));
        if custom_path.exists() {
            if let Ok(content) = fs::read_to_string(&custom_path) {
                return Theme::from_json(&content).ok();
            }
        }

        None
    }

    pub fn list_themes(&self) -> Vec<ThemeMetadata> {
        let mut themes: Vec<ThemeMetadata> = self
            .built_in_themes
            .values()
            .map(|t| t.metadata.clone())
            .collect();

        if let Ok(entries) = fs::read_dir(&self.themes_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(theme) = Theme::from_json(&content) {
                            themes.push(theme.metadata);
                        }
                    }
                }
            }
        }

        themes
    }

    pub fn load_user_themes(&self) -> Vec<Theme> {
        let mut themes = Vec::new();

        if let Ok(entries) = fs::read_dir(&self.themes_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "yaml").unwrap_or(false) {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(theme) = Theme::from_json(&content) {
                            themes.push(theme);
                        }
                    }
                }
            }
        }

        themes
    }

    pub fn save_theme(&self, theme: &Theme) -> Result<(), String> {
        let file_name = format!(
            "{}.json",
            theme.metadata.name.to_lowercase().replace(' ', "-")
        );
        let file_path = self.themes_dir.join(&file_name);

        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create themes directory: {}", e))?;
        }

        let json = serde_json::to_string_pretty(theme)
            .map_err(|e| format!("Failed to serialize theme: {}", e))?;

        fs::write(&file_path, json).map_err(|e| format!("Failed to write theme file: {}", e))?;

        Ok(())
    }

    pub fn delete_theme(&self, name: &str) -> Result<(), String> {
        if self.built_in_themes.contains_key(name) {
            return Err("Cannot delete built-in theme".to_string());
        }

        let file_name = format!("{}.json", name.to_lowercase().replace(' ', "-"));
        let file_path = self.themes_dir.join(&file_name);

        if file_path.exists() {
            fs::remove_file(&file_path).map_err(|e| format!("Failed to delete theme: {}", e))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_theme() {
        let engine = ThemeEngine::new(PathBuf::from("."));
        let theme = engine.get_theme("default").unwrap();
        assert_eq!(theme.metadata.name, "Default");
        assert_eq!(theme.colors.background, "#1e1e2e");
    }

    #[test]
    fn test_xterm_theme_conversion() {
        let engine = ThemeEngine::new(PathBuf::from("."));
        let theme = engine.get_theme("default").unwrap();
        let xterm_theme = theme.to_xterm_theme();
        assert_eq!(xterm_theme.background, "#1e1e2e");
        assert_eq!(xterm_theme.foreground, "#cdd6f4");
    }
}
