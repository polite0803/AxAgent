use serde_json::Value;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TemplateError {
    #[error("Variable `{0}` is missing")]
    MissingVariable(String),
    #[error("Invalid variable type for `{variable}`: expected {expected}, got {actual}")]
    InvalidType {
        variable: String,
        expected: String,
        actual: String,
    },
    #[error("Template parsing error: {0}")]
    ParseError(String),
    #[error("JSON Schema validation failed: {0}")]
    SchemaValidationError(String),
}

pub struct PromptTemplateRenderer {
    template: String,
    variables: Vec<String>,
}

impl PromptTemplateRenderer {
    pub fn new(template: impl Into<String>) -> Self {
        let template = template.into();
        let variables = Self::extract_variables(&template);
        Self {
            template,
            variables,
        }
    }

    pub fn with_schema(template: impl Into<String>, _schema: Option<&Value>) -> Self {
        Self::new(template)
    }

    fn extract_variables(template: &str) -> Vec<String> {
        let mut variables = Vec::new();
        let mut chars = template.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' {
                let mut var_name = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '}' {
                        chars.next();
                        break;
                    }
                    var_name.push(c);
                    chars.next();
                }
                if !var_name.is_empty() {
                    variables.push(var_name);
                }
            }
        }

        variables
    }

    pub fn render(&self, vars: &Value) -> Result<String, TemplateError> {
        let mut result = self.template.clone();

        for var in &self.variables {
            let placeholder = format!("{{{}}}", var);
            let value = vars
                .get(var)
                .ok_or_else(|| TemplateError::MissingVariable(var.clone()))?;

            let value_str = match value {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Null => String::new(),
                _ => value.to_string(),
            };

            result = result.replace(&placeholder, &value_str);
        }

        Ok(result)
    }

    pub fn validate(&self, vars: &Value) -> Result<(), TemplateError> {
        if let Value::Object(obj) = vars {
            for var in &self.variables {
                if !obj.contains_key(var) {
                    return Err(TemplateError::MissingVariable(var.clone()));
                }
            }
            Ok(())
        } else {
            Err(TemplateError::SchemaValidationError(
                "Variables must be a JSON object".to_string(),
            ))
        }
    }

    pub fn get_variables(&self) -> &[String] {
        &self.variables
    }
}

impl From<&str> for PromptTemplateRenderer {
    fn from(template: &str) -> Self {
        Self::new(template)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_variables() {
        let template = PromptTemplateRenderer::new("Hello, {name}! You have {count} messages.");
        assert_eq!(template.get_variables(), &["name", "count"]);
    }

    #[test]
    fn test_render_simple() {
        let renderer = PromptTemplateRenderer::new("Hello, {name}!");
        let vars = json!({"name": "World"});
        assert_eq!(renderer.render(&vars).unwrap(), "Hello, World!");
    }

    #[test]
    fn test_render_missing_variable() {
        let renderer = PromptTemplateRenderer::new("Hello, {name}!");
        let vars = json!({});
        assert!(matches!(
            renderer.render(&vars),
            Err(TemplateError::MissingVariable(_))
        ));
    }

    #[test]
    fn test_render_with_number() {
        let renderer = PromptTemplateRenderer::new("Count: {count}");
        let vars = json!({"count": 42});
        assert_eq!(renderer.render(&vars).unwrap(), "Count: 42");
    }

    #[test]
    fn test_validate() {
        let renderer = PromptTemplateRenderer::new("Hello, {name}!");
        let vars = json!({"name": "World"});
        assert!(renderer.validate(&vars).is_ok());
    }

    #[test]
    fn test_validate_missing() {
        let renderer = PromptTemplateRenderer::new("Hello, {name}!");
        let vars = json!({});
        assert!(renderer.validate(&vars).is_err());
    }
}
