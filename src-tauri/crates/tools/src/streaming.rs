//! 流式工具执行器 - 支持边执行边推送结果

use crate::registry::ToolRegistry;
use crate::{ToolContext, ToolError, ToolResult};
use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};

pub struct StreamingToolExecutor {
    semaphore: Arc<Semaphore>,
    result_tx: mpsc::Sender<StreamingToolResult>,
    result_rx: mpsc::Receiver<StreamingToolResult>,
    pending: Vec<tokio::task::JoinHandle<()>>,
}

#[derive(Debug, Clone)]
pub struct StreamingToolResult {
    pub id: String,
    pub name: String,
    pub result: Result<ToolResult, ToolError>,
}

impl StreamingToolExecutor {
    pub fn new(max_concurrency: usize) -> Self {
        let (tx, rx) = mpsc::channel(max_concurrency * 2);
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrency)),
            result_tx: tx,
            result_rx: rx,
            pending: Vec::new(),
        }
    }

    /// 添加待执行工具，结果通过通道自动推送
    pub fn add_tool(
        &mut self,
        id: String,
        name: String,
        input: serde_json::Value,
        registry: Arc<ToolRegistry>,
        ctx: ToolContext,
    ) {
        let sem = self.semaphore.clone();
        let tx = self.result_tx.clone();

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await;
            let tool = match registry.find(&name) {
                Some(t) => t.clone(),
                None => {
                    let _ = tx
                        .send(StreamingToolResult {
                            id,
                            name: name.clone(),
                            result: Err(ToolError::not_found(&name)),
                        })
                        .await;
                    return;
                },
            };

            let result = tool.call(input, &ctx).await;

            let _ = tx.send(StreamingToolResult { id, name, result }).await;
        });

        self.pending.push(handle);
    }

    /// 获取下一个完成的结果（阻塞直到有结果或通道关闭）
    pub async fn recv(&mut self) -> Option<StreamingToolResult> {
        self.result_rx.recv().await
    }

    /// 尝试非阻塞获取结果
    pub fn try_recv(&mut self) -> Option<StreamingToolResult> {
        self.result_rx.try_recv().ok()
    }

    /// 取消指定工具（通过 ID）
    #[allow(unused)]
    pub fn cancel(&mut self, _id: &str) {
        // 中止 handle
        self.pending.retain(|h| !h.is_finished());
    }

    /// 等待所有工具完成
    pub async fn wait_all(mut self) -> Vec<StreamingToolResult> {
        drop(self.result_tx);
        let mut results = Vec::new();
        while let Some(r) = self.result_rx.recv().await {
            results.push(r);
        }
        for h in self.pending {
            let _ = h.await;
        }
        results
    }

    pub fn pending_count(&self) -> usize {
        self.pending.iter().filter(|h| !h.is_finished()).count()
    }
}

impl Default for StreamingToolExecutor {
    fn default() -> Self {
        Self::new(10)
    }
}
