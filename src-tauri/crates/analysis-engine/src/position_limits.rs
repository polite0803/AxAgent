/// 风险档位（与 portfolio_monitor::compute_risk_level 对齐）
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RiskTier {
    /// 低风险
    Low,
    /// 中风险
    Medium,
    /// 中高风险
    MediumHigh,
    /// 高风险
    High,
    /// 极高风险
    Extreme,
}

impl RiskTier {
    /// 该风险档位下的单股仓位上限（%）
    pub fn max_single_stock_pct(self) -> f64 {
        match self {
            RiskTier::Low => 20.0,
            RiskTier::Medium => 20.0,
            RiskTier::MediumHigh => 20.0,
            // 修复 H5: 高风险 35% 上限
            RiskTier::High => 35.0,
            // 极高风险强制观望（实际不允许开新仓）
            RiskTier::Extreme => 0.0,
        }
    }

    /// 是否禁止开新仓
    pub fn forbid_new_position(self) -> bool {
        matches!(self, RiskTier::Extreme)
    }

    /// 修复 H7: 统一风险分级归一化函数
    /// 将三套口径的字符串统一映射到 RiskTier:
    /// - decision.rs: 低/中/高（3 级）
    /// - portfolio_monitor.rs: 低/中/中高/高/无持仓（5 级）
    /// - backtest_strategy.rs: 极高/高/低（3 级混用）
    pub fn from_risk_str(s: &str) -> Self {
        let s = s.trim();
        if s.contains("极高") || s.contains("极高风险") {
            RiskTier::Extreme
        } else if s.contains("中高") {
            RiskTier::MediumHigh
        } else if s.contains("高") || s.eq_ignore_ascii_case("high") {
            RiskTier::High
        } else if s.contains("中") || s.eq_ignore_ascii_case("medium") {
            RiskTier::Medium
        } else if s.contains("低") || s.eq_ignore_ascii_case("low") {
            RiskTier::Low
        } else {
            // "无持仓" 或未知 → 默认中风险（保守不激进）
            RiskTier::Medium
        }
    }

    /// 转中文显示
    pub fn to_cn(self) -> &'static str {
        match self {
            RiskTier::Low => "低风险",
            RiskTier::Medium => "中风险",
            RiskTier::MediumHigh => "中高风险",
            RiskTier::High => "高风险",
            RiskTier::Extreme => "极高风险",
        }
    }
}

/// 全局仓位限制配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionLimits {
    pub max_single_stock_pct: f64,
    pub max_total_positions: u32,
    pub max_sector_exposure_pct: f64,
}

impl Default for PositionLimits {
    fn default() -> Self {
        Self { max_single_stock_pct: 20.0, max_total_positions: 10, max_sector_exposure_pct: 40.0 }
    }
}

/// 空头预测阈值：targetPrice < current × 0.85 视为强烈看空，应强制卖出
pub const BEARISH_TARGET_PRICE_RATIO: f64 = 0.85;

impl PositionLimits {
    /// 检查新增仓位是否合规
    ///
    /// 修复 P2-9: 当 `total_portfolio_value == 0` 时原代码把 new_pct 置为 0，
    /// 静默绕过单股仓位与行业暴露上限检查。这在"空仓首次建仓"场景下形成风控漏洞
    /// —— 任意金额的买单都会"合规"。改为明确拒绝，迫使 caller 传入含现金的
    /// 组合总价值（持仓市值 + 可用现金），让仓位上限检查真实生效。
    pub fn check_new_position(
        &self,
        new_position_value: f64,
        total_portfolio_value: f64,
        current_positions: usize,
        new_sector: Option<&str>,
        current_sector_exposures: &[(String, f64)],
    ) -> Result<(), String> {
        if total_portfolio_value <= 0.0 {
            return Err(format!(
                "组合总价值为 {}，无法计算仓位比例（请传入 持仓市值+可用现金 作为分母）",
                total_portfolio_value
            ));
        }

        if let Some(sector) = new_sector {
            let current_sector_pct = current_sector_exposures
                .iter()
                .filter(|(s, _)| s == sector)
                .map(|(_, pct)| *pct)
                .next()
                .unwrap_or(0.0);
            let new_pct = (new_position_value / total_portfolio_value) * 100.0;
            if current_sector_pct + new_pct > self.max_sector_exposure_pct {
                return Err(format!(
                    "行业{}暴露{:.1}%将超过上限{:.0}%",
                    sector,
                    current_sector_pct + new_pct,
                    self.max_sector_exposure_pct
                ));
            }
        }

        if current_positions >= self.max_total_positions as usize {
            return Err(format!(
                "持仓数量已达上限 ({}只)，请先减仓再新增",
                self.max_total_positions
            ));
        }

        let new_pct = (new_position_value / total_portfolio_value) * 100.0;

        if new_pct > self.max_single_stock_pct {
            return Err(format!(
                "单股仓位 {:.1}% 超过上限 {:.0}%，请减少买入数量",
                new_pct, self.max_single_stock_pct
            ));
        }

        Ok(())
    }

    /// 修复 H5: 风险档位感知的仓位检查
    /// - 极高风险档位直接拒绝开新仓（强制观望）
    /// - 高风险档位使用 35% 单股上限（覆盖 max_single_stock_pct）
    /// - 其余档位沿用 max_single_stock_pct
    pub fn check_new_position_with_risk(
        &self,
        new_position_value: f64,
        total_portfolio_value: f64,
        current_positions: usize,
        new_sector: Option<&str>,
        current_sector_exposures: &[(String, f64)],
        risk_tier: RiskTier,
    ) -> Result<(), String> {
        if risk_tier.forbid_new_position() {
            return Err("风险档位为极高，强制观望，禁止开新仓".to_string());
        }
        // 临时覆盖单股上限为风险档位上限（取较小者）
        let original = self.max_single_stock_pct;
        let tier_cap = risk_tier.max_single_stock_pct();
        let effective_cap = original.min(tier_cap);
        let capped = PositionLimits {
            max_single_stock_pct: effective_cap,
            max_total_positions: self.max_total_positions,
            max_sector_exposure_pct: self.max_sector_exposure_pct,
        };
        capped.check_new_position(
            new_position_value,
            total_portfolio_value,
            current_positions,
            new_sector,
            current_sector_exposures,
        )
    }

    /// 修复 H5: 空头预测强制卖出检查
    /// 当 target_price < current_price × 0.85 时，应强制卖出该持仓
    pub fn check_bearish_force_sell(
        current_price: f64,
        target_price: Option<f64>,
    ) -> Result<bool, String> {
        match target_price {
            Some(tp) if tp > 0.0 && current_price > 0.0 => {
                let ratio = tp / current_price;
                if ratio < BEARISH_TARGET_PRICE_RATIO {
                    // 强制卖出信号
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
            _ => Ok(false),
        }
    }
}
