use crate::{PermissionResult, Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;

const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// SECURITY (C5): 绝对禁止的写入路径前缀 — 命中后一律 Block（不询问）。
const FORBIDDEN_PREFIXES: &[&str] = &[
    "/etc",
    "/boot",
    "/sys",
    "/proc",
    "/dev",
    "/var/lib",
    "/var/run",
    "/var/log",
    "/root/.ssh",
    "/root/.gnupg",
    "/home/*/.ssh",
    "/home/*/.gnupg",
    "/Users/*/.ssh",
    "C:\\Windows",
    "C:\\Program Files",
    "C:\\Program Files (x86)",
    "C:\\ProgramData",
];

/// 询问级：写入需要用户确认的路径。
const ASK_PREFIXES: &[&str] = &[
    "/usr",
    "/opt",
    "/bin",
    "/sbin",
    "/lib",
    "/lib64",
    "/etc",
    "C:\\Users\\*\\AppData\\Roaming\\Microsoft",
    "C:\\Users\\*\\NTUSER",
];

pub struct FileWriteTool;

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "FileWrite"
    }
    fn description(&self) -> &str {
        "创建新文件或完全覆盖已有文件（⚠️ 不可逆）。自动创建父目录。\
         适用：新建代码文件、配置文件、脚本。不适用：修改现有文件（用 FileEdit）、追加内容。\
         file_path 必须是绝对路径，内容为字符串。大文件（>50MB）将被拒绝。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "目标文件的绝对路径"
                },
                "content": {
                    "type": "string",
                    "description": "要写入的文件内容"
                }
            },
            "required": ["file_path", "content"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::FileWrite
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    fn is_destructive(&self) -> bool {
        true
    }

    async fn validate(&self, input: &Value, ctx: &ToolContext) -> Result<(), ToolError> {
        let path = input["file_path"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_input_for("FileWrite", "缺少 file_path 参数"))?;

        if !Path::new(path).is_absolute() {
            return Err(ToolError::invalid_input_for("FileWrite", "file_path 必须是绝对路径"));
        }
        if path.contains('\0') {
            return Err(ToolError::invalid_input_for("FileWrite", "file_path 含 NUL"));
        }
        if path.contains("..") || path.starts_with('~') {
            return Err(ToolError::invalid_input_for(
                "FileWrite",
                "file_path 包含禁止的路径遍历模式 (.. 或 ~)",
            ));
        }
        // 阻断硬禁区
        for bad in FORBIDDEN_PREFIXES {
            if matches_glob(bad, path) {
                return Err(ToolError::permission_denied(
                    "FileWrite",
                    &format!("禁止写入路径: {} 命中硬禁前缀 {}", path, bad),
                ));
            }
        }

        let content = input["content"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_input_for("FileWrite", "缺少 content 参数"))?;

        if content.len() as u64 > MAX_FILE_SIZE {
            return Err(ToolError::invalid_input(format!(
                "内容过大 ({} MB)，最大允许 {} MB",
                content.len() / 1024 / 1024,
                MAX_FILE_SIZE / 1024 / 1024
            )));
        }

        if !ctx.allow_write {
            return Err(ToolError::permission_denied("FileWrite", "当前上下文不允许写入操作"));
        }

        Ok(())
    }

    fn check_permissions(&self, input: &Value, ctx: &ToolContext) -> PermissionResult {
        let path = input["file_path"].as_str().unwrap_or("");
        let working_dir = std::path::Path::new(&ctx.working_dir);

        // 阻断硬禁
        for bad in FORBIDDEN_PREFIXES {
            if matches_glob(bad, path) {
                return PermissionResult::Deny(format!("路径命中硬禁: {}", bad));
            }
        }
        // 询问级
        for ask in ASK_PREFIXES {
            if matches_glob(ask, path) {
                return PermissionResult::Ask(format!("写入系统/共享路径 '{}'，确认？", path));
            }
        }

        // 解析符号链接后再次确认
        if let Ok(canonical) = std::fs::canonicalize(path) {
            for bad in FORBIDDEN_PREFIXES {
                if matches_glob(bad, &canonical.to_string_lossy()) {
                    return PermissionResult::Deny(format!(
                        "符号链接解析后命中硬禁: {}",
                        canonical.display()
                    ));
                }
            }
        }

        // 工作区外 → 询问
        if !is_within_workspace(Path::new(path), working_dir) {
            return PermissionResult::Ask(format!(
                "目标 {} 不在工作区 {} 内，确认写入？",
                path,
                working_dir.display()
            ));
        }

        PermissionResult::Allow
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        // SECURITY: 二次强制检查
        match self.check_permissions(&input, ctx) {
            PermissionResult::Allow => {},
            PermissionResult::Deny(reason) => {
                return Err(ToolError::permission_denied("FileWrite", &reason));
            },
            PermissionResult::Ask(_) => {
                return Err(ToolError::permission_denied(
                    "FileWrite",
                    "需要用户确认 (未配置自动批准)",
                ));
            },
        }

        let file_path = input["file_path"].as_str().unwrap_or("");
        if file_path.is_empty() {
            return Err(ToolError::invalid_input_for("FileWrite", "缺少 file_path 参数"));
        }
        let content = input["content"].as_str().unwrap_or("");
        if content.is_empty() && !input["content"].is_string() {
            return Err(ToolError::invalid_input_for("FileWrite", "缺少 content 参数"));
        }

        let path = Path::new(file_path);

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| ToolError::execution_failed(format!("创建父目录失败: {}", e)))?;
        }

        let existed = path.exists();
        let old_content = if existed {
            tokio::fs::read_to_string(path).await.ok()
        } else {
            None
        };

        tokio::fs::write(path, content)
            .await
            .map_err(|e| ToolError::execution_failed(format!("写入文件失败: {}", e)))?;

        let action = if existed { "更新" } else { "创建" };
        let mut output = format!("✅ 已{}文件: {}\n", action, file_path);

        if let Some(old) = old_content
            && old != content
            && old.len() < 50_000
            && content.len() < 50_000
        {
            output.push_str("\n## 变更对比\n```diff\n");
            for diff in diff::lines(&old, content) {
                match diff {
                    diff::Result::Left(l) => output.push_str(&format!("-{}\n", l)),
                    diff::Result::Right(r) => output.push_str(&format!("+{}\n", r)),
                    diff::Result::Both(b, _) => output.push_str(&format!(" {}\n", b)),
                }
            }
            output.push_str("```\n");
        }

        let lines = content.lines().count();
        output.push_str(&format!("\n{} 行, {} 字节", lines, content.len()));

        Ok(ToolResult::success(output))
    }
}

/// 简易 glob 匹配：仅支持 `*`（匹配单个路径段）。
/// 模式与文本按 `/` 或 `\` 切段比较：每个段必须相等或为 `*`；
/// 模式可作为前缀匹配更深的路径（但不能跨段匹配，如 `/etc` 不应匹配 `/etcfoo`）。
fn matches_glob(pattern: &str, text: &str) -> bool {
    let pattern_trim = pattern.trim_end_matches(['/', '\\']);

    let p_segs: Vec<&str> = pattern_trim
        .split(['/', '\\'])
        .filter(|s| !s.is_empty())
        .collect();
    let t_segs: Vec<&str> = text.split(['/', '\\']).filter(|s| !s.is_empty()).collect();

    if p_segs.len() > t_segs.len() {
        return false;
    }
    for (i, p_seg) in p_segs.iter().enumerate() {
        if *p_seg == "*" {
            continue;
        }
        if *p_seg != t_segs[i] {
            return false;
        }
    }
    true
}

fn is_within_workspace(path: &Path, workspace: &Path) -> bool {
    let p = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let w = std::fs::canonicalize(workspace).unwrap_or_else(|_| workspace.to_path_buf());
    let p_comp: Vec<std::path::Component> = p.components().collect();
    let w_comp: Vec<std::path::Component> = w.components().collect();
    if p_comp.len() < w_comp.len() {
        return false;
    }
    p_comp.iter().take(w_comp.len()).eq(w_comp.iter())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn blocks_system_paths() {
        assert!(matches_glob("/etc", "/etc/passwd"));
        // SECURITY: 路径段必须严格匹配，/etc 不应误判 /etcfoo/*
        assert!(!matches_glob("/etc", "/etcfoo/x"));
        assert!(!matches_glob("/etc", "/etcfoo"));
    }

    #[test]
    fn workspace_components_strict() {
        let p = PathBuf::from("/workspace/inside.txt");
        let w = PathBuf::from("/workspace");
        assert!(is_within_workspace(&p, &w));
        let p2 = PathBuf::from("/workspace_evil/x");
        assert!(!is_within_workspace(&p2, &w));
    }
}
