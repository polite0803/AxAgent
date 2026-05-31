use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use tracing::{info, warn};

use crate::PluginMcpServer;

const MCP_STARTUP_POLL_INTERVAL_MS: u64 = 100;
const MCP_STARTUP_TIMEOUT_SECS: u64 = 10;

struct RunningMcpProcess {
    child: Child,
    server_name: String,
}

impl fmt::Debug for RunningMcpProcess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RunningMcpProcess")
            .field("server_name", &self.server_name)
            .field("pid", &self.child.id())
            .finish()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum McpLaunchError {
    #[error("MCP server `{server}` failed to start: {source}")]
    SpawnFailed {
        server: String,
        source: std::io::Error,
    },
    #[error("MCP server `{0}` exited immediately after start")]
    ImmediateExit(String),
    #[error("MCP server `{0}` did not become healthy within startup timeout")]
    StartupTimeout(String),
}

pub struct McpLauncher {
    running: HashMap<String, Vec<RunningMcpProcess>>,
    startup_timeout: Duration,
}

impl fmt::Debug for McpLauncher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("McpLauncher")
            .field("plugin_count", &self.running.len())
            .field("total_servers", &self.running.values().map(|v| v.len()).sum::<usize>())
            .finish()
    }
}

impl McpLauncher {
    pub fn new() -> Self {
        Self {
            running: HashMap::new(),
            startup_timeout: Duration::from_secs(MCP_STARTUP_TIMEOUT_SECS),
        }
    }

    pub fn with_startup_timeout(mut self, timeout: Duration) -> Self {
        self.startup_timeout = timeout;
        self
    }

    pub fn start_plugin_mcps(
        &mut self,
        plugin_id: &str,
        servers: &[PluginMcpServer],
        plugin_root: &Path,
    ) -> Result<(), McpLaunchError> {
        let mut processes = Vec::new();
        for server in servers {
            let proc = self.spawn_server(plugin_id, server, plugin_root)?;
            processes.push(proc);
        }
        self.running.insert(plugin_id.to_string(), processes);
        Ok(())
    }

    pub fn stop_plugin_mcps(&mut self, plugin_id: &str) {
        if let Some(processes) = self.running.remove(plugin_id) {
            for mut proc in processes {
                info!("mcp: stopping server `{}` for plugin `{}`", proc.server_name, plugin_id);
                let _ = proc.child.kill();
                let _ = proc.child.wait();
            }
        }
    }

    pub fn healthcheck(&mut self) -> HashMap<String, Vec<ServerHealthStatusEntry>> {
        let mut result = HashMap::new();
        for (plugin_id, processes) in &mut self.running {
            let mut statuses = Vec::new();
            for proc in processes {
                let status = match proc.child.try_wait() {
                    Ok(None) => ServerHealthStatus::Running,
                    Ok(Some(status)) => ServerHealthStatus::Exited {
                        code: status.code(),
                    },
                    Err(e) => ServerHealthStatus::Error(e.to_string()),
                };
                statuses.push(ServerHealthStatusEntry {
                    server_name: proc.server_name.clone(),
                    pid: proc.child.id(),
                    status,
                });
            }
            result.insert(plugin_id.clone(), statuses);
        }
        result
    }

    fn spawn_server(
        &self,
        plugin_id: &str,
        server: &PluginMcpServer,
        plugin_root: &Path,
    ) -> Result<RunningMcpProcess, McpLaunchError> {
        info!(
            "mcp: starting server `{}` for plugin `{}`: {} {}",
            server.name,
            plugin_id,
            server.command,
            server.args.join(" ")
        );

        let mut cmd = Command::new(&server.command);
        cmd.args(&server.args);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.env("CLAWD_PLUGIN_ID", plugin_id);
        cmd.env("CLAWD_PLUGIN_ROOT", plugin_root);
        for (k, v) in &server.env {
            cmd.env(k, v);
        }
        if let Some(cwd) = &server.cwd {
            cmd.current_dir(cwd);
        } else {
            cmd.current_dir(plugin_root);
        }

        let mut child = cmd.spawn().map_err(|source| McpLaunchError::SpawnFailed {
            server: server.name.clone(),
            source,
        })?;

        let start = Instant::now();
        let poll_interval = Duration::from_millis(MCP_STARTUP_POLL_INTERVAL_MS);
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    warn!(
                        "mcp: server `{}` for plugin `{}` exited immediately with {:?}",
                        server.name, plugin_id, status
                    );
                    return Err(McpLaunchError::ImmediateExit(server.name.clone()));
                },
                Ok(None) => {
                    if start.elapsed() >= self.startup_timeout {
                        let stdin_pipe = child.stdin.take();
                        let is_responsive = if let Some(mut stdin_pipe) = stdin_pipe {
                            let init_request = serde_json::json!({
                                "jsonrpc": "2.0",
                                "id": 1,
                                "method": "initialize",
                                "params": {
                                    "protocolVersion": "2024-11-05",
                                    "capabilities": {},
                                    "clientInfo": {"name": "axagent", "version": "1.0.0"}
                                }
                            });
                            let request_str = format!("{}\n", init_request);
                            use std::io::Write;
                            match stdin_pipe.write_all(request_str.as_bytes()) {
                                Ok(()) => {
                                    drop(stdin_pipe);
                                    child.stdin = None;
                                    true
                                },
                                Err(_) => false,
                            }
                        } else {
                            true
                        };

                        if is_responsive {
                            info!(
                                "mcp: server `{}` for plugin `{}` started successfully (pid {})",
                                server.name,
                                plugin_id,
                                child.id()
                            );
                            Ok(RunningMcpProcess {
                                child,
                                server_name: server.name.clone(),
                            })
                        } else {
                            warn!(
                                "mcp: server `{}` for plugin `{}` failed health check (pid {})",
                                server.name,
                                plugin_id,
                                child.id()
                            );
                            let _ = child.kill();
                            let _ = child.wait();
                            Err(McpLaunchError::StartupTimeout(server.name.clone()))
                        }
                    }
                    std::thread::sleep(poll_interval);
                },
                Err(e) => {
                    return Err(McpLaunchError::SpawnFailed {
                        server: server.name.clone(),
                        source: e,
                    });
                },
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum ServerHealthStatus {
    Running,
    Exited { code: Option<i32> },
    Error(String),
}

#[derive(Debug, Clone)]
pub struct ServerHealthStatusEntry {
    pub server_name: String,
    pub pid: u32,
    pub status: ServerHealthStatus,
}

impl Drop for McpLauncher {
    fn drop(&mut self) {
        let plugin_ids: Vec<String> = self.running.keys().cloned().collect();
        for plugin_id in plugin_ids {
            self.stop_plugin_mcps(&plugin_id);
        }
    }
}

impl Default for McpLauncher {
    fn default() -> Self {
        Self::new()
    }
}
