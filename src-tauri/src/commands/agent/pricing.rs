// SPDX-License-Identifier: AGPL-3.0-only

//! Pricing configuration and cost estimation.
//!
//! Loaded from pricing.toml at startup with heuristic fallback for
//! unrecognized model variants.

use crate::commands::error::ErrorResponse;
use crate::commands::error_code::agent as agent_err;
use serde::Deserialize;
use std::sync::OnceLock;
use std::sync::atomic::AtomicBool;

#[derive(Debug, Clone, Deserialize)]
pub(super) struct PricingModel {
    pub(super) model_id: String,
    #[serde(default)]
    pub(super) aliases: Vec<String>,
    pub(super) input_price: f64,
    pub(super) output_price: f64,
    #[serde(default)]
    pub(super) tier: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct PricingConfigFile {
    #[serde(default)]
    pub(super) budget: BudgetConfig,
    pub(super) models: Vec<PricingModel>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub(super) struct BudgetConfig {
    #[serde(default)]
    pub(super) max_tokens_per_turn: u64,
    #[serde(default)]
    pub(super) max_cost_per_day_usd: f64,
    #[serde(default)]
    pub(super) max_cost_per_session_usd: f64,
}

/// Cached pricing config loaded at startup.
static PRICING_CONFIG: OnceLock<PricingConfigFile> = OnceLock::new();

/// M6: Whether pricing.toml was successfully loaded (set to false on fallback).
static PRICING_AVAILABLE: AtomicBool = AtomicBool::new(true);

/// Look up pricing from the loaded config. Returns (input_price, output_price) per million tokens.
pub(super) fn lookup_pricing_from_config(model_id: &str) -> Option<(f64, f64)> {
    let config = PRICING_CONFIG.get()?;
    for m in &config.models {
        if m.model_id == model_id || m.aliases.iter().any(|a| a == model_id) {
            let _ = &m.tier;
            return Some((m.input_price, m.output_price));
        }
    }
    None
}

/// Check if a turn would exceed the per-turn token budget.
/// Returns Ok(()) if within budget, Err(message) if exceeded.
pub(super) fn check_token_budget(input_tokens: u64) -> Result<(), String> {
    let config = PRICING_CONFIG.get();
    let max_tokens = config.map(|c| c.budget.max_tokens_per_turn).unwrap_or(0);
    if max_tokens > 0 && input_tokens > max_tokens {
        return Err(format!(
            "Token budget exceeded: {} input tokens > {} max per turn. \
             Consider reducing context, compressing history, or increasing the budget in pricing.toml.",
            input_tokens, max_tokens
        ));
    }
    Ok(())
}

/// Heuristic pricing for unrecognized model variants.
/// Uses model name patterns to estimate a reasonable price tier.
pub(super) fn heuristic_pricing(model_id: &str) -> Option<(f64, f64)> {
    let lower = model_id.to_lowercase();
    // Nano/tiny models — cheapest tier
    if lower.contains("nano") || lower.contains("tiny") {
        return Some((0.10, 0.40));
    }
    // Mini/small/flash/haiku — budget tier
    if lower.contains("mini")
        || lower.contains("small")
        || lower.contains("flash")
        || lower.contains("haiku")
        || lower.contains("turbo")
    {
        return Some((0.15, 0.60));
    }
    // Pro/sonnet/plus — mid tier
    if lower.contains("pro")
        || lower.contains("sonnet")
        || lower.contains("plus")
        || lower.contains("4o")
        || lower.contains("4.1")
    {
        return Some((2.50, 10.00));
    }
    // Opus/o1/o3 — premium tier
    if lower.contains("opus")
        || lower.starts_with("o1")
        || lower.starts_with("o3")
        || lower.starts_with("o4")
    {
        return Some((15.00, 60.00));
    }
    // DeepSeek/Qwen — budget tier
    if lower.contains("deepseek") || lower.contains("qwen") {
        return Some((0.27, 1.10));
    }
    // Default: mid tier for completely unknown models
    Some((2.50, 10.00))
}

/// Initialize pricing from the config file. Called once during app startup.
pub fn init_pricing_config(app: &tauri::AppHandle) {
    let config = load_pricing_from_disk(app).unwrap_or_else(|e| {
        tracing::warn!("Failed to load pricing.toml, using heuristic fallback: {}", e);
        PRICING_AVAILABLE.store(false, std::sync::atomic::Ordering::Release);
        PricingConfigFile {
            budget: BudgetConfig::default(),
            models: Vec::new(),
        }
    });
    let _ = PRICING_CONFIG.set(config);
}

/// M6: Expose pricing availability status to the UI layer.
/// Returns true if pricing.toml was loaded successfully, false if heuristic fallback is active.
#[tauri::command]
pub fn is_pricing_available() -> bool {
    PRICING_AVAILABLE.load(std::sync::atomic::Ordering::Acquire)
}

fn load_pricing_from_disk(app_handle: &tauri::AppHandle) -> Result<PricingConfigFile, String> {
    use std::fs;
    use tauri::Manager;
    let resource_dir = app_handle.path().resource_dir().map_err(|e| {
        ErrorResponse::new(agent_err::INTERNAL)
            .with_detail(format!("Failed to get resource dir: {}", e))
    })?;
    let mut path = resource_dir.join("pricing.toml");
    // Also check next to the executable (production fallback)
    if !path.exists() {
        let exe_dir = std::env::current_exe()
            .map_err(|e| {
                ErrorResponse::new(agent_err::INTERNAL)
                    .with_detail(format!("Failed to get exe dir: {}", e))
            })?
            .parent()
            .ok_or("No exe parent dir")?
            .to_path_buf();
        path = exe_dir.join("pricing.toml");
    }
    // Development fallback: check CARGO_MANIFEST_DIR baked at compile time
    if !path.exists() {
        let dev_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("pricing.toml");
        if dev_path.exists() {
            path = dev_path;
        }
    }
    let content = fs::read_to_string(&path).map_err(|e| {
        ErrorResponse::new(agent_err::INTERNAL).with_detail(format!(
            "Failed to read {}: {}",
            path.display(),
            e
        ))
    })?;
    let config: PricingConfigFile = toml::from_str(&content).map_err(|e| {
        ErrorResponse::new(agent_err::INTERNAL)
            .with_detail(format!("Failed to parse pricing.toml: {}", e))
    })?;
    tracing::info!(
        "Loaded pricing config with {} models, budget: tokens={}, daily=${}, session=${}",
        config.models.len(),
        config.budget.max_tokens_per_turn,
        config.budget.max_cost_per_day_usd,
        config.budget.max_cost_per_session_usd,
    );
    Ok(config)
}

/// Estimate cost in USD using model price fields (highest priority), then
/// pricing.toml config, then heuristic fallback.
pub(super) fn estimate_cost_usd(
    model_id: &str,
    input_tokens: u64,
    output_tokens: u64,
    model_input_price: Option<f64>,
    model_output_price: Option<f64>,
) -> Option<f64> {
    // 1. Model's own price fields — synced from provider or user-configured
    if let (Some(inp), Some(out)) = (model_input_price, model_output_price) {
        return Some(
            (input_tokens as f64 * inp / 1_000_000.0) + (output_tokens as f64 * out / 1_000_000.0),
        );
    }
    // 2. pricing.toml config
    if let Some((inp, out)) = lookup_pricing_from_config(model_id) {
        return Some(
            (input_tokens as f64 * inp / 1_000_000.0) + (output_tokens as f64 * out / 1_000_000.0),
        );
    }
    // 3. Heuristic fallback
    let (inp, out) = heuristic_pricing(model_id)?;
    Some((input_tokens as f64 * inp / 1_000_000.0) + (output_tokens as f64 * out / 1_000_000.0))
}
