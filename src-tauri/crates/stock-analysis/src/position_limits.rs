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
}
