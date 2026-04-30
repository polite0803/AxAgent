use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtySessionConfig {
    pub shell: Option<String>,
    pub cwd: Option<String>,
    pub env: HashMap<String, String>,
    pub rows: u16,
    pub cols: u16,
}

impl Default for PtySessionConfig {
    fn default() -> Self {
        Self {
            shell: None,
            cwd: None,
            env: HashMap::new(),
            rows: 24,
            cols: 80,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtyOutputEvent {
    pub session_id: String,
    pub data: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtyExitEvent {
    pub session_id: String,
    pub exit_code: Option<i32>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PtySessionStatus {
    Starting,
    Running,
    Exited,
    Error,
}

struct PtySessionInner {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send>,
    status: PtySessionStatus,
}

pub struct PtySession {
    id: String,
    #[allow(dead_code)]
    config: PtySessionConfig,
    inner: Arc<tokio::sync::Mutex<Option<PtySessionInner>>>,
    output_tx: mpsc::UnboundedSender<PtyOutputEvent>,
    exit_tx: mpsc::UnboundedSender<PtyExitEvent>,
    status: Arc<RwLock<PtySessionStatus>>,
}

impl PtySession {
    pub fn new(
        id: String,
        config: PtySessionConfig,
        output_tx: mpsc::UnboundedSender<PtyOutputEvent>,
        exit_tx: mpsc::UnboundedSender<PtyExitEvent>,
    ) -> Result<Self, String> {
        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows: config.rows,
                cols: config.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Failed to open PTY: {}", e))?;

        let mut cmd = if let Some(ref shell) = config.shell {
            CommandBuilder::new(shell)
        } else {
            CommandBuilder::new_default_prog()
        };

        if let Some(ref cwd) = config.cwd {
            cmd.cwd(cwd);
        }

        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("Failed to spawn PTY child: {}", e))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| format!("Failed to take PTY writer: {}", e))?;

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("Failed to clone PTY reader: {}", e))?;

        drop(pair.slave);

        let inner = PtySessionInner {
            master: pair.master,
            writer,
            child,
            status: PtySessionStatus::Starting,
        };

        let session = Self {
            id,
            config,
            inner: Arc::new(tokio::sync::Mutex::new(Some(inner))),
            output_tx,
            exit_tx,
            status: Arc::new(RwLock::new(PtySessionStatus::Starting)),
        };

        session.start_reader(reader);

        Ok(session)
    }

    fn start_reader(&self, reader: Box<dyn Read + Send>) {
        let session_id = self.id.clone();
        let output_tx = self.output_tx.clone();
        let exit_tx = self.exit_tx.clone();
        let status = Arc::clone(&self.status);
        let inner = Arc::clone(&self.inner);

        let reader = Arc::new(std::sync::Mutex::new(reader));
        let buf = Arc::new(vec![0u8; 4096]);

        std::thread::spawn({
            let reader = Arc::clone(&reader);
            let buf = Arc::clone(&buf);
            let session_id = session_id.clone();
            let output_tx = output_tx.clone();
            move || {
                let mut buf = buf.as_ref().clone();
                loop {
                    let n = {
                        let mut reader = reader.lock().unwrap();
                        reader.read(&mut buf)
                    };

                    match n {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            let data = String::from_utf8_lossy(&buf[..n]).to_string();
                            let event = PtyOutputEvent {
                                session_id: session_id.clone(),
                                data,
                                timestamp: chrono::Utc::now().timestamp_millis(),
                            };
                            let _ = output_tx.send(event);
                        }
                    }
                }
            }
        });

        let status_clone = Arc::clone(&status);
        let inner_clone = Arc::clone(&inner);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                let s = status_clone.read().await;
                if *s == PtySessionStatus::Exited {
                    break;
                }
            }

            let exit_code = {
                let mut guard = inner_clone.lock().await;
                if let Some(ref mut inner) = *guard {
                    inner.status = PtySessionStatus::Exited;
                    match inner.child.try_wait() {
                        Ok(Some(status)) => Some(status.exit_code() as i32),
                        Ok(None) => None,
                        Err(_) => None,
                    }
                } else {
                    None
                }
            };

            {
                let mut s = status_clone.write().await;
                *s = PtySessionStatus::Exited;
            }

            let event = PtyExitEvent {
                session_id,
                exit_code,
                timestamp: chrono::Utc::now().timestamp_millis(),
            };
            let _ = exit_tx.send(event);
        });
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub async fn start(&self) -> Result<(), String> {
        {
            let mut guard = self.inner.lock().await;
            if let Some(ref mut inner) = *guard {
                inner.status = PtySessionStatus::Running;
            }
        }

        {
            let mut s = self.status.write().await;
            *s = PtySessionStatus::Running;
        }

        Ok(())
    }

    pub async fn write(&self, data: &[u8]) -> Result<(), String> {
        let mut guard = self.inner.lock().await;
        match guard.as_mut() {
            Some(inner) => {
                inner
                    .writer
                    .write_all(data)
                    .map_err(|e| format!("Failed to write to PTY: {}", e))?;
                inner
                    .writer
                    .flush()
                    .map_err(|e| format!("Failed to flush PTY: {}", e))?;
                Ok(())
            }
            None => Err("PTY session not available".to_string()),
        }
    }

    pub async fn write_str(&self, text: &str) -> Result<(), String> {
        self.write(text.as_bytes()).await
    }

    pub async fn resize(&self, rows: u16, cols: u16) -> Result<(), String> {
        let mut guard = self.inner.lock().await;
        match guard.as_mut() {
            Some(inner) => inner
                .master
                .resize(PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .map_err(|e| format!("Failed to resize PTY: {}", e)),
            None => Err("PTY session not available".to_string()),
        }
    }

    pub async fn kill(&self) -> Result<(), String> {
        let mut guard = self.inner.lock().await;
        match guard.as_mut() {
            Some(inner) => {
                inner
                    .child
                    .kill()
                    .map_err(|e| format!("Failed to kill PTY child: {}", e))?;
                inner.status = PtySessionStatus::Exited;
                let mut s = self.status.write().await;
                *s = PtySessionStatus::Exited;
                Ok(())
            }
            None => Err("PTY session not available".to_string()),
        }
    }

    pub async fn status(&self) -> PtySessionStatus {
        *self.status.read().await
    }

    pub async fn is_running(&self) -> bool {
        *self.status.read().await == PtySessionStatus::Running
    }
}

pub struct PtyManager {
    sessions: Arc<RwLock<HashMap<String, Arc<PtySession>>>>,
    output_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<PtyOutputEvent>>>,
    exit_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<PtyExitEvent>>>,
    output_tx: mpsc::UnboundedSender<PtyOutputEvent>,
    exit_tx: mpsc::UnboundedSender<PtyExitEvent>,
}

impl PtyManager {
    pub fn new() -> Self {
        let (output_tx, output_rx) = mpsc::unbounded_channel();
        let (exit_tx, exit_rx) = mpsc::unbounded_channel();

        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            output_rx: Arc::new(tokio::sync::Mutex::new(output_rx)),
            exit_rx: Arc::new(tokio::sync::Mutex::new(exit_rx)),
            output_tx,
            exit_tx,
        }
    }

    pub async fn create_session(
        &self,
        id: impl Into<String>,
        config: PtySessionConfig,
    ) -> Result<Arc<PtySession>, String> {
        let id = id.into();
        let session = Arc::new(PtySession::new(
            id.clone(),
            config,
            self.output_tx.clone(),
            self.exit_tx.clone(),
        )?);

        session.start().await?;

        let mut sessions = self.sessions.write().await;
        sessions.insert(id, Arc::clone(&session));

        Ok(session)
    }

    pub async fn get_session(&self, id: &str) -> Option<Arc<PtySession>> {
        let sessions = self.sessions.read().await;
        sessions.get(id).cloned()
    }

    pub async fn kill_session(&self, id: &str) -> Result<(), String> {
        let sessions = self.sessions.read().await;
        match sessions.get(id) {
            Some(session) => session.kill().await,
            None => Err(format!("PTY session '{}' not found", id)),
        }
    }

    pub async fn remove_session(&self, id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.remove(id) {
            if session.is_running().await {
                session.kill().await?;
            }
            Ok(())
        } else {
            Err(format!("PTY session '{}' not found", id))
        }
    }

    pub async fn list_sessions(&self) -> Vec<(String, PtySessionStatus)> {
        let sessions = self.sessions.read().await;
        let mut result = Vec::new();
        for (id, session) in sessions.iter() {
            result.push((id.clone(), session.status().await));
        }
        result
    }

    pub async fn try_recv_output(&self) -> Option<PtyOutputEvent> {
        let mut rx = self.output_rx.lock().await;
        rx.try_recv().ok()
    }

    pub async fn recv_output(&self) -> Option<PtyOutputEvent> {
        let mut rx = self.output_rx.lock().await;
        rx.recv().await
    }

    pub async fn try_recv_exit(&self) -> Option<PtyExitEvent> {
        let mut rx = self.exit_rx.lock().await;
        rx.try_recv().ok()
    }

    pub async fn kill_all(&self) {
        let mut sessions = self.sessions.write().await;
        for (_, session) in sessions.drain() {
            let _ = session.kill().await;
        }
    }
}

impl Default for PtyManager {
    fn default() -> Self {
        Self::new()
    }
}
