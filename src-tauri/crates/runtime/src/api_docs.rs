use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiSpec {
    pub openapi: String,
    pub info: InfoObject,
    pub servers: Vec<ServerObject>,
    pub paths: HashMap<String, HashMap<String, OperationObject>>,
    pub components: Option<ComponentsObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoObject {
    pub title: String,
    pub description: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerObject {
    pub url: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationObject {
    pub summary: String,
    pub description: Option<String>,
    pub operation_id: String,
    pub tags: Vec<String>,
    pub parameters: Vec<ParameterObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_body: Option<RequestBodyObject>,
    pub responses: HashMap<String, ResponseObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterObject {
    pub name: String,
    #[serde(rename = "in")]
    pub location: String,
    pub description: String,
    pub required: bool,
    pub schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBodyObject {
    pub description: String,
    pub content: HashMap<String, MediaTypeObject>,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaTypeObject {
    pub schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseObject {
    pub description: String,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub content: HashMap<String, MediaTypeObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentsObject {
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub schemas: HashMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub security_schemes: HashMap<String, SecuritySchemeObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySchemeObject {
    #[serde(rename = "type")]
    pub scheme_type: String,
    pub scheme: Option<String>,
    pub bearer_format: Option<String>,
    pub description: Option<String>,
}

pub struct ApiDocGenerator {
    spec: OpenApiSpec,
}

impl ApiDocGenerator {
    pub fn new(title: &str, version: &str) -> Self {
        Self {
            spec: OpenApiSpec {
                openapi: "3.0.3".to_string(),
                info: InfoObject {
                    title: title.to_string(),
                    description: format!("{} API Documentation", title),
                    version: version.to_string(),
                },
                servers: vec![ServerObject {
                    url: "http://localhost:5174".to_string(),
                    description: "Local development server".to_string(),
                }],
                paths: HashMap::new(),
                components: Some(ComponentsObject {
                    schemas: HashMap::new(),
                    security_schemes: {
                        let mut schemes = HashMap::new();
                        schemes.insert(
                            "BearerAuth".to_string(),
                            SecuritySchemeObject {
                                scheme_type: "http".to_string(),
                                scheme: Some("bearer".to_string()),
                                bearer_format: Some("JWT".to_string()),
                                description: Some("API key authentication".to_string()),
                            },
                        );
                        schemes
                    },
                }),
            },
        }
    }

    pub fn add_operation(&mut self, path: &str, method: &str, operation: OperationObject) {
        self.spec
            .paths
            .entry(path.to_string())
            .or_insert_with(HashMap::new)
            .insert(method.to_lowercase(), operation);
    }

    pub fn add_chat_completion(&mut self) {
        self.add_operation("/v1/chat/completions", "post", OperationObject {
            summary: "Create a chat completion".to_string(),
            description: Some("Send messages and receive AI-generated responses".to_string()),
            operation_id: "createChatCompletion".to_string(),
            tags: vec!["Chat".to_string()],
            parameters: vec![],
            request_body: Some(RequestBodyObject {
                description: "Chat completion request".to_string(),
                content: {
                    let mut content = HashMap::new();
                    content.insert("application/json".to_string(), MediaTypeObject {
                        schema: serde_json::json!({
                            "type": "object",
                            "required": ["model", "messages"],
                            "properties": {
                                "model": {"type": "string", "description": "Model ID"},
                                "messages": {"type": "array", "items": {"$ref": "#/components/schemas/ChatMessage"}},
                                "stream": {"type": "boolean", "default": false},
                                "temperature": {"type": "number", "minimum": 0, "maximum": 2},
                                "max_tokens": {"type": "integer", "minimum": 1},
                            }
                        }),
                    });
                    content
                },
                required: true,
            }),
            responses: {
                let mut responses = HashMap::new();
                responses.insert("200".to_string(), ResponseObject {
                    description: "Successful response".to_string(),
                    content: HashMap::new(),
                });
                responses.insert("401".to_string(), ResponseObject {
                    description: "Unauthorized".to_string(),
                    content: HashMap::new(),
                });
                responses.insert("429".to_string(), ResponseObject {
                    description: "Rate limited".to_string(),
                    content: HashMap::new(),
                });
                responses
            },
        });
    }

    pub fn add_models_endpoint(&mut self) {
        self.add_operation(
            "/v1/models",
            "get",
            OperationObject {
                summary: "List available models".to_string(),
                description: Some("Get a list of all available AI models".to_string()),
                operation_id: "listModels".to_string(),
                tags: vec!["Models".to_string()],
                parameters: vec![],
                request_body: None,
                responses: {
                    let mut responses = HashMap::new();
                    responses.insert(
                        "200".to_string(),
                        ResponseObject {
                            description: "List of models".to_string(),
                            content: HashMap::new(),
                        },
                    );
                    responses
                },
            },
        );
    }

    pub fn add_schema(&mut self, name: &str, schema: serde_json::Value) {
        if let Some(ref mut components) = self.spec.components {
            components.schemas.insert(name.to_string(), schema);
        }
    }

    pub fn generate_default_spec() -> OpenApiSpec {
        let mut gen = Self::new("AxAgent Gateway", env!("CARGO_PKG_VERSION"));
        gen.add_chat_completion();
        gen.add_models_endpoint();
        gen.add_schema(
            "ChatMessage",
            serde_json::json!({
                "type": "object",
                "required": ["role", "content"],
                "properties": {
                    "role": {"type": "string", "enum": ["system", "user", "assistant", "tool"]},
                    "content": {"type": "string"},
                    "name": {"type": "string"},
                    "tool_calls": {"type": "array"},
                }
            }),
        );
        gen.build()
    }

    pub fn build(self) -> OpenApiSpec {
        self.spec
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.spec)
    }

    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(&self.spec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_default_spec() {
        let _spec = ApiDocGenerator::generate_default_spec();
        assert_eq!(spec.openapi, "3.0.3");
        assert!(spec.paths.contains_key("/v1/chat/completions"));
        assert!(spec.paths.contains_key("/v1/models"));
    }

    #[test]
    fn test_to_json() {
        let spec = ApiDocGenerator::generate_default_spec();
        let json = ApiDocGenerator::new("Test", "1.0.0").to_json();
        assert!(json.is_ok());
        assert!(json.unwrap().contains("\"openapi\""));
    }

    #[test]
    fn test_add_custom_operation() {
        let mut gen = ApiDocGenerator::new("Test", "1.0.0");
        gen.add_operation(
            "/v1/custom",
            "get",
            OperationObject {
                summary: "Custom endpoint".to_string(),
                description: None,
                operation_id: "customEndpoint".to_string(),
                tags: vec!["Custom".to_string()],
                parameters: vec![ParameterObject {
                    name: "id".to_string(),
                    location: "query".to_string(),
                    description: "Item ID".to_string(),
                    required: false,
                    schema: Some(serde_json::json!({"type": "string"})),
                }],
                request_body: None,
                responses: HashMap::new(),
            },
        );

        let spec = gen.build();
        assert!(spec.paths.contains_key("/v1/custom"));
        let get_op = spec.paths.get("/v1/custom").unwrap().get("get").unwrap();
        assert_eq!(get_op.parameters.len(), 1);
    }
}
