// SPDX-License-Identifier: AGPL-3.0-only

//! axagent-kit — 通用工具集
//!
//! 包含浏览器自动化、HTML 清洗、操作审计、提示模板等零散模块。

pub mod browser_automation;
pub mod command_validator;
#[cfg(not(target_os = "android"))]
pub mod computer_control;
#[cfg(target_os = "android")]
pub mod computer_control;
pub mod git_tools;
pub mod html_cleaner;
pub mod memory_forgetting;
pub mod model_knowledge;
pub mod operation_audit;
pub mod output_processor;
pub mod prompt_template;
pub mod resource_limits;
pub mod sandbox_runner;
pub mod schema_validator;
pub mod screen_capture;
pub mod service_container;
pub mod shell_parser;
pub mod token_budget;
pub mod token_counter;
pub mod unified_config;
pub mod utils;

#[cfg(not(target_os = "android"))]
pub mod ui_automation;
#[cfg(target_os = "android")]
pub mod ui_automation;

pub mod billing;
pub mod markdown_parser;
pub mod marketplace;
pub mod marketplace_service;
pub mod plan_compiler;
pub mod preset_templates;
pub mod prompts;
pub mod screen_vision;
pub mod secure_store;
pub mod skill_dirs;
pub mod slash_command;

#[cfg(not(target_os = "android"))]
pub mod workflow_version;
#[cfg(target_os = "android")]
pub mod workflow_version;
