use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBehavior {
    pub action_type: String,
    pub context: String,
    pub accepted: bool,
    pub timestamp: i64,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleProfile {
    pub user_id: String,
    pub coding_style: CodingStyle,
    pub preferred_patterns: Vec<String>,
    pub rejected_patterns: Vec<String>,
    pub common_apis: Vec<String>,
    pub naming_convention: String,
    pub indent_style: String,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingStyle {
    pub indent_style: String,
    pub quote_style: String,
    pub semicolons: bool,
    pub max_line_length: usize,
    pub function_style: String,
}

impl Default for CodingStyle {
    fn default() -> Self {
        Self {
            indent_style: "spaces".into(),
            quote_style: "single".into(),
            semicolons: true,
            max_line_length: 100,
            function_style: "named".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorPattern {
    pub pattern_type: String,
    pub pattern_value: String,
    pub frequency: u32,
    pub acceptance_rate: f64,
    pub last_seen: i64,
}

pub struct BehaviorLearner {
    behaviors: Vec<UserBehavior>,
    style_profile: Option<StyleProfile>,
    user_id: String,
}

impl BehaviorLearner {
    pub fn new(user_id: &str) -> Self {
        Self {
            behaviors: Vec::new(),
            style_profile: None,
            user_id: user_id.into(),
        }
    }

    pub fn record_behavior(&mut self, behavior: UserBehavior) {
        self.behaviors.push(behavior);
        if self.behaviors.len().is_multiple_of(50) {
            self.recompute_profile();
        }
    }

    pub fn record_batch(&mut self, behaviors: Vec<UserBehavior>) {
        self.behaviors.extend(behaviors);
        self.recompute_profile();
    }

    fn recompute_profile(&mut self) {
        let accepted: Vec<&UserBehavior> = self.behaviors.iter().filter(|b| b.accepted).collect();
        let rejected: Vec<&UserBehavior> = self.behaviors.iter().filter(|b| !b.accepted).collect();

        let preferred_patterns = Self::extract_patterns(&accepted);
        let rejected_patterns = Self::extract_patterns(&rejected);

        let common_apis = self.extract_common_apis(&accepted);
        let naming_convention = self.detect_naming_convention(&accepted);
        let indent_style = self.detect_indent_style(&accepted);

        self.style_profile = Some(StyleProfile {
            user_id: self.user_id.clone(),
            coding_style: CodingStyle::default(),
            preferred_patterns,
            rejected_patterns,
            common_apis,
            naming_convention,
            indent_style,
            updated_at: Utc::now().timestamp(),
        });
    }

    pub fn get_style_hints(&self) -> Vec<String> {
        self.style_profile
            .as_ref()
            .map(|p| {
                let mut hints = vec![];
                if !p.preferred_patterns.is_empty() {
                    hints.push(format!("User prefers: {}", p.preferred_patterns.join(", ")));
                }
                if !p.rejected_patterns.is_empty() {
                    hints.push(format!("User avoids: {}", p.rejected_patterns.join(", ")));
                }
                if !p.common_apis.is_empty() {
                    hints.push(format!("Common APIs: {}", p.common_apis.join(", ")));
                }
                if !p.naming_convention.is_empty() {
                    hints.push(format!("Naming convention: {}", p.naming_convention));
                }
                hints
            })
            .unwrap_or_default()
    }

    pub fn get_patterns(&self) -> Vec<BehaviorPattern> {
        let mut pattern_map: HashMap<(String, String), (u32, u32)> = HashMap::new();
        for behavior in &self.behaviors {
            let key = (behavior.action_type.clone(), behavior.context.clone());
            let entry = pattern_map.entry(key).or_insert((0, 0));
            entry.0 += 1;
            if behavior.accepted {
                entry.1 += 1;
            }
        }

        pattern_map
            .into_iter()
            .map(|((action, ctx), (total, accepted))| BehaviorPattern {
                pattern_type: action,
                pattern_value: ctx,
                frequency: total,
                acceptance_rate: if total > 0 {
                    accepted as f64 / total as f64
                } else {
                    0.0
                },
                last_seen: Utc::now().timestamp(),
            })
            .collect()
    }

    pub fn get_style_profile(&self) -> Option<&StyleProfile> {
        self.style_profile.as_ref()
    }

    fn extract_patterns(behaviors: &[&UserBehavior]) -> Vec<String> {
        let mut freq: HashMap<String, u32> = HashMap::new();
        for b in behaviors {
            let key = format!("{}:{}", b.action_type, b.context);
            *freq.entry(key).or_default() += 1;
        }
        let mut patterns: Vec<(String, u32)> = freq.into_iter().collect();
        patterns.sort_by(|a, b| b.1.cmp(&a.1));
        patterns.into_iter().take(10).map(|(k, _)| k).collect()
    }

    fn extract_common_apis(&self, behaviors: &[&UserBehavior]) -> Vec<String> {
        let mut freq: HashMap<String, u32> = HashMap::new();
        for b in behaviors {
            if let Some(api) = b.metadata.get("api") {
                *freq.entry(api.clone()).or_default() += 1;
            }
        }
        let mut apis: Vec<(String, u32)> = freq.into_iter().collect();
        apis.sort_by(|a, b| b.1.cmp(&a.1));
        apis.into_iter().take(10).map(|(k, _)| k).collect()
    }

    fn detect_naming_convention(&self, behaviors: &[&UserBehavior]) -> String {
        let mut camel_case = 0u32;
        let mut snake_case = 0u32;
        let mut pascal_case = 0u32;

        for b in behaviors {
            if let Some(name) = b.metadata.get("identifier") {
                if name.contains('_') {
                    snake_case += 1;
                } else if name.chars().next().is_some_and(|c| c.is_uppercase()) {
                    pascal_case += 1;
                } else {
                    camel_case += 1;
                }
            }
        }

        let max = *[camel_case, snake_case, pascal_case]
            .iter()
            .max()
            .unwrap_or(&0);
        if max == snake_case {
            "snake_case".into()
        } else if max == pascal_case {
            "PascalCase".into()
        } else {
            "camelCase".into()
        }
    }

    fn detect_indent_style(&self, behaviors: &[&UserBehavior]) -> String {
        let mut spaces = 0u32;
        let mut tabs = 0u32;
        for b in behaviors {
            if let Some(indent) = b.metadata.get("indent") {
                if indent == "tabs" {
                    tabs += 1;
                } else {
                    spaces += 1;
                }
            }
        }
        if tabs > spaces {
            "tabs".into()
        } else {
            "spaces".into()
        }
    }
}
