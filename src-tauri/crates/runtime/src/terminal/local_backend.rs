use async_trait::async_trait;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use super::backend_trait::{BackendType, SpawnConfig, TerminalBackend, TerminalExit, TerminalOutput};

#[allow(dead_code)]
struct LocalPtySession {
    writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send>,
    _master: Box<dyn MasterPty + Send>,
}

#[allow(dead_code)]
pub struct LocalBackend {
    connected: Arc<RwLock<bool>>,
    output_tx: mpsc::UnboundedSender<TerminalOutput>,
    output_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<TerminalOutput>>>,
    exit_tx: mpsc::UnboundedSender<TerminalExit>,
    exit_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<TerminalExit>>>,
    sessions: Arc<RwLock<HashMap<String, Arc<tokio::sync::Mutex<LocalPtySession>>>>>,
}

impl LocalBackend {
    pub fn new() -> Self {
        let (output_tx, output_rx) = mpsc::unbounded_channel();
        let (exit_tx, exit_rx) = mpsc::unbounded_channel();

        Self {
            connected: Arc::new(RwLock::new(true)),
            output_tx,
            output_rx: Arc::new(tokio::sync::Mutex::new(output_rx)),
            exit_tx,
            exit_rx: Arc::new(tokio::sync::Mutex::new(exit_rx)),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for LocalBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TerminalBackend for LocalBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::Local
    }

    async fn connect(&self) -> anyhow::Result<()> {
        let mut c = self.connected.write().await;
        *c = true;
        Ok(())
    }

    async fn disconnect(&self) -> anyhow::Result<()> {
        let sessions = self.sessions.read().await;
        for (_, session) in sessions.iter() {
            let mut s = session.lock().await;
            let _ = s.child.kill();
        }
        drop(sessions);
        let mut c = self.connected.write().await;
        *c = false;
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    async fn spawn_session(
        &self,
        session_id: &str,
        config: SpawnConfig,
    ) -> anyhow::Result<()> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: config.rows,
                cols: config.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| anyhow::anyhow!("Failed to open PTY: {}", e))?;

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
            .map_err(|e| anyhow::anyhow!("Failed to spawn PTY child: {}", e))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| anyhow::anyhow!("Failed to take PTY writer: {}", e))?;

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| anyhow::anyhow!("Failed to clone PTY reader: {}", e))?;

        let output_tx = self.output_tx.clone();
        let sid = session_id.to_string();

        // Reader thread
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let data = String::from_utf8_lossy(&buf[..n]).to_string();
                        let _ = output_tx.send(TerminalOutput {
                            session_id: sid.clone(),
                            data,
                            timestamp: chrono::Utc::now().timestamp_millis(),
                        });
                    }
                }
            }
        });

        let session = Arc::new(tokio::sync::Mutex::new(LocalPtySession {
            writer,
            child,
            _master: pair.master,
        }));

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.to_string(), session);

        Ok(())
    }

    async fn write_to_session(&self, session_id: &str, data: &[u8]) -> anyhow::Result<()> {
        let sessions = self.sessions.read().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

        let mut s = session.lock().await;
        s.writer
            .write_all(data)
            .map_err(|e| anyhow::anyhow!("Write failed: {}", e))?;
        s.writer
            .flush()
            .map_err(|e| anyhow::anyhow!("Flush failed: {}", e))?;

        Ok(())
    }

    async fn resize_session(
        &self,
        _session_id: &str,
        _rows: u16,
        _cols: u16,
    ) -> anyhow::Result<()> {
        // Local backend resize managed through the old PtySession API
        Ok(())
    }

    async fn kill_session(&self, session_id: &str) -> anyhow::Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.remove(session_id) {
            let mut s = session.lock().await;
            let _ = s.child.kill();
        }
        Ok(())
    }

    async fn read_output(&self, _session_id: &str) -> anyhow::Result<Vec<TerminalOutput>> {
        let mut rx = self.output_rx.lock().await;
        let mut outputs = Vec::new();
        while let Ok(event) = rx.try_recv() {
            outputs.push(event);
        }
        Ok(outputs)
    }

    async fn wait_for_exit(&self, _session_id: &str) -> anyhow::Result<TerminalExit> {
        let mut rx = self.exit_rx.lock().await;
        match rx.recv().await {
            Some(exit) => Ok(exit),
            None => Err(anyhow::anyhow!("Exit channel closed")),
        }
    }

    async fn list_sessions(&self) -> anyhow::Result<Vec<String>> {
        let sessions = self.sessions.read().await;
        Ok(sessions.keys().cloned().collect())
    }
}
