use std::ffi::OsStr;
#[cfg(not(windows))]
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;

use crate::{PluginError, PluginHooks, PluginRegistry};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookEvent {
    PreToolUse,
    PostToolUse,
    PostToolUseFailure,
}

impl HookEvent {
    fn as_str(self) -> &'static str {
        match self {
            Self::PreToolUse => "PreToolUse",
            Self::PostToolUse => "PostToolUse",
            Self::PostToolUseFailure => "PostToolUseFailure",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookRunResult {
    denied: bool,
    failed: bool,
    timed_out: bool,
    messages: Vec<String>,
}

impl HookRunResult {
    #[must_use]
    pub fn allow(messages: Vec<String>) -> Self {
        Self {
            denied: false,
            failed: false,
            timed_out: false,
            messages,
        }
    }

    #[must_use]
    pub fn is_denied(&self) -> bool {
        self.denied
    }

    #[must_use]
    pub fn is_failed(&self) -> bool {
        self.failed
    }

    #[must_use]
    pub fn is_timed_out(&self) -> bool {
        self.timed_out
    }

    #[must_use]
    pub fn messages(&self) -> &[String] {
        &self.messages
    }
}

const DEFAULT_HOOK_TIMEOUT_SECS: u64 = 30;

#[derive(Clone, Default)]
pub struct HookRunner {
    hooks: PluginHooks,
    timeout: Duration,
    in_process_hooks: Vec<Arc<dyn axagent_runtime_core::plugin_hooks::PluginHook>>,
}

impl std::fmt::Debug for HookRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookRunner")
            .field("hooks", &self.hooks)
            .field("timeout", &self.timeout)
            .field("in_process_hooks_count", &self.in_process_hooks.len())
            .finish()
    }
}

impl PartialEq for HookRunner {
    fn eq(&self, other: &Self) -> bool {
        self.hooks == other.hooks
            && self.timeout == other.timeout
            && self.in_process_hooks.len() == other.in_process_hooks.len()
    }
}

impl Eq for HookRunner {}

impl HookRunner {
    #[must_use]
    pub fn new(hooks: PluginHooks) -> Self {
        Self {
            hooks,
            timeout: Duration::from_secs(DEFAULT_HOOK_TIMEOUT_SECS),
            in_process_hooks: Vec::new(),
        }
    }

    pub fn from_registry(plugin_registry: &PluginRegistry) -> Result<Self, PluginError> {
        Ok(Self::new(plugin_registry.aggregated_hooks()?))
    }

    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn register_in_process_hook(
        &mut self,
        hook: Arc<dyn axagent_runtime_core::plugin_hooks::PluginHook>,
    ) {
        self.in_process_hooks.push(hook);
    }

    #[must_use]
    pub fn in_process_hooks(&self) -> &[Arc<dyn axagent_runtime_core::plugin_hooks::PluginHook>] {
        &self.in_process_hooks
    }

    #[must_use]
    pub fn run_pre_tool_use(&self, tool_name: &str, tool_input: &str) -> HookRunResult {
        Self::run_commands(
            HookEvent::PreToolUse,
            &self.hooks.pre_tool_use,
            tool_name,
            tool_input,
            None,
            false,
            self.timeout,
        )
    }

    #[must_use]
    pub fn run_post_tool_use(
        &self,
        tool_name: &str,
        tool_input: &str,
        tool_output: &str,
        is_error: bool,
    ) -> HookRunResult {
        Self::run_commands(
            HookEvent::PostToolUse,
            &self.hooks.post_tool_use,
            tool_name,
            tool_input,
            Some(tool_output),
            is_error,
            self.timeout,
        )
    }

    #[must_use]
    pub fn run_post_tool_use_failure(
        &self,
        tool_name: &str,
        tool_input: &str,
        tool_error: &str,
    ) -> HookRunResult {
        Self::run_commands(
            HookEvent::PostToolUseFailure,
            &self.hooks.post_tool_use_failure,
            tool_name,
            tool_input,
            Some(tool_error),
            true,
            self.timeout,
        )
    }

    fn run_commands(
        event: HookEvent,
        commands: &[String],
        tool_name: &str,
        tool_input: &str,
        tool_output: Option<&str>,
        is_error: bool,
        timeout: Duration,
    ) -> HookRunResult {
        if commands.is_empty() {
            return HookRunResult::allow(Vec::new());
        }

        let payload = hook_payload(event, tool_name, tool_input, tool_output, is_error).to_string();

        let mut messages = Vec::new();

        for command in commands {
            match Self::run_command(
                command,
                event,
                tool_name,
                tool_input,
                tool_output,
                is_error,
                &payload,
                timeout,
            ) {
                HookCommandOutcome::Allow { message } => {
                    if let Some(message) = message {
                        messages.push(message);
                    }
                },
                HookCommandOutcome::Deny { message } => {
                    messages.push(message.unwrap_or_else(|| {
                        format!("{} hook denied tool `{tool_name}`", event.as_str())
                    }));
                    return HookRunResult {
                        denied: true,
                        failed: false,
                        timed_out: false,
                        messages,
                    };
                },
                HookCommandOutcome::Failed { message } => {
                    messages.push(message);
                    return HookRunResult {
                        denied: false,
                        failed: true,
                        timed_out: false,
                        messages,
                    };
                },
                HookCommandOutcome::TimedOut { message } => {
                    messages.push(message);
                    return HookRunResult {
                        denied: false,
                        failed: true,
                        timed_out: true,
                        messages,
                    };
                },
            }
        }

        HookRunResult::allow(messages)
    }

    #[allow(clippy::too_many_arguments)]
    fn run_command(
        command: &str,
        event: HookEvent,
        tool_name: &str,
        tool_input: &str,
        tool_output: Option<&str>,
        is_error: bool,
        payload: &str,
        timeout: Duration,
    ) -> HookCommandOutcome {
        let mut child = shell_command(command);
        child.stdin(std::process::Stdio::piped());
        child.stdout(std::process::Stdio::piped());
        child.stderr(std::process::Stdio::piped());
        child.env("HOOK_EVENT", event.as_str());
        child.env("HOOK_TOOL_NAME", tool_name);
        child.env("HOOK_TOOL_INPUT", tool_input);
        child.env("HOOK_TOOL_IS_ERROR", if is_error { "1" } else { "0" });
        if let Some(tool_output) = tool_output {
            child.env("HOOK_TOOL_OUTPUT", tool_output);
        }

        match child.spawn_with_timeout(payload.as_bytes(), timeout) {
            Ok(TimeoutOutput::Completed(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                let message = (!stdout.is_empty()).then_some(stdout);
                match output.status.code() {
                    Some(0) => HookCommandOutcome::Allow { message },
                    Some(2) => HookCommandOutcome::Deny { message },
                    Some(code) => HookCommandOutcome::Failed {
                        message: format_hook_warning(
                            command,
                            code,
                            message.as_deref(),
                            stderr.as_str(),
                        ),
                    },
                    None => HookCommandOutcome::Failed {
                        message: format!(
                            "{} hook `{command}` terminated by signal while handling `{tool_name}`",
                            event.as_str()
                        ),
                    },
                }
            },
            Ok(TimeoutOutput::TimedOut) => HookCommandOutcome::TimedOut {
                message: format!(
                    "{} hook `{command}` timed out after {}s while handling `{tool_name}`",
                    event.as_str(),
                    timeout.as_secs(),
                ),
            },
            Err(error) => HookCommandOutcome::Failed {
                message: format!(
                    "{} hook `{command}` failed to start for `{tool_name}`: {error}",
                    event.as_str()
                ),
            },
        }
    }
}

enum HookCommandOutcome {
    Allow { message: Option<String> },
    Deny { message: Option<String> },
    Failed { message: String },
    TimedOut { message: String },
}

fn hook_payload(
    event: HookEvent,
    tool_name: &str,
    tool_input: &str,
    tool_output: Option<&str>,
    is_error: bool,
) -> serde_json::Value {
    match event {
        HookEvent::PostToolUseFailure => json!({
            "hook_event_name": event.as_str(),
            "tool_name": tool_name,
            "tool_input": parse_tool_input(tool_input),
            "tool_input_json": tool_input,
            "tool_error": tool_output,
            "tool_result_is_error": true,
        }),
        _ => json!({
            "hook_event_name": event.as_str(),
            "tool_name": tool_name,
            "tool_input": parse_tool_input(tool_input),
            "tool_input_json": tool_input,
            "tool_output": tool_output,
            "tool_result_is_error": is_error,
        }),
    }
}

fn parse_tool_input(tool_input: &str) -> serde_json::Value {
    serde_json::from_str(tool_input).unwrap_or_else(|_| json!({ "raw": tool_input }))
}

fn format_hook_warning(command: &str, code: i32, stdout: Option<&str>, stderr: &str) -> String {
    let mut message = format!("Hook `{command}` exited with status {code}");
    if let Some(stdout) = stdout.filter(|stdout| !stdout.is_empty()) {
        message.push_str(": ");
        message.push_str(stdout);
    } else if !stderr.is_empty() {
        message.push_str(": ");
        message.push_str(stderr);
    }
    message
}

fn shell_command(command: &str) -> CommandWithStdin {
    #[cfg(windows)]
    let command_builder = {
        let mut command_builder = Command::new("cmd");
        command_builder.arg("/C").arg(command);
        CommandWithStdin::new(command_builder)
    };

    #[cfg(not(windows))]
    let command_builder = if Path::new(command).exists() {
        let mut command_builder = Command::new("sh");
        command_builder.arg(command);
        CommandWithStdin::new(command_builder)
    } else {
        let mut command_builder = Command::new("sh");
        command_builder.arg("-lc").arg(command);
        CommandWithStdin::new(command_builder)
    };

    command_builder
}

struct CommandWithStdin {
    command: Command,
}

impl CommandWithStdin {
    fn new(command: Command) -> Self {
        Self { command }
    }

    fn stdin(&mut self, cfg: std::process::Stdio) -> &mut Self {
        self.command.stdin(cfg);
        self
    }

    fn stdout(&mut self, cfg: std::process::Stdio) -> &mut Self {
        self.command.stdout(cfg);
        self
    }

    fn stderr(&mut self, cfg: std::process::Stdio) -> &mut Self {
        self.command.stderr(cfg);
        self
    }

    fn env<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.command.env(key, value);
        self
    }

    fn spawn_with_timeout(
        &mut self,
        stdin_data: &[u8],
        timeout: Duration,
    ) -> std::io::Result<TimeoutOutput> {
        self.command.stdin(std::process::Stdio::piped());
        self.command.stdout(std::process::Stdio::piped());
        self.command.stderr(std::process::Stdio::piped());

        let mut child = self.command.spawn()?;

        if let Some(mut child_stdin) = child.stdin.take() {
            use std::io::Write as _;
            match child_stdin.write_all(stdin_data) {
                Ok(()) => {},
                Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => {},
                Err(error) => return Err(error),
            }
        }

        match child.try_wait() {
            Ok(Some(_status)) => {
                let output = child.wait_with_output()?;
                Ok(TimeoutOutput::Completed(output))
            },
            Ok(None) => {
                let start = std::time::Instant::now();
                loop {
                    match child.try_wait() {
                        Ok(Some(_)) => {
                            let output = child.wait_with_output()?;
                            return Ok(TimeoutOutput::Completed(output));
                        },
                        Ok(None) => {
                            if start.elapsed() >= timeout {
                                let _ = child.kill();
                                let _ = child.wait();
                                return Ok(TimeoutOutput::TimedOut);
                            }
                            std::thread::sleep(Duration::from_millis(50));
                        },
                        Err(e) => return Err(e),
                    }
                }
            },
            Err(e) => Err(e),
        }
    }
}

enum TimeoutOutput {
    Completed(std::process::Output),
    TimedOut,
}

#[cfg(test)]
mod tests {
    use super::{HookRunResult, HookRunner};
    use crate::{PluginManager, PluginManagerConfig};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("plugins-hook-runner-{label}-{nanos}"))
    }

    fn make_executable(path: &Path) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o755);
            fs::set_permissions(path, perms).unwrap_or_else(|e| {
                tracing::error!("chmod +x {}: {e}", path.display());
            });
        }
        #[cfg(not(unix))]
        let _ = path;
    }

    #[cfg(windows)]
    fn write_script(path: &Path, message: &str) {
        fs::write(path, format!("@echo off\r\necho {message}\r\n")).expect("write script");
    }

    #[cfg(not(windows))]
    fn write_script(path: &Path, message: &str) {
        fs::write(path, format!("#!/bin/sh\nprintf '%s\\n' '{message}'\n")).expect("write script");
    }

    fn write_hook_plugin(
        root: &Path,
        name: &str,
        pre_message: &str,
        post_message: &str,
        failure_message: &str,
    ) {
        fs::create_dir_all(root.join(".claude-plugin")).expect("manifest dir");
        fs::create_dir_all(root.join("hooks")).expect("hooks dir");

        #[cfg(windows)]
        let (pre_ext, post_ext, failure_ext) = ("pre.cmd", "post.cmd", "failure.cmd");
        #[cfg(not(windows))]
        let (pre_ext, post_ext, failure_ext) = ("pre.sh", "post.sh", "failure.sh");

        let pre_path = root.join("hooks").join(pre_ext);
        write_script(&pre_path, pre_message);
        make_executable(&pre_path);

        let post_path = root.join("hooks").join(post_ext);
        write_script(&post_path, post_message);
        make_executable(&post_path);

        let failure_path = root.join("hooks").join(failure_ext);
        write_script(&failure_path, failure_message);
        make_executable(&failure_path);
        fs::write(
            root.join(".claude-plugin").join("plugin.json"),
            format!(
                "{{\n  \"name\": \"{name}\",\n  \"version\": \"1.0.0\",\n  \"description\": \"hook plugin\",\n  \"hooks\": {{\n    \"PreToolUse\": [\"./hooks/{pre_ext}\"],\n    \"PostToolUse\": [\"./hooks/{post_ext}\"],\n    \"PostToolUseFailure\": [\"./hooks/{failure_ext}\"]\n  }}\n}}"
            ),
        )
        .expect("write plugin manifest");
    }

    #[test]
    fn collects_and_runs_hooks_from_enabled_plugins() {
        let config_home = temp_dir("config");
        let first_source_root = temp_dir("source-a");
        let second_source_root = temp_dir("source-b");
        write_hook_plugin(
            &first_source_root,
            "first",
            "plugin pre one",
            "plugin post one",
            "plugin failure one",
        );
        write_hook_plugin(
            &second_source_root,
            "second",
            "plugin pre two",
            "plugin post two",
            "plugin failure two",
        );

        let mut manager = PluginManager::new(PluginManagerConfig::new(&config_home));
        manager
            .install(first_source_root.to_str().expect("utf8 path"))
            .expect("first plugin install should succeed");
        manager
            .install(second_source_root.to_str().expect("utf8 path"))
            .expect("second plugin install should succeed");
        let registry = manager.plugin_registry().expect("registry should build");

        let runner = HookRunner::from_registry(&registry).expect("plugin hooks should load");

        assert_eq!(
            runner.run_pre_tool_use("Read", r#"{"path":"README.md"}"#),
            HookRunResult::allow(vec!["plugin pre one".to_string(), "plugin pre two".to_string(),])
        );
        assert_eq!(
            runner.run_post_tool_use("Read", r#"{"path":"README.md"}"#, "ok", false),
            HookRunResult::allow(vec![
                "plugin post one".to_string(),
                "plugin post two".to_string(),
            ])
        );
        assert_eq!(
            runner.run_post_tool_use_failure("Read", r#"{"path":"README.md"}"#, "tool failed",),
            HookRunResult::allow(vec![
                "plugin failure one".to_string(),
                "plugin failure two".to_string(),
            ])
        );

        let _ = fs::remove_dir_all(config_home);
        let _ = fs::remove_dir_all(first_source_root);
        let _ = fs::remove_dir_all(second_source_root);
    }

    #[test]
    fn pre_tool_use_denies_when_plugin_hook_exits_two() {
        #[cfg(windows)]
        let deny_cmd = "echo blocked by plugin & exit 2";
        #[cfg(not(windows))]
        let deny_cmd = "printf 'blocked by plugin'; exit 2";

        let runner = HookRunner::new(crate::PluginHooks {
            pre_tool_use: vec![deny_cmd.to_string()],
            post_tool_use: Vec::new(),
            post_tool_use_failure: Vec::new(),
        });

        let result = runner.run_pre_tool_use("Bash", r#"{"command":"pwd"}"#);

        assert!(result.is_denied());
        assert_eq!(result.messages(), &["blocked by plugin".to_string()]);
    }

    #[test]
    fn propagates_plugin_hook_failures() {
        #[cfg(windows)]
        let (fail_cmd, later_cmd) = ("echo broken plugin hook & exit 1", "echo later plugin hook");
        #[cfg(not(windows))]
        let (fail_cmd, later_cmd) =
            ("printf 'broken plugin hook'; exit 1", "printf 'later plugin hook'");

        let runner = HookRunner::new(crate::PluginHooks {
            pre_tool_use: vec![fail_cmd.to_string(), later_cmd.to_string()],
            post_tool_use: Vec::new(),
            post_tool_use_failure: Vec::new(),
        });

        let result = runner.run_pre_tool_use("Bash", r#"{"command":"pwd"}"#);

        assert!(result.is_failed());
        assert!(
            result
                .messages()
                .iter()
                .any(|message| message.contains("broken plugin hook"))
        );
        assert!(
            !result
                .messages()
                .iter()
                .any(|message| message == "later plugin hook")
        );
    }

    #[test]
    fn hook_timeout_kills_stuck_process() {
        #[cfg(windows)]
        let stuck_cmd = "powershell -Command \"Start-Sleep -Seconds 60\"";
        #[cfg(not(windows))]
        let stuck_cmd = "sleep 60";

        let runner = HookRunner::new(crate::PluginHooks {
            pre_tool_use: vec![stuck_cmd.to_string()],
            post_tool_use: Vec::new(),
            post_tool_use_failure: Vec::new(),
        })
        .with_timeout(Duration::from_secs(5));

        let start = std::time::Instant::now();
        let result = runner.run_pre_tool_use("Bash", r#"{"command":"pwd"}"#);
        let elapsed = start.elapsed();

        assert!(
            result.is_timed_out(),
            "expected timed_out, got: denied={}, failed={}, messages={:?}",
            result.is_denied(),
            result.is_failed(),
            result.messages()
        );
        assert!(
            elapsed < Duration::from_secs(15),
            "timeout should have killed the process quickly, took {:?}",
            elapsed
        );
    }

    #[test]
    #[cfg(unix)]
    fn generated_hook_scripts_are_executable() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_dir("exec-guard");
        write_hook_plugin(&root, "exec-check", "pre", "post", "fail");

        for script in ["pre.sh", "post.sh", "failure.sh"] {
            let path = root.join("hooks").join(script);
            let mode = fs::metadata(&path)
                .unwrap_or_else(|e| panic!("{script} metadata: {e}"))
                .permissions()
                .mode();
            assert!(
                mode & 0o111 != 0,
                "{script} must have at least one execute bit set, got mode {mode:#o}"
            );
        }
    }
}
