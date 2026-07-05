// SPDX-License-Identifier: AGPL-3.0-only
use super::{
    CLAW_SETTINGS_SCHEMA_NAME, ConfigLoader, ConfigSource, McpServerConfig, McpTransport,
    ResolvedPermissionMode, RuntimeHookConfig, RuntimePluginConfig, deep_merge_objects,
    parse_permission_mode_label,
};
use crate::json::JsonValue;
use crate::sandbox::FilesystemIsolationMode;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir() -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("runtime-config-{nanos}"))
}

#[test]
fn rejects_non_object_settings_files() {
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    fs::create_dir_all(&home).expect("home config dir");
    fs::create_dir_all(&cwd).expect("project dir");
    fs::write(home.join("settings.json"), "[]").expect("write bad settings");

    let error = ConfigLoader::new(&cwd, &home)
        .load()
        .expect_err("config should fail");
    assert!(
        error
            .to_string()
            .contains("top-level settings value must be a JSON object")
    );

    if root.exists() {
        fs::remove_dir_all(root).expect("cleanup temp dir");
    }
}

#[test]
fn loads_and_merges_claude_code_config_files_by_precedence() {
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    fs::create_dir_all(cwd.join(".claw")).expect("project config dir");
    fs::create_dir_all(&home).expect("home config dir");

    fs::write(
            home.parent().expect("home parent").join(".claw.json"),
            r#"{"model":"haiku","env":{"A":"1"},"mcpServers":{"home":{"command":"uvx","args":["home"]}}}"#,
        )
        .expect("write user compat config");
    fs::write(
            home.join("settings.json"),
            r#"{"model":"sonnet","env":{"A2":"1"},"hooks":{"PreToolUse":["base"]},"permissions":{"defaultMode":"plan","allow":["Read"],"deny":["Bash(rm -rf)"]}}"#,
        )
        .expect("write user settings");
    fs::write(cwd.join(".claw.json"), r#"{"model":"project-compat","env":{"B":"2"}}"#)
        .expect("write project compat config");
    fs::write(
            cwd.join(".claw").join("settings.json"),
            r#"{"env":{"C":"3"},"hooks":{"PostToolUse":["project"],"PostToolUseFailure":["project-failure"]},"permissions":{"ask":["Edit"]},"mcpServers":{"project":{"command":"uvx","args":["project"]}}}"#,
        )
        .expect("write project settings");
    fs::write(
        cwd.join(".claw").join("settings.local.json"),
        r#"{"model":"opus","permissionMode":"acceptEdits"}"#,
    )
    .expect("write local settings");

    let loaded = ConfigLoader::new(&cwd, &home)
        .load()
        .expect("config should load");

    assert_eq!(CLAW_SETTINGS_SCHEMA_NAME, "SettingsSchema");
    assert_eq!(loaded.loaded_entries().len(), 5);
    assert_eq!(loaded.loaded_entries()[0].source, ConfigSource::User);
    assert_eq!(loaded.get("model"), Some(&JsonValue::String("opus".to_string())));
    assert_eq!(loaded.model(), Some("opus"));
    assert_eq!(loaded.permission_mode(), Some(ResolvedPermissionMode::WorkspaceWrite));
    assert_eq!(
        loaded
            .get("env")
            .and_then(JsonValue::as_object)
            .expect("env object")
            .len(),
        4
    );
    assert!(
        loaded
            .get("hooks")
            .and_then(JsonValue::as_object)
            .expect("hooks object")
            .contains_key("PreToolUse")
    );
    assert!(
        loaded
            .get("hooks")
            .and_then(JsonValue::as_object)
            .expect("hooks object")
            .contains_key("PostToolUse")
    );
    assert_eq!(loaded.hooks().pre_tool_use(), &["base".to_string()]);
    assert_eq!(loaded.hooks().post_tool_use(), &["project".to_string()]);
    assert_eq!(loaded.hooks().post_tool_use_failure(), &["project-failure".to_string()]);
    assert_eq!(loaded.permission_rules().allow(), &["Read".to_string()]);
    assert_eq!(loaded.permission_rules().deny(), &["Bash(rm -rf)".to_string()]);
    assert_eq!(loaded.permission_rules().ask(), &["Edit".to_string()]);
    assert!(loaded.mcp().get("home").is_some());
    assert!(loaded.mcp().get("project").is_some());

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn parses_sandbox_config() {
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    fs::create_dir_all(cwd.join(".claw")).expect("project config dir");
    fs::create_dir_all(&home).expect("home config dir");

    fs::write(
        cwd.join(".claw").join("settings.local.json"),
        r#"{
              "sandbox": {
                "enabled": true,
                "namespaceRestrictions": false,
                "networkIsolation": true,
                "filesystemMode": "allow-list",
                "allowedMounts": ["logs", "tmp/cache"]
              }
            }"#,
    )
    .expect("write local settings");

    let loaded = ConfigLoader::new(&cwd, &home)
        .load()
        .expect("config should load");

    assert_eq!(loaded.sandbox().enabled, Some(true));
    assert_eq!(loaded.sandbox().namespace_restrictions, Some(false));
    assert_eq!(loaded.sandbox().network_isolation, Some(true));
    assert_eq!(loaded.sandbox().filesystem_mode, Some(FilesystemIsolationMode::AllowList));
    assert_eq!(loaded.sandbox().allowed_mounts, vec!["logs", "tmp/cache"]);

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn parses_provider_fallbacks_chain_with_primary_and_ordered_fallbacks() {
    // given
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    fs::create_dir_all(cwd.join(".claw")).expect("project config dir");
    fs::create_dir_all(&home).expect("home config dir");
    fs::write(
        home.join("settings.json"),
        r#"{
              "providerFallbacks": {
                "primary": "claude-opus-4-6",
                "fallbacks": ["deepseek-v4-flash", "gpt-5.4-mini"]
              }
            }"#,
    )
    .expect("write provider fallback settings");

    // when
    let loaded = ConfigLoader::new(&cwd, &home)
        .load()
        .expect("config should load");

    // then
    let chain = loaded.provider_fallbacks();
    assert_eq!(chain.primary(), Some("claude-opus-4-6"));
    assert_eq!(
        chain.fallbacks(),
        &["deepseek-v4-flash".to_string(), "gpt-5.4-mini".to_string()]
    );
    assert!(!chain.is_empty());

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn provider_fallbacks_default_is_empty_when_unset() {
    // given
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    fs::create_dir_all(&home).expect("home config dir");
    fs::create_dir_all(&cwd).expect("project dir");
    fs::write(home.join("settings.json"), "{}").expect("write empty settings");

    // when
    let loaded = ConfigLoader::new(&cwd, &home)
        .load()
        .expect("config should load");

    // then
    let chain = loaded.provider_fallbacks();
    assert_eq!(chain.primary(), None);
    assert!(chain.fallbacks().is_empty());
    assert!(chain.is_empty());

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn parses_trusted_roots_from_settings() {
    // given
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    fs::create_dir_all(&home).expect("home config dir");
    fs::create_dir_all(&cwd).expect("project dir");
    fs::write(
        home.join("settings.json"),
        r#"{"trustedRoots": ["/tmp/worktrees", "/home/user/projects"]}"#,
    )
    .expect("write settings");

    // when
    let loaded = ConfigLoader::new(&cwd, &home)
        .load()
        .expect("config should load");

    // then
    let roots = loaded.trusted_roots();
    assert_eq!(roots, ["/tmp/worktrees", "/home/user/projects"]);

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn trusted_roots_default_is_empty_when_unset() {
    // given
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    fs::create_dir_all(&home).expect("home config dir");
    fs::create_dir_all(&cwd).expect("project dir");
    fs::write(home.join("settings.json"), "{}").expect("write empty settings");

    // when
    let loaded = ConfigLoader::new(&cwd, &home)
        .load()
        .expect("config should load");

    // then
    assert!(loaded.trusted_roots().is_empty());

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn parses_typed_mcp_and_oauth_config() {
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    fs::create_dir_all(cwd.join(".claw")).expect("project config dir");
    fs::create_dir_all(&home).expect("home config dir");

    fs::write(
            home.join("settings.json"),
            r#"{
              "mcpServers": {
                "stdio-server": {
                  "command": "uvx",
                  "args": ["mcp-server"],
                  "env": {"TOKEN": "secret"}
                },
                "remote-server": {
                  "type": "http",
                  "url": "https://example.test/mcp",
                  "headers": {"Authorization": "Bearer token"},
                  "headersHelper": "helper.sh",
                  "oauth": {
                    "clientId": "mcp-client",
                    "callbackPort": 7777,
                    "authServerMetadataUrl": "https://issuer.test/.well-known/oauth-authorization-server",
                    "xaa": true
                  }
                }
              },
              "oauth": {
                "clientId": "runtime-client",
                "authorizeUrl": "https://console.test/oauth/authorize",
                "tokenUrl": "https://console.test/oauth/token",
                "callbackPort": 54545,
                "manualRedirectUrl": "https://console.test/oauth/callback",
                "scopes": ["org:read", "user:write"]
              }
            }"#,
        )
        .expect("write user settings");
    fs::write(
        cwd.join(".claw").join("settings.local.json"),
        r#"{
              "mcpServers": {
                "remote-server": {
                  "type": "ws",
                  "url": "wss://override.test/mcp",
                  "headers": {"X-Env": "local"}
                }
              }
            }"#,
    )
    .expect("write local settings");

    let loaded = ConfigLoader::new(&cwd, &home)
        .load()
        .expect("config should load");

    let stdio_server = loaded
        .mcp()
        .get("stdio-server")
        .expect("stdio server should exist");
    assert_eq!(stdio_server.scope, ConfigSource::User);
    assert_eq!(stdio_server.transport(), McpTransport::Stdio);

    let remote_server = loaded
        .mcp()
        .get("remote-server")
        .expect("remote server should exist");
    assert_eq!(remote_server.scope, ConfigSource::Local);
    assert_eq!(remote_server.transport(), McpTransport::Ws);
    match &remote_server.config {
        McpServerConfig::Ws(config) => {
            assert_eq!(config.url, "wss://override.test/mcp");
            assert_eq!(config.headers.get("X-Env").map(String::as_str), Some("local"));
        },
        other => panic!("expected ws config, got {other:?}"),
    }

    let oauth = loaded.oauth().expect("oauth config should exist");
    assert_eq!(oauth.client_id, "runtime-client");
    assert_eq!(oauth.callback_port, Some(54_545));
    assert_eq!(oauth.scopes, vec!["org:read", "user:write"]);

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn infers_http_mcp_servers_from_url_only_config() {
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    fs::create_dir_all(&home).expect("home config dir");
    fs::create_dir_all(&cwd).expect("project dir");
    fs::write(
        home.join("settings.json"),
        r#"{
              "mcpServers": {
                "remote": {
                  "url": "https://example.test/mcp"
                }
              }
            }"#,
    )
    .expect("write mcp settings");

    let loaded = ConfigLoader::new(&cwd, &home)
        .load()
        .expect("config should load");

    let remote_server = loaded
        .mcp()
        .get("remote")
        .expect("remote server should exist");
    assert_eq!(remote_server.transport(), McpTransport::Http);
    match &remote_server.config {
        McpServerConfig::Http(config) => {
            assert_eq!(config.url, "https://example.test/mcp");
        },
        other => panic!("expected http config, got {other:?}"),
    }

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn parses_plugin_config_from_enabled_plugins() {
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    fs::create_dir_all(cwd.join(".claw")).expect("project config dir");
    fs::create_dir_all(&home).expect("home config dir");

    fs::write(
        home.join("settings.json"),
        r#"{
              "enabledPlugins": {
                "tool-guard@builtin": true,
                "sample-plugin@external": false
              }
            }"#,
    )
    .expect("write user settings");

    let loaded = ConfigLoader::new(&cwd, &home)
        .load()
        .expect("config should load");

    assert_eq!(loaded.plugins().enabled_plugins().get("tool-guard@builtin"), Some(&true));
    assert_eq!(
        loaded
            .plugins()
            .enabled_plugins()
            .get("sample-plugin@external"),
        Some(&false)
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn parses_plugin_config() {
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    fs::create_dir_all(cwd.join(".claw")).expect("project config dir");
    fs::create_dir_all(&home).expect("home config dir");

    fs::write(
        home.join("settings.json"),
        r#"{
              "enabledPlugins": {
                "core-helpers@builtin": true
              },
              "plugins": {
                "externalDirectories": ["./external-plugins"],
                "installRoot": "plugin-cache/installed",
                "registryPath": "plugin-cache/installed.json",
                "bundledRoot": "./bundled-plugins"
              }
            }"#,
    )
    .expect("write plugin settings");

    let loaded = ConfigLoader::new(&cwd, &home)
        .load()
        .expect("config should load");

    assert_eq!(
        loaded
            .plugins()
            .enabled_plugins()
            .get("core-helpers@builtin"),
        Some(&true)
    );
    assert_eq!(loaded.plugins().external_directories(), &["./external-plugins".to_string()]);
    assert_eq!(loaded.plugins().install_root(), Some("plugin-cache/installed"));
    assert_eq!(loaded.plugins().registry_path(), Some("plugin-cache/installed.json"));
    assert_eq!(loaded.plugins().bundled_root(), Some("./bundled-plugins"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn rejects_invalid_mcp_server_shapes() {
    // given
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    fs::create_dir_all(&home).expect("home config dir");
    fs::create_dir_all(&cwd).expect("project dir");
    fs::write(
        home.join("settings.json"),
        r#"{"mcpServers":{"broken":{"type":"http","url":123}}}"#,
    )
    .expect("write broken settings");

    // when
    let error = ConfigLoader::new(&cwd, &home)
        .load()
        .expect_err("config should fail");

    // then
    assert!(
        error
            .to_string()
            .contains("mcpServers.broken: missing string field url")
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn parses_user_defined_model_aliases_from_settings() {
    // given
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    fs::create_dir_all(cwd.join(".claw")).expect("project config dir");
    fs::create_dir_all(&home).expect("home config dir");

    fs::write(
        home.join("settings.json"),
        r#"{"aliases":{"fast":"claude-haiku-4-5-20251213","smart":"claude-opus-4-6"}}"#,
    )
    .expect("write user settings");
    fs::write(
        cwd.join(".claw").join("settings.local.json"),
        r#"{"aliases":{"smart":"claude-sonnet-4-6","cheap":"deepseek-v4-flash"}}"#,
    )
    .expect("write local settings");

    // when
    let loaded = ConfigLoader::new(&cwd, &home)
        .load()
        .expect("config should load");

    // then
    let aliases = loaded.aliases();
    assert_eq!(aliases.get("fast").map(String::as_str), Some("claude-haiku-4-5-20251213"));
    assert_eq!(aliases.get("smart").map(String::as_str), Some("claude-sonnet-4-6"));
    assert_eq!(aliases.get("cheap").map(String::as_str), Some("deepseek-v4-flash"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn empty_settings_file_loads_defaults() {
    // given
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    fs::create_dir_all(&home).expect("home config dir");
    fs::create_dir_all(&cwd).expect("project dir");
    fs::write(home.join("settings.json"), "").expect("write empty settings");

    // when
    let loaded = ConfigLoader::new(&cwd, &home)
        .load()
        .expect("empty settings should still load");

    // then
    assert_eq!(loaded.loaded_entries().len(), 1);
    assert_eq!(loaded.permission_mode(), None);
    assert_eq!(loaded.plugins().enabled_plugins().len(), 0);

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn deep_merge_objects_merges_nested_maps() {
    // given
    let mut target = JsonValue::parse(r#"{"env":{"A":"1","B":"2"},"model":"haiku"}"#)
        .expect("target JSON should parse")
        .as_object()
        .expect("target should be an object")
        .clone();
    let source = JsonValue::parse(r#"{"env":{"B":"override","C":"3"},"sandbox":{"enabled":true}}"#)
        .expect("source JSON should parse")
        .as_object()
        .expect("source should be an object")
        .clone();

    // when
    deep_merge_objects(&mut target, &source);

    // then
    let env = target
        .get("env")
        .and_then(JsonValue::as_object)
        .expect("env should remain an object");
    assert_eq!(env.get("A"), Some(&JsonValue::String("1".to_string())));
    assert_eq!(env.get("B"), Some(&JsonValue::String("override".to_string())));
    assert_eq!(env.get("C"), Some(&JsonValue::String("3".to_string())));
    assert!(target.contains_key("sandbox"));
}

#[test]
fn rejects_invalid_hook_entries_before_merge() {
    // given
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    let project_settings = cwd.join(".claw").join("settings.json");
    fs::create_dir_all(cwd.join(".claw")).expect("project config dir");
    fs::create_dir_all(&home).expect("home config dir");

    fs::write(home.join("settings.json"), r#"{"hooks":{"PreToolUse":["base"]}}"#)
        .expect("write user settings");
    fs::write(&project_settings, r#"{"hooks":{"PreToolUse":["project",42]}}"#)
        .expect("write invalid project settings");

    // when
    let error = ConfigLoader::new(&cwd, &home)
        .load()
        .expect_err("config should fail");

    // then — config validation now catches the mixed array before the hooks parser
    let rendered = error.to_string();
    assert!(
        rendered.contains("hooks.PreToolUse") && rendered.contains("must be an array of strings"),
        "expected validation error for hooks.PreToolUse, got: {rendered}"
    );
    assert!(!rendered.contains("merged settings.hooks"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn permission_mode_aliases_resolve_to_expected_modes() {
    // given / when / then
    assert_eq!(
        parse_permission_mode_label("plan", "test").expect("plan should resolve"),
        ResolvedPermissionMode::ReadOnly
    );
    assert_eq!(
        parse_permission_mode_label("acceptEdits", "test").expect("acceptEdits should resolve"),
        ResolvedPermissionMode::WorkspaceWrite
    );
    assert_eq!(
        parse_permission_mode_label("dontAsk", "test").expect("dontAsk should resolve"),
        ResolvedPermissionMode::DangerFullAccess
    );
}

#[test]
fn hook_config_merge_preserves_uniques() {
    // given
    let base = RuntimeHookConfig::new(
        vec!["pre-a".to_string()],
        vec!["post-a".to_string()],
        vec!["failure-a".to_string()],
    );
    let overlay = RuntimeHookConfig::new(
        vec!["pre-a".to_string(), "pre-b".to_string()],
        vec!["post-a".to_string(), "post-b".to_string()],
        vec!["failure-b".to_string()],
    );

    // when
    let merged = base.merged(&overlay);

    // then
    assert_eq!(merged.pre_tool_use(), &["pre-a".to_string(), "pre-b".to_string()]);
    assert_eq!(merged.post_tool_use(), &["post-a".to_string(), "post-b".to_string()]);
    assert_eq!(
        merged.post_tool_use_failure(),
        &["failure-a".to_string(), "failure-b".to_string()]
    );
}

#[test]
fn plugin_state_falls_back_to_default_for_unknown_plugin() {
    // given
    let mut config = RuntimePluginConfig::default();
    config.set_plugin_state("known".to_string(), true);

    // when / then
    assert!(config.state_for("known", false));
    assert!(config.state_for("missing", true));
    assert!(!config.state_for("missing", false));
}

#[test]
fn validates_unknown_top_level_keys_with_line_and_field_name() {
    // given
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    let user_settings = home.join("settings.json");
    fs::create_dir_all(&home).expect("home config dir");
    fs::create_dir_all(&cwd).expect("project dir");
    fs::write(&user_settings, "{\n  \"model\": \"opus\",\n  \"telemetry\": true\n}\n")
        .expect("write user settings");

    // when
    let error = ConfigLoader::new(&cwd, &home)
        .load()
        .expect_err("config should fail");

    // then
    let rendered = error.to_string();
    assert!(
        rendered.contains(&user_settings.display().to_string()),
        "error should include file path, got: {rendered}"
    );
    assert!(rendered.contains("line 3"), "error should include line number, got: {rendered}");
    assert!(
        rendered.contains("telemetry"),
        "error should name the offending field, got: {rendered}"
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn validates_deprecated_top_level_keys_with_replacement_guidance() {
    // given
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    let user_settings = home.join("settings.json");
    fs::create_dir_all(&home).expect("home config dir");
    fs::create_dir_all(&cwd).expect("project dir");
    fs::write(&user_settings, "{\n  \"model\": \"opus\",\n  \"allowedTools\": [\"Read\"]\n}\n")
        .expect("write user settings");

    // when
    let error = ConfigLoader::new(&cwd, &home)
        .load()
        .expect_err("config should fail");

    // then
    let rendered = error.to_string();
    assert!(
        rendered.contains(&user_settings.display().to_string()),
        "error should include file path, got: {rendered}"
    );
    assert!(rendered.contains("line 3"), "error should include line number, got: {rendered}");
    assert!(
        rendered.contains("allowedTools"),
        "error should call out the unknown field, got: {rendered}"
    );
    // allowedTools is an unknown key; validator should name it in the error
    assert!(
        rendered.contains("allowedTools"),
        "error should name the offending field, got: {rendered}"
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn validates_wrong_type_for_known_field_with_field_path() {
    // given
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    let user_settings = home.join("settings.json");
    fs::create_dir_all(&home).expect("home config dir");
    fs::create_dir_all(&cwd).expect("project dir");
    fs::write(
        &user_settings,
        "{\n  \"hooks\": {\n    \"PreToolUse\": \"not-an-array\"\n  }\n}\n",
    )
    .expect("write user settings");

    // when
    let error = ConfigLoader::new(&cwd, &home)
        .load()
        .expect_err("config should fail");

    // then
    let rendered = error.to_string();
    assert!(
        rendered.contains(&user_settings.display().to_string()),
        "error should include file path, got: {rendered}"
    );
    assert!(
        rendered.contains("hooks"),
        "error should include field path component 'hooks', got: {rendered}"
    );
    assert!(
        rendered.contains("PreToolUse"),
        "error should describe the type mismatch, got: {rendered}"
    );
    assert!(
        rendered.contains("array"),
        "error should describe the expected type, got: {rendered}"
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn unknown_top_level_key_suggests_closest_match() {
    // given
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    let user_settings = home.join("settings.json");
    fs::create_dir_all(&home).expect("home config dir");
    fs::create_dir_all(&cwd).expect("project dir");
    fs::write(&user_settings, "{\n  \"modle\": \"opus\"\n}\n").expect("write user settings");

    // when
    let error = ConfigLoader::new(&cwd, &home)
        .load()
        .expect_err("config should fail");

    // then
    let rendered = error.to_string();
    assert!(
        rendered.contains("modle"),
        "error should name the offending field, got: {rendered}"
    );
    assert!(
        rendered.contains("model"),
        "error should suggest the closest known key, got: {rendered}"
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}
