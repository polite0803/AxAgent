use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use tracing::{info, warn};

use crate::PluginMcpServer;

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
}

pub struct McpLauncher {
    running: HashMap<String, Vec<RunningMcpProcess>>,
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
        }
    }

    /// 启动插件声明的所有 MCP 服务
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

    /// 停止插件所有 MCP 服务
    pub fn stop_plugin_mcps(&mut self, plugin_id: &str) {
        if let Some(processes) = self.running.remove(plugin_id) {
            for mut proc in processes {
                info!("mcp: stopping server `{}` for plugin `{}`", proc.server_name, plugin_id);
                let _ = proc.child.kill();
                let _ = proc.child.wait();
            }
        }
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

        // 等待短暂时间确认进程没有立即崩溃
        std::thread::sleep(Duration::from_secs(2));
        match child.try_wait() {
            Ok(Some(status)) => {
                warn!(
                    "mcp: server `{}` for plugin `{}` exited immediately with {:?}",
                    server.name, plugin_id, status
                );
                Err(McpLaunchError::ImmediateExit(server.name.clone()))
            },
            Ok(None) => {
                info!(
                    "mcp: server `{}` for plugin `{}` running (pid {})",
                    server.name,
                    plugin_id,
                    child.id()
                );
                Ok(RunningMcpProcess {
                    child,
                    server_name: server.name.clone(),
                })
            },
            Err(e) => Err(McpLaunchError::SpawnFailed {
                server: server.name.clone(),
                source: e,
            }),
        }
    }
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
