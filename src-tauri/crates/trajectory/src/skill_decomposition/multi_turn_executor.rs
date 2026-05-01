use super::multi_turn::DecompositionEvent;
use super::package_parser::SkillPackageParser;
use super::prompt_templates::PromptTemplates;
use super::workflow_validator::WorkflowValidator;
use crate::skill_decomposition::DecompositionResult;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

pub trait LlmClient: Send + Sync {
    fn chat(
        &self,
        messages: Vec<ChatMessageInput>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send + '_>>;
}

#[derive(Debug, Clone)]
pub struct ChatMessageInput {
    pub role: String,
    pub content: String,
}

pub struct MultiTurnDecomposer<C: LlmClient> {
    llm_client: C,
}

impl<C: LlmClient> MultiTurnDecomposer<C> {
    pub fn new(llm_client: C) -> Self {
        Self { llm_client }
    }

    pub async fn decompose_stream(
        &self,
        files: Vec<(String, String)>,
        tx: mpsc::Sender<DecompositionEvent>,
    ) -> Result<DecompositionResult, String> {
        let parsed_files = SkillPackageParser::parse_files(files.clone());
        let file_list = Self::build_file_list(&parsed_files);
        let file_summaries = Self::build_file_summaries(&parsed_files);
        let templates = PromptTemplates::get_all_turns();
        let mut results = Vec::new();

        for (turn_idx, template) in templates.iter().enumerate() {
            let turn_id = (turn_idx + 1) as u32;

            tx.send(DecompositionEvent::turn_start(
                turn_id,
                &format!("开始：{}", Self::turn_name(turn_id)),
            ))
            .await
            .map_err(|e| e.to_string())?;

            let user_content = match turn_id {
                1 => PromptTemplates::format_turn1_user_content(&file_list, &file_summaries),
                2 => {
                    let structure = results.first().map(|r: &String| r.as_str()).unwrap_or("{}");
                    let contents = Self::build_file_contents_for_classify(&parsed_files);
                    PromptTemplates::format_turn2_user_content(structure, &contents)
                },
                3 => {
                    let language = Self::detect_primary_language(&parsed_files);
                    PromptTemplates::format_turn3_user_content(
                        &format!("基于文件：{}", file_list),
                        &language,
                        &Self::extract_code_from_files(&parsed_files),
                    )
                },
                4 => {
                    let analysis = results.get(2).map(|r: &String| r.as_str()).unwrap_or("{}");
                    PromptTemplates::format_turn4_user_content(analysis, "[]", "[]")
                },
                5 => {
                    let workflow = results.get(3).map(|r: &String| r.as_str()).unwrap_or("{}");
                    let analysis = results.get(2).map(|r: &String| r.as_str()).unwrap_or("{}");
                    PromptTemplates::format_turn5_user_content(workflow, analysis)
                },
                _ => template.user_template.clone(),
            };

            let messages = vec![
                ChatMessageInput {
                    role: "system".to_string(),
                    content: template.system.clone(),
                },
                ChatMessageInput {
                    role: "user".to_string(),
                    content: user_content,
                },
            ];

            let msg_id = uuid::Uuid::new_v4().to_string();
            tx.send(DecompositionEvent::turn_start(turn_id, "正在分析..."))
                .await
                .map_err(|e| e.to_string())?;

            let full_response = self.llm_client.chat(messages).await?;

            let chunks: Vec<&str> = full_response.lines().collect();
            let total_chunks = chunks.len();
            for (idx, chunk) in chunks.into_iter().enumerate() {
                if chunk.is_empty() {
                    continue;
                }
                let delay = if idx == total_chunks - 1 { 50 } else { 30 };
                sleep(Duration::from_millis(delay)).await;
                tx.send(DecompositionEvent::message_chunk(turn_id, &msg_id, chunk))
                    .await
                    .map_err(|e| e.to_string())?;
            }

            let parsed_metadata = serde_json::from_str::<serde_json::Value>(&full_response).ok();

            tx.send(DecompositionEvent::message_complete(
                turn_id,
                &msg_id,
                &full_response,
                parsed_metadata,
            ))
            .await
            .map_err(|e| e.to_string())?;

            results.push(full_response);

            tx.send(DecompositionEvent::turn_complete(
                turn_id,
                &format!("完成：{}", Self::turn_name(turn_id)),
                Some(serde_json::json!({ "turn_result": turn_id })),
            ))
            .await
            .map_err(|e| e.to_string())?;

            sleep(Duration::from_millis(200)).await;
        }

        let final_result = Self::build_final_result(&results);
        let final_json: serde_json::Value = serde_json::from_str(&final_result).unwrap_or_default();

        tx.send(DecompositionEvent::final_result(final_json.clone()))
            .await
            .map_err(|e| e.to_string())?;

        Self::parse_final_result(&final_json)
    }

    fn build_file_list(files: &[super::multi_turn::SkillFile]) -> String {
        files
            .iter()
            .map(|f| format!("- {} ({:?})", f.path, f.file_type))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn build_file_summaries(files: &[super::multi_turn::SkillFile]) -> String {
        files
            .iter()
            .map(|f| {
                let code_blocks = f.code_blocks.len();
                let refs = f.references.len();
                format!(
                    "- {}: {} 行, {} 个代码块, {} 个引用",
                    f.path,
                    f.content.lines().count(),
                    code_blocks,
                    refs
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn build_file_contents_for_classify(files: &[super::multi_turn::SkillFile]) -> String {
        files
            .iter()
            .map(|f| {
                format!(
                    "=== {} ===\n{}\n--- 代码块 ---\n{}\n--- 引用 ---\n{}",
                    f.path,
                    &f.content[..f.content.len().min(500)],
                    f.code_blocks
                        .iter()
                        .map(|cb| format!(
                            "[{}] {}",
                            cb.language.clone().unwrap_or_default(),
                            &cb.content[..cb.content.len().min(100)]
                        ))
                        .collect::<Vec<_>>()
                        .join("\n"),
                    f.references
                        .iter()
                        .map(|r| format!("{} -> {}", r.from_file, r.to_file))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    fn detect_primary_language(files: &[super::multi_turn::SkillFile]) -> String {
        let mut language_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for f in files {
            let lang = f.file_type.language_name();
            if !lang.is_empty() {
                *language_counts.entry(lang.to_string()).or_insert(0) += f.content.lines().count();
            }
        }

        language_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(lang, _)| lang)
            .unwrap_or_else(|| "python".to_string())
    }

    fn extract_code_from_files(files: &[super::multi_turn::SkillFile]) -> String {
        files
            .iter()
            .filter(|f| f.file_type.is_code())
            .map(|f| f.content.clone())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    fn turn_name(turn: u32) -> &'static str {
        match turn {
            1 => "文件理解",
            2 => "内容分类",
            3 => "功能分析",
            4 => "工作流设计",
            5 => "生成输出",
            _ => "未知",
        }
    }

    fn build_final_result(results: &[String]) -> String {
        results.last().cloned().unwrap_or_else(|| "{}".to_string())
    }

    fn parse_final_result(json: &serde_json::Value) -> Result<DecompositionResult, String> {
        let validation_result = WorkflowValidator::validate(json);

        let validated_json = if let Some(corrected) = validation_result.corrected_workflow {
            tracing::warn!(
                "Workflow validation found {} issues, corrections applied",
                validation_result.issues.len()
            );
            for issue in &validation_result.issues {
                tracing::debug!(
                    "Validation issue: {:?} - {} (corrected: {:?})",
                    issue.severity,
                    issue.message,
                    issue.corrected_value
                );
            }
            corrected
        } else {
            json.clone()
        };

        let workflow_value = validated_json.get("workflow").cloned().unwrap_or_default();

        Ok(DecompositionResult {
            atomic_skills: validated_json
                .get("atomic_skills")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default(),
            tool_dependencies: validated_json
                .get("tool_dependencies")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default(),
            workflow_nodes: workflow_value.get("nodes").cloned().unwrap_or_default(),
            workflow_edges: workflow_value.get("edges").cloned().unwrap_or_default(),
            original_source: super::decomposer::CompositeSourceInfo {
                market: "multi_turn_decomposition".to_string(),
                repo: None,
                version: None,
            },
            original_content: String::new(),
            parsed_steps_metadata: vec![],
            code_blocks: vec![],
        })
    }
}
