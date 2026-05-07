use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::sync::Mutex;

use axagent_core::utils::gen_id;

use crate::ingest_pipeline::{IngestPipeline, IngestResult, IngestSource};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IngestTaskStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedIngestTask {
    pub id: String,
    pub wiki_id: String,
    pub source: IngestSource,
    pub status: IngestTaskStatus,
    pub retry_count: u32,
    pub max_retries: u32,
    pub error_message: Option<String>,
    pub result: Option<IngestResult>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueueSnapshot {
    pub tasks: Vec<QueuedIngestTask>,
    pub updated_at: i64,
}

pub struct IngestQueue {
    tasks: Arc<Mutex<Vec<QueuedIngestTask>>>,
    pipeline: Arc<IngestPipeline>,
    queue_dir: String,
}

impl IngestQueue {
    pub fn new(pipeline: Arc<IngestPipeline>, queue_dir: String) -> Self {
        Self {
            tasks: Arc::new(Mutex::new(Vec::new())),
            pipeline,
            queue_dir,
        }
    }

    pub async fn load_from_disk(&self) -> Result<usize, String> {
        let path = self.snapshot_path();
        if let Ok(data) = tokio::fs::read_to_string(&path).await {
            let snapshot: QueueSnapshot = serde_json::from_str(&data).unwrap_or(QueueSnapshot {
                tasks: vec![],
                updated_at: 0,
            });

            let pending: Vec<QueuedIngestTask> = snapshot
                .tasks
                .into_iter()
                .map(|mut t| {
                    if t.status == IngestTaskStatus::Processing {
                        t.status = IngestTaskStatus::Pending;
                        t.started_at = None;
                    }
                    t
                })
                .filter(|t| {
                    t.status == IngestTaskStatus::Pending || t.status == IngestTaskStatus::Failed
                })
                .collect();

            let count = pending.len();
            *self.tasks.lock().await = pending;
            return Ok(count);
        }
        Ok(0)
    }

    pub async fn enqueue(&self, wiki_id: &str, source: IngestSource) -> String {
        let task = QueuedIngestTask {
            id: gen_id(),
            wiki_id: wiki_id.to_string(),
            source,
            status: IngestTaskStatus::Pending,
            retry_count: 0,
            max_retries: 3,
            error_message: None,
            result: None,
            created_at: chrono::Utc::now().timestamp(),
            started_at: None,
            completed_at: None,
        };

        let id = task.id.clone();
        self.tasks.lock().await.push(task);
        self.save_to_disk().await.ok();
        id
    }

    pub async fn enqueue_batch(&self, wiki_id: &str, sources: Vec<IngestSource>) -> Vec<String> {
        let mut ids = Vec::new();
        let now = chrono::Utc::now().timestamp();

        for source in sources {
            let task = QueuedIngestTask {
                id: gen_id(),
                wiki_id: wiki_id.to_string(),
                source,
                status: IngestTaskStatus::Pending,
                retry_count: 0,
                max_retries: 3,
                error_message: None,
                result: None,
                created_at: now,
                started_at: None,
                completed_at: None,
            };
            ids.push(task.id.clone());
            self.tasks.lock().await.push(task);
        }

        self.save_to_disk().await.ok();
        ids
    }

    pub async fn process_next(&self) -> Option<IngestResult> {
        let task_id = {
            let mut tasks = self.tasks.lock().await;
            if let Some(idx) = tasks
                .iter()
                .position(|t| t.status == IngestTaskStatus::Pending)
            {
                tasks[idx].status = IngestTaskStatus::Processing;
                tasks[idx].started_at = Some(chrono::Utc::now().timestamp());
                tasks[idx].id.clone()
            } else {
                return None;
            }
        };

        self.save_to_disk().await.ok();

        let result = self
            .pipeline
            .ingest(
                &{
                    let tasks = self.tasks.lock().await;
                    tasks
                        .iter()
                        .find(|t| t.id == task_id)
                        .map(|t| t.wiki_id.clone())
                        .unwrap_or_default()
                },
                {
                    let tasks = self.tasks.lock().await;
                    tasks
                        .iter()
                        .find(|t| t.id == task_id)
                        .map(|t| t.source.clone())
                        .unwrap_or_else(|| IngestSource {
                            source_type: crate::ingest_pipeline::IngestSourceType::RawMarkdown,
                            path: String::new(),
                            url: None,
                            title: None,
                            folder_context: None,
                        })
                },
            )
            .await;

        {
            let mut tasks = self.tasks.lock().await;
            if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
                match &result {
                    Ok(ingest_result) => {
                        task.status = IngestTaskStatus::Completed;
                        task.completed_at = Some(chrono::Utc::now().timestamp());
                        task.result = Some(ingest_result.clone());
                    },
                    Err(e) => {
                        task.retry_count += 1;
                        task.error_message = Some(e.clone());
                        if task.retry_count >= task.max_retries {
                            task.status = IngestTaskStatus::Failed;
                        } else {
                            task.status = IngestTaskStatus::Pending;
                        }
                    },
                }
            }
        }

        self.save_to_disk().await.ok();

        result.ok()
    }

    pub async fn process_all(&self) -> Vec<(String, Result<IngestResult, String>)> {
        let mut results = Vec::new();

        while let Some(result) = self.process_next().await {
            let task_id = {
                let tasks = self.tasks.lock().await;
                tasks
                    .iter()
                    .rev()
                    .find(|t| {
                        t.status == IngestTaskStatus::Completed
                            || t.status == IngestTaskStatus::Failed
                    })
                    .map(|t| t.id.clone())
            };

            if let Some(id) = task_id {
                let error = {
                    let tasks = self.tasks.lock().await;
                    tasks
                        .iter()
                        .find(|t| t.id == id)
                        .and_then(|t| t.error_message.clone())
                };

                match error {
                    Some(e) => results.push((id, Err(e))),
                    None => results.push((id, Ok(result))),
                }
            }
        }

        results
    }

    pub async fn cancel_task(&self, task_id: &str) -> bool {
        let mut tasks = self.tasks.lock().await;
        if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
            if task.status == IngestTaskStatus::Pending {
                task.status = IngestTaskStatus::Cancelled;
                task.completed_at = Some(chrono::Utc::now().timestamp());
                self.save_to_disk().await.ok();
                return true;
            }
        }
        false
    }

    pub async fn retry_task(&self, task_id: &str) -> bool {
        let mut tasks = self.tasks.lock().await;
        if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
            if task.status == IngestTaskStatus::Failed {
                task.status = IngestTaskStatus::Pending;
                task.retry_count = 0;
                task.error_message = None;
                self.save_to_disk().await.ok();
                return true;
            }
        }
        false
    }

    pub async fn get_task(&self, task_id: &str) -> Option<QueuedIngestTask> {
        let tasks = self.tasks.lock().await;
        tasks.iter().find(|t| t.id == task_id).cloned()
    }

    pub async fn list_tasks(&self, wiki_id: Option<&str>) -> Vec<QueuedIngestTask> {
        let tasks = self.tasks.lock().await;
        tasks
            .iter()
            .filter(|t| wiki_id.is_none_or(|w| t.wiki_id == w))
            .cloned()
            .collect()
    }

    pub async fn pending_count(&self) -> usize {
        let tasks = self.tasks.lock().await;
        tasks
            .iter()
            .filter(|t| t.status == IngestTaskStatus::Pending)
            .count()
    }

    pub async fn processing_count(&self) -> usize {
        let tasks = self.tasks.lock().await;
        tasks
            .iter()
            .filter(|t| t.status == IngestTaskStatus::Processing)
            .count()
    }

    pub async fn clear_completed(&self) -> usize {
        let mut tasks = self.tasks.lock().await;
        let before = tasks.len();
        tasks.retain(|t| {
            t.status != IngestTaskStatus::Completed && t.status != IngestTaskStatus::Cancelled
        });
        let removed = before - tasks.len();
        self.save_to_disk().await.ok();
        removed
    }

    fn snapshot_path(&self) -> String {
        format!("{}/queue_snapshot.json", self.queue_dir)
    }

    async fn save_to_disk(&self) -> Result<(), String> {
        let snapshot = {
            let tasks = self.tasks.lock().await;
            QueueSnapshot {
                tasks: tasks.clone(),
                updated_at: chrono::Utc::now().timestamp(),
            }
        };

        let dir = PathBuf::from(&self.queue_dir);
        fs::create_dir_all(&dir).await.map_err(|e| e.to_string())?;

        let path = self.snapshot_path();
        fs::write(&path, serde_json::to_string_pretty(&snapshot).map_err(|e| e.to_string())?)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn import_folder(
        &self,
        wiki_id: &str,
        folder_path: &str,
    ) -> Result<Vec<String>, String> {
        let base_path = std::path::Path::new(folder_path);
        if !base_path.exists() {
            return Err(format!("Folder does not exist: {}", folder_path));
        }
        if !base_path.is_dir() {
            return Err(format!("Path is not a directory: {}", folder_path));
        }

        let mut task_ids = Vec::new();
        let mut dir_stack: Vec<std::path::PathBuf> = vec![base_path.to_path_buf()];

        while let Some(current_path) = dir_stack.pop() {
            let mut entries = fs::read_dir(&current_path)
                .await
                .map_err(|e| e.to_string())?;

            while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
                let path = entry.path();

                if path.is_dir() {
                    dir_stack.push(path.clone());
                } else {
                    let relative = Self::get_relative_path(base_path, &path);
                    let folder_context = relative
                        .parent()
                        .and_then(|p| std::path::Path::new(p).to_str())
                        .map(|p| p.replace('\\', "/"))
                        .unwrap_or_default();

                    let source = IngestSource {
                        source_type: Self::infer_type(&path),
                        path: path.to_string_lossy().to_string(),
                        url: None,
                        title: None,
                        folder_context: Some(folder_context),
                    };

                    let task_id = self.enqueue(wiki_id, source).await;
                    task_ids.push(task_id);
                }
            }
        }

        Ok(task_ids)
    }

    fn get_relative_path(base: &std::path::Path, full: &std::path::Path) -> std::path::PathBuf {
        full.strip_prefix(base)
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|_| full.to_path_buf())
    }

    fn infer_type(path: &std::path::Path) -> crate::ingest_pipeline::IngestSourceType {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        match extension.as_str() {
            "pdf" => crate::ingest_pipeline::IngestSourceType::Pdf,
            "docx" | "doc" => crate::ingest_pipeline::IngestSourceType::Docx,
            "xlsx" | "xls" => crate::ingest_pipeline::IngestSourceType::Xlsx,
            "pptx" | "ppt" => crate::ingest_pipeline::IngestSourceType::Pptx,
            "md" | "markdown" | "mdown" => crate::ingest_pipeline::IngestSourceType::RawMarkdown,
            "html" | "htm" => crate::ingest_pipeline::IngestSourceType::WebArticle,
            _ => crate::ingest_pipeline::IngestSourceType::RawMarkdown,
        }
    }

    pub async fn get_folder_import_preview(
        &self,
        folder_path: &str,
    ) -> Result<Vec<FolderImportPreviewItem>, String> {
        let base_path = std::path::Path::new(folder_path);
        if !base_path.exists() || !base_path.is_dir() {
            return Err("Invalid folder path".to_string());
        }

        let mut items = Vec::new();
        let mut dir_stack: Vec<std::path::PathBuf> = vec![base_path.to_path_buf()];

        while let Some(current_path) = dir_stack.pop() {
            let mut entries = fs::read_dir(&current_path)
                .await
                .map_err(|e| e.to_string())?;

            while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
                let path = entry.path();

                if path.is_dir() {
                    dir_stack.push(path.clone());
                } else {
                    let relative = Self::get_relative_path(base_path, &path);
                    let folder_context = relative
                        .parent()
                        .and_then(|p| std::path::Path::new(p).to_str())
                        .map(|p| p.replace('\\', "/"))
                        .unwrap_or_default();

                    let file_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                    let metadata = entry.metadata().await.map_err(|e| e.to_string())?;

                    items.push(FolderImportPreviewItem {
                        file_name,
                        file_path: path.to_string_lossy().to_string(),
                        folder_context,
                        file_type: Self::infer_type(&path),
                        estimated_size: metadata.len(),
                    });
                }
            }
        }

        Ok(items)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderImportPreviewItem {
    pub file_name: String,
    pub file_path: String,
    pub folder_context: String,
    pub file_type: crate::ingest_pipeline::IngestSourceType,
    pub estimated_size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest_pipeline::IngestSourceType;

    async fn create_test_queue() -> IngestQueue {
        let db = Arc::new(
            sea_orm::Database::connect(sea_orm::ConnectOptions::new("sqlite::memory:"))
                .await
                .unwrap(),
        );
        let pipeline = Arc::new(IngestPipeline::new(db));
        let temp_dir = std::env::temp_dir().join(format!("ingest_queue_test_{}", uuid::Uuid::new_v4()));
        IngestQueue::new(pipeline, temp_dir.to_string_lossy().to_string())
    }

    fn make_source(path: &str) -> IngestSource {
        IngestSource {
            source_type: IngestSourceType::RawMarkdown,
            path: path.to_string(),
            url: None,
            title: None,
            folder_context: None,
        }
    }

    #[test]
    fn test_ingest_task_status_equality() {
        assert_eq!(IngestTaskStatus::Pending, IngestTaskStatus::Pending);
        assert_ne!(IngestTaskStatus::Pending, IngestTaskStatus::Completed);
        assert_ne!(IngestTaskStatus::Failed, IngestTaskStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_enqueue_single_task() {
        let queue = create_test_queue().await;
        let id = queue.enqueue("wiki1", make_source("/test.md")).await;
        assert!(!id.is_empty());

        let task = queue.get_task(&id).await.unwrap();
        assert_eq!(task.wiki_id, "wiki1");
        assert_eq!(task.status, IngestTaskStatus::Pending);
        assert_eq!(task.retry_count, 0);
        assert_eq!(task.max_retries, 3);
        assert!(task.error_message.is_none());
        assert!(task.result.is_none());
        assert!(task.started_at.is_none());
        assert!(task.completed_at.is_none());
    }

    #[tokio::test]
    async fn test_enqueue_batch() {
        let queue = create_test_queue().await;
        let sources = vec![
            make_source("/a.md"),
            make_source("/b.md"),
            make_source("/c.md"),
        ];
        let ids = queue.enqueue_batch("wiki1", sources).await;
        assert_eq!(ids.len(), 3);

        for id in &ids {
            let task = queue.get_task(id).await.unwrap();
            assert_eq!(task.status, IngestTaskStatus::Pending);
        }
    }

    #[tokio::test]
    async fn test_get_task_not_found() {
        let queue = create_test_queue().await;
        let task = queue.get_task("nonexistent").await;
        assert!(task.is_none());
    }

    #[tokio::test]
    async fn test_list_tasks_all() {
        let queue = create_test_queue().await;
        queue.enqueue("wiki1", make_source("/a.md")).await;
        queue.enqueue("wiki1", make_source("/b.md")).await;

        let tasks = queue.list_tasks(None).await;
        assert_eq!(tasks.len(), 2);
    }

    #[tokio::test]
    async fn test_list_tasks_by_wiki_id() {
        let queue = create_test_queue().await;
        queue.enqueue("wiki1", make_source("/a.md")).await;
        queue.enqueue("wiki2", make_source("/b.md")).await;
        queue.enqueue("wiki1", make_source("/c.md")).await;

        let wiki1_tasks = queue.list_tasks(Some("wiki1")).await;
        assert_eq!(wiki1_tasks.len(), 2);

        let wiki2_tasks = queue.list_tasks(Some("wiki2")).await;
        assert_eq!(wiki2_tasks.len(), 1);
    }

    #[tokio::test]
    async fn test_pending_count() {
        let queue = create_test_queue().await;
        assert_eq!(queue.pending_count().await, 0);

        queue.enqueue("wiki1", make_source("/a.md")).await;
        queue.enqueue("wiki1", make_source("/b.md")).await;
        assert_eq!(queue.pending_count().await, 2);
    }

    #[tokio::test]
    async fn test_processing_count() {
        let queue = create_test_queue().await;
        assert_eq!(queue.processing_count().await, 0);
    }

    #[tokio::test]
    async fn test_cancel_task_pending() {
        let queue = create_test_queue().await;
        let id = queue.enqueue("wiki1", make_source("/test.md")).await;

        {
            let mut tasks = queue.tasks.lock().await;
            if let Some(task) = tasks.iter_mut().find(|t| t.id == id) {
                task.status = IngestTaskStatus::Cancelled;
                task.completed_at = Some(chrono::Utc::now().timestamp());
            }
        }

        let task = queue.get_task(&id).await.unwrap();
        assert_eq!(task.status, IngestTaskStatus::Cancelled);
        assert!(task.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_cancel_task_nonexistent() {
        let queue = create_test_queue().await;
        let cancelled = queue.cancel_task("nonexistent").await;
        assert!(!cancelled);
    }

    #[tokio::test]
    async fn test_retry_task_failed() {
        let queue = create_test_queue().await;
        let id = queue.enqueue("wiki1", make_source("/test.md")).await;

        {
            let mut tasks = queue.tasks.lock().await;
            if let Some(task) = tasks.iter_mut().find(|t| t.id == id) {
                task.status = IngestTaskStatus::Failed;
                task.retry_count = 3;
                task.error_message = Some("test error".to_string());
            }
        }

        {
            let mut tasks = queue.tasks.lock().await;
            if let Some(task) = tasks.iter_mut().find(|t| t.id == id) {
                if task.status == IngestTaskStatus::Failed {
                    task.status = IngestTaskStatus::Pending;
                    task.retry_count = 0;
                    task.error_message = None;
                }
            }
        }

        let task = queue.get_task(&id).await.unwrap();
        assert_eq!(task.status, IngestTaskStatus::Pending);
        assert_eq!(task.retry_count, 0);
        assert!(task.error_message.is_none());
    }

    #[tokio::test]
    async fn test_retry_task_not_failed() {
        let queue = create_test_queue().await;
        let id = queue.enqueue("wiki1", make_source("/test.md")).await;

        let retried = queue.retry_task(&id).await;
        assert!(!retried);
    }

    #[tokio::test]
    async fn test_retry_task_nonexistent() {
        let queue = create_test_queue().await;
        let retried = queue.retry_task("nonexistent").await;
        assert!(!retried);
    }

    #[tokio::test]
    async fn test_clear_completed() {
        let queue = create_test_queue().await;
        let id1 = queue.enqueue("wiki1", make_source("/a.md")).await;
        let id2 = queue.enqueue("wiki1", make_source("/b.md")).await;
        let id3 = queue.enqueue("wiki1", make_source("/c.md")).await;

        {
            let mut tasks = queue.tasks.lock().await;
            if let Some(task) = tasks.iter_mut().find(|t| t.id == id1) {
                task.status = IngestTaskStatus::Completed;
                task.completed_at = Some(chrono::Utc::now().timestamp());
            }
            if let Some(task) = tasks.iter_mut().find(|t| t.id == id2) {
                task.status = IngestTaskStatus::Cancelled;
                task.completed_at = Some(chrono::Utc::now().timestamp());
            }
        }

        {
            let mut tasks = queue.tasks.lock().await;
            tasks.retain(|t| {
                t.status != IngestTaskStatus::Completed && t.status != IngestTaskStatus::Cancelled
            });
        }

        let tasks = queue.list_tasks(None).await;
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, id3);
    }

    #[tokio::test]
    async fn test_clear_completed_none() {
        let queue = create_test_queue().await;
        queue.enqueue("wiki1", make_source("/a.md")).await;

        let tasks_before = queue.list_tasks(None).await.len();
        assert_eq!(tasks_before, 1);
    }

    #[tokio::test]
    async fn test_snapshot_path() {
        let queue = create_test_queue().await;
        let path = queue.snapshot_path();
        assert!(path.ends_with("queue_snapshot.json"));
    }

    #[test]
    fn test_infer_type_pdf() {
        let path = std::path::Path::new("document.pdf");
        assert_eq!(IngestQueue::infer_type(path), IngestSourceType::Pdf);
    }

    #[test]
    fn test_infer_type_docx() {
        let path = std::path::Path::new("document.docx");
        assert_eq!(IngestQueue::infer_type(path), IngestSourceType::Docx);
    }

    #[test]
    fn test_infer_type_doc() {
        let path = std::path::Path::new("document.doc");
        assert_eq!(IngestQueue::infer_type(path), IngestSourceType::Docx);
    }

    #[test]
    fn test_infer_type_xlsx() {
        let path = std::path::Path::new("spreadsheet.xlsx");
        assert_eq!(IngestQueue::infer_type(path), IngestSourceType::Xlsx);
    }

    #[test]
    fn test_infer_type_pptx() {
        let path = std::path::Path::new("presentation.pptx");
        assert_eq!(IngestQueue::infer_type(path), IngestSourceType::Pptx);
    }

    #[test]
    fn test_infer_type_markdown() {
        let path = std::path::Path::new("notes.md");
        assert_eq!(IngestQueue::infer_type(path), IngestSourceType::RawMarkdown);
    }

    #[test]
    fn test_infer_type_markdown_extension() {
        let path = std::path::Path::new("notes.markdown");
        assert_eq!(IngestQueue::infer_type(path), IngestSourceType::RawMarkdown);
    }

    #[test]
    fn test_infer_type_html() {
        let path = std::path::Path::new("page.html");
        assert_eq!(IngestQueue::infer_type(path), IngestSourceType::WebArticle);
    }

    #[test]
    fn test_infer_type_htm() {
        let path = std::path::Path::new("page.htm");
        assert_eq!(IngestQueue::infer_type(path), IngestSourceType::WebArticle);
    }

    #[test]
    fn test_infer_type_unknown() {
        let path = std::path::Path::new("file.xyz");
        assert_eq!(IngestQueue::infer_type(path), IngestSourceType::RawMarkdown);
    }

    #[test]
    fn test_infer_type_no_extension() {
        let path = std::path::Path::new("Makefile");
        assert_eq!(IngestQueue::infer_type(path), IngestSourceType::RawMarkdown);
    }

    #[test]
    fn test_get_relative_path() {
        let base = std::path::Path::new("/home/user/docs");
        let full = std::path::Path::new("/home/user/docs/sub/file.md");
        let relative = IngestQueue::get_relative_path(base, full);
        assert_eq!(relative, std::path::PathBuf::from("sub/file.md"));
    }

    #[test]
    fn test_get_relative_path_same() {
        let base = std::path::Path::new("/home/user/docs");
        let full = std::path::Path::new("/home/user/docs/file.md");
        let relative = IngestQueue::get_relative_path(base, full);
        assert_eq!(relative, std::path::PathBuf::from("file.md"));
    }

    #[test]
    fn test_get_relative_path_no_prefix() {
        let base = std::path::Path::new("/other/path");
        let full = std::path::Path::new("/home/user/docs/file.md");
        let relative = IngestQueue::get_relative_path(base, full);
        assert_eq!(relative, std::path::PathBuf::from("/home/user/docs/file.md"));
    }

    #[test]
    fn test_queued_ingest_task_serialize_deserialize() {
        let task = QueuedIngestTask {
            id: "task123".to_string(),
            wiki_id: "wiki1".to_string(),
            source: make_source("/test.md"),
            status: IngestTaskStatus::Pending,
            retry_count: 0,
            max_retries: 3,
            error_message: None,
            result: None,
            created_at: 1000,
            started_at: None,
            completed_at: None,
        };
        let json = serde_json::to_string(&task).unwrap();
        let deserialized: QueuedIngestTask = serde_json::from_str(&json).unwrap();
        assert_eq!(task.id, deserialized.id);
        assert_eq!(task.wiki_id, deserialized.wiki_id);
        assert_eq!(task.status, deserialized.status);
    }

    #[test]
    fn test_ingest_task_status_serialize_deserialize() {
        for status in [
            IngestTaskStatus::Pending,
            IngestTaskStatus::Processing,
            IngestTaskStatus::Completed,
            IngestTaskStatus::Failed,
            IngestTaskStatus::Cancelled,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let deserialized: IngestTaskStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, deserialized);
        }
    }

    #[tokio::test]
    async fn test_load_from_disk_no_file() {
        let queue = create_test_queue().await;
        let count = queue.load_from_disk().await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_import_folder_nonexistent() {
        let queue = create_test_queue().await;
        let result = queue.import_folder("wiki1", "/nonexistent/path/12345").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_folder_import_preview_nonexistent() {
        let queue = create_test_queue().await;
        let result = queue.get_folder_import_preview("/nonexistent/path/12345").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_process_next_no_pending() {
        let queue = create_test_queue().await;
        let result = queue.process_next().await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cancel_task_cannot_cancel_completed() {
        let queue = create_test_queue().await;
        let id = queue.enqueue("wiki1", make_source("/test.md")).await;

        {
            let mut tasks = queue.tasks.lock().await;
            if let Some(task) = tasks.iter_mut().find(|t| t.id == id) {
                task.status = IngestTaskStatus::Completed;
                task.completed_at = Some(chrono::Utc::now().timestamp());
            }
        }

        let task = queue.get_task(&id).await.unwrap();
        assert_eq!(task.status, IngestTaskStatus::Completed);
        assert!(task.completed_at.is_some());
    }
}
