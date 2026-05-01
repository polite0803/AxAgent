use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::backend_trait::{
    BackendType, SpawnConfig, TerminalBackend, TerminalExit, TerminalOutput,
};

#[allow(dead_code)]
pub struct DockerBackend {
    socket_path: String,
    connected: Arc<RwLock<bool>>,
    sessions: Arc<RwLock<HashMap<String, String>>>,
}

impl DockerBackend {
    pub fn new(socket_path: Option<String>) -> Self {
        Self {
            socket_path: socket_path.unwrap_or_else(|| {
                #[cfg(windows)]
                {
                    "npipe:////./pipe/docker_engine".to_string()
                }
                #[cfg(not(windows))]
                {
                    "unix:///var/run/docker.sock".to_string()
                }
            }),
            connected: Arc::new(RwLock::new(false)),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn docker_api_request(
        &self,
        method: &str,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> anyhow::Result<serde_json::Value> {
        let base_url = "http://localhost";
        let url = format!("{}{}", base_url, path);
        let client = reqwest::Client::new();

        let mut req = match method {
            "GET" => client.get(&url),
            "POST" => client.post(&url),
            "DELETE" => client.delete(&url),
            _ => anyhow::bail!("Unsupported HTTP method: {}", method),
        };

        req = req.header("Content-Type", "application/json");

        if let Some(body_val) = body {
            req = req.json(&body_val);
        }

        let resp = req.send().await?;
        let status = resp.status();

        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Docker API error ({}): {}", status, text);
        }

        if status == reqwest::StatusCode::NO_CONTENT || method == "DELETE" {
            return Ok(serde_json::json!({}));
        }

        let json: serde_json::Value = resp.json().await?;
        Ok(json)
    }
}

#[async_trait]
impl TerminalBackend for DockerBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::Docker
    }

    async fn connect(&self) -> anyhow::Result<()> {
        match self.docker_api_request("GET", "/_ping", None).await {
            Ok(_) => {
                let mut c = self.connected.write().await;
                *c = true;
                Ok(())
            },
            Err(e) => {
                anyhow::bail!("Docker daemon not reachable: {}", e);
            },
        }
    }

    async fn disconnect(&self) -> anyhow::Result<()> {
        let sessions = self.sessions.read().await;
        for container_id in sessions.values() {
            let _ = self
                .docker_api_request(
                    "DELETE",
                    &format!("/containers/{}?force=true", container_id),
                    None,
                )
                .await;
        }
        drop(sessions);

        let mut c = self.connected.write().await;
        *c = false;
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    async fn spawn_session(&self, session_id: &str, config: SpawnConfig) -> anyhow::Result<()> {
        let shell_cmd = config.shell.unwrap_or_else(|| {
            #[cfg(windows)]
            {
                "cmd.exe".to_string()
            }
            #[cfg(not(windows))]
            {
                "/bin/sh".to_string()
            }
        });

        let create_body = serde_json::json!({
            "Image": "alpine:latest",
            "Cmd": [&shell_cmd],
            "Tty": true,
            "OpenStdin": true,
            "AttachStdin": true,
            "AttachStdout": true,
            "AttachStderr": true,
            "WorkingDir": config.cwd.unwrap_or_else(|| "/".to_string()),
            "Env": config.env.iter().map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<_>>(),
        });

        let resp = self
            .docker_api_request("POST", "/containers/create", Some(create_body))
            .await?;

        let container_id = resp["Id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No container ID in response"))?;

        self.docker_api_request("POST", &format!("/containers/{}/start", container_id), None)
            .await?;

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.to_string(), container_id.to_string());

        Ok(())
    }

    async fn write_to_session(&self, session_id: &str, _data: &[u8]) -> anyhow::Result<()> {
        let sessions = self.sessions.read().await;
        let container_id = sessions
            .get(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

        let text = String::from_utf8_lossy(_data);
        let body = serde_json::json!({
            "AttachStdin": true,
            "Cmd": ["sh", "-c", &text],
        });

        let exec_resp = self
            .docker_api_request(
                "POST",
                &format!("/containers/{}/exec", container_id),
                Some(body),
            )
            .await?;

        let exec_id = exec_resp["Id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No exec ID"))?;

        self.docker_api_request(
            "POST",
            &format!("/exec/{}/start", exec_id),
            Some(serde_json::json!({"Detach": false, "Tty": true})),
        )
        .await?;

        Ok(())
    }

    async fn resize_session(&self, session_id: &str, rows: u16, cols: u16) -> anyhow::Result<()> {
        let sessions = self.sessions.read().await;
        let container_id = sessions
            .get(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

        let body = serde_json::json!({
            "Height": rows,
            "Width": cols,
        });

        self.docker_api_request(
            "POST",
            &format!("/containers/{}/resize", container_id),
            Some(body),
        )
        .await?;

        Ok(())
    }

    async fn kill_session(&self, session_id: &str) -> anyhow::Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(container_id) = sessions.remove(session_id) {
            let _ = self
                .docker_api_request(
                    "DELETE",
                    &format!("/containers/{}?force=true", container_id),
                    None,
                )
                .await;
        }
        Ok(())
    }

    async fn read_output(&self, session_id: &str) -> anyhow::Result<Vec<TerminalOutput>> {
        let sessions = self.sessions.read().await;
        let container_id = sessions
            .get(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
        let cid = container_id.clone();
        drop(sessions);

        let resp = self
            .docker_api_request(
                "GET",
                &format!("/containers/{}/logs?stdout=true&stderr=true&tail=50", cid),
                None,
            )
            .await;

        let mut outputs = Vec::new();
        let now = chrono::Utc::now().timestamp_millis();

        if let Ok(json) = resp {
            if let Some(log_str) = json.as_str() {
                outputs.push(TerminalOutput {
                    session_id: session_id.to_string(),
                    data: log_str.to_string(),
                    timestamp: now,
                });
            }
        }

        Ok(outputs)
    }

    async fn wait_for_exit(&self, session_id: &str) -> anyhow::Result<TerminalExit> {
        let sessions = self.sessions.read().await;
        let container_id = sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
        drop(sessions);

        let mut attempts = 0;
        loop {
            if attempts > 120 {
                anyhow::bail!("Timeout waiting for container exit");
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            attempts += 1;

            let resp = self
                .docker_api_request("GET", &format!("/containers/{}/json", container_id), None)
                .await?;

            let status_str = resp["State"]["Status"].as_str().unwrap_or("unknown");
            if status_str == "exited" || status_str == "dead" {
                let exit_code = resp["State"]["ExitCode"].as_i64().map(|c| c as i32);

                return Ok(TerminalExit {
                    session_id: session_id.to_string(),
                    exit_code,
                    timestamp: chrono::Utc::now().timestamp_millis(),
                });
            }
        }
    }

    async fn list_sessions(&self) -> anyhow::Result<Vec<String>> {
        let sessions = self.sessions.read().await;
        Ok(sessions.keys().cloned().collect())
    }
}
