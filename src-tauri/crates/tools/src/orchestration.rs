//! 工具编排器 - 并发/串行执行工具调用

use crate::registry::ToolRegistry;
use crate::{ToolContext, ToolError, ToolResult};
use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Debug, Clone)]
pub struct ToolCallRequest {
    pub id: String,
    pub name: String,
    pub input: String,
}

#[derive(Debug, Clone)]
pub struct ToolCallResponse {
    pub id: String,
    pub name: String,
    pub result: Result<ToolResult, ToolError>,
}

#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub max_concurrency: usize,
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_concurrency: 10,
            timeout_secs: 120,
            max_retries: 3,
            retry_delay_ms: 1000,
        }
    }
}

pub struct Orchestrator {
    config: OrchestratorConfig,
}

impl Orchestrator {
    pub fn new(config: OrchestratorConfig) -> Self {
        Self { config }
    }

    pub async fn execute(
        &self,
        requests: Vec<ToolCallRequest>,
        registry: Arc<ToolRegistry>,
        ctx: &ToolContext,
    ) -> Vec<ToolCallResponse> {
        let (concurrent, serial) = self.partition(requests, &registry);

        let mut results = Vec::new();

        if !concurrent.is_empty() {
            let concurrent_results = self
                .run_concurrently(concurrent, registry.clone(), ctx)
                .await;
            results.extend(concurrent_results);
        }

        for request in serial {
            let result = self.run_single(request, registry.clone(), ctx).await;
            results.push(result);
        }

        results
    }

    fn partition(
        &self,
        requests: Vec<ToolCallRequest>,
        registry: &ToolRegistry,
    ) -> (Vec<ToolCallRequest>, Vec<ToolCallRequest>) {
        let mut concurrent = Vec::new();
        let mut serial = Vec::new();

        for req in requests {
            let is_safe = registry
                .find(&req.name)
                .map(|t| t.is_concurrency_safe())
                .unwrap_or(false);

            if is_safe {
                concurrent.push(req);
            } else {
                serial.push(req);
            }
        }

        (concurrent, serial)
    }

    async fn run_concurrently(
        &self,
        requests: Vec<ToolCallRequest>,
        registry: Arc<ToolRegistry>,
        ctx: &ToolContext,
    ) -> Vec<ToolCallResponse> {
        let semaphore = Arc::new(Semaphore::new(self.config.max_concurrency));
        let mut handles = Vec::with_capacity(requests.len());

        let max_retries = self.config.max_retries;
        let retry_delay_ms = self.config.retry_delay_ms;
        for request in requests {
            let sem = semaphore.clone();
            let reg = registry.clone();
            let context = ctx.clone();
            let timeout = self.config.timeout_secs;

            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await;
                run_single(request, reg, &context, timeout, max_retries, retry_delay_ms).await
            });

            handles.push(handle);
        }

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.await {
                Ok(response) => results.push(response),
                Err(_) => {
                    results.push(ToolCallResponse {
                        id: "error".into(),
                        name: "error".into(),
                        result: Err(ToolError::execution_failed_for(
                            "Orchestrator",
                            "并发任务 panic",
                        )),
                    });
                },
            }
        }

        results
    }

    async fn run_single(
        &self,
        request: ToolCallRequest,
        registry: Arc<ToolRegistry>,
        ctx: &ToolContext,
    ) -> ToolCallResponse {
        run_single(
            request,
            registry,
            ctx,
            self.config.timeout_secs,
            self.config.max_retries,
            self.config.retry_delay_ms,
        )
        .await
    }
}

async fn run_single(
    request: ToolCallRequest,
    registry: Arc<ToolRegistry>,
    ctx: &ToolContext,
    timeout_secs: u64,
    max_retries: u32,
    retry_delay_ms: u64,
) -> ToolCallResponse {
    let name = request.name.clone();
    let id = request.id.clone();

    let tool = match registry.find(&name) {
        Some(t) => t.clone(),
        None => {
            return ToolCallResponse {
                id,
                name: name.clone(),
                result: Err(ToolError::not_found(&name)),
            };
        },
    };

    let input: serde_json::Value = match serde_json::from_str(&request.input) {
        Ok(v) => v,
        Err(e) => {
            return ToolCallResponse {
                id,
                name,
                result: Err(ToolError::invalid_input(e.to_string())),
            };
        },
    };

    if let Err(e) = tool.validate(&input, ctx).await {
        return ToolCallResponse {
            id,
            name,
            result: Err(e),
        };
    }

    // ── 重试循环 ──
    let mut last_error = None;
    for attempt in 0..=max_retries {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(
                retry_delay_ms * attempt as u64,
            ))
            .await;
        }

        let future = tool.call(input.clone(), ctx);
        let result = match tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            future,
        )
        .await
        {
            Ok(call_result) => match call_result {
                Ok(r) => {
                    return ToolCallResponse {
                        id,
                        name,
                        result: Ok(r),
                    }
                },
                Err(e) => {
                    if matches!(
                        e.kind,
                        crate::ToolErrorKind::Timeout | crate::ToolErrorKind::ExecutionFailed
                    ) && attempt < max_retries
                    {
                        last_error = Some(e);
                        continue;
                    }
                    Err(e)
                },
            },
            Err(_) => {
                if attempt < max_retries {
                    last_error = Some(ToolError {
                        error_code: format!("tool.{}.timeout", name),
                        message: format!(
                            "工具 '{}' 执行超时（{} 秒），重试 {}/{}",
                            name,
                            timeout_secs,
                            attempt + 1,
                            max_retries
                        ),
                        kind: crate::ToolErrorKind::Timeout,
                    });
                    continue;
                }
                Err(ToolError {
                    error_code: format!("tool.{}.timeout", name),
                    message: format!(
                        "工具 '{}' 执行超时（{} 秒），已达最大重试次数",
                        name, timeout_secs
                    ),
                    kind: crate::ToolErrorKind::Timeout,
                })
            },
        };

        match result {
            Ok(r) => {
                return ToolCallResponse {
                    id,
                    name,
                    result: Ok(r),
                }
            },
            Err(e) => {
                return ToolCallResponse {
                    id,
                    name,
                    result: Err(e),
                }
            },
        }
    }

    // 不应到达这里
    ToolCallResponse {
        id,
        name,
        result: Err(last_error
            .unwrap_or_else(|| ToolError::execution_failed_for("Orchestrator", "未知错误"))),
    }
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new(OrchestratorConfig::default())
    }
}
