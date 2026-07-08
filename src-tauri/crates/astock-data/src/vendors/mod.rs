use crate::as_of_capability::AsOfCapability;
use crate::error::DataError;
use crate::types::*;
use async_trait::async_trait;

#[async_trait]
pub trait StockVendor: Send + Sync {
    // ─── Live 模式方法(vendor 默认实现就是"调用实时 API") ───
    // as-of 模式下,lib.rs 路由层读 current_asof() + 查 asof_capability() 决策
    // 是否改用对应的 _with_asof 方法

    async fn get_quote(&self, stock_code: &str) -> Result<StockQuote, DataError>;

    async fn get_klines(
        &self,
        stock_code: &str,
        period: &str,
        limit: u32,
        _adj: Option<AdjType>,
    ) -> Result<Vec<KLine>, DataError>;

    async fn get_financials(&self, stock_code: &str) -> Result<Vec<FinancialReport>, DataError>;

    async fn get_news(&self, stock_code: &str, limit: u32) -> Result<Vec<NewsItem>, DataError>;

    /// 按关键词搜索新闻（用于验证 CapEx/催化剂/行业趋势）
    async fn search_news(&self, keyword: &str, limit: u32) -> Result<Vec<NewsItem>, DataError> {
        let (_keyword, _limit) = (keyword, limit);
        Ok(vec![])
    }

    async fn get_money_flow(&self, stock_code: &str) -> Result<Option<MoneyFlow>, DataError>;

    async fn get_dragon_tiger(&self, stock_code: &str) -> Result<Vec<DragonTigerEntry>, DataError>;

    async fn get_lockup_schedule(&self, stock_code: &str)
        -> Result<Vec<LockupSchedule>, DataError>;

    async fn search_stock(&self, keyword: &str) -> Result<Vec<StockSearchResult>, DataError>;

    /// 获取融资融券数据
    async fn get_margin_data(&self, stock_code: &str) -> Result<Option<MarginData>, DataError> {
        let _ = stock_code;
        Ok(None)
    }

    /// 获取北向资金持仓
    async fn get_north_bound_holding(
        &self,
        stock_code: &str,
    ) -> Result<Option<NorthBoundHolding>, DataError> {
        let _ = stock_code;
        Ok(None)
    }

    /// 获取行业分类
    async fn get_sector_info(&self, stock_code: &str) -> Result<Option<SectorInfo>, DataError> {
        let _ = stock_code;
        Ok(None)
    }

    /// 获取股东增减持
    async fn get_shareholder_trades(
        &self,
        stock_code: &str,
    ) -> Result<Vec<ShareholderTrade>, DataError> {
        let _ = stock_code;
        Ok(vec![])
    }

    /// 获取除权除息记录
    async fn get_dividend_records(
        &self,
        stock_code: &str,
    ) -> Result<Vec<DividendRecord>, DataError> {
        let _ = stock_code;
        Ok(vec![])
    }

    async fn get_research_reports(
        &self,
        stock_code: &str,
    ) -> Result<Vec<ResearchReport>, DataError> {
        let _ = stock_code;
        Ok(vec![])
    }

    async fn get_consensus_eps(&self, stock_code: &str) -> Result<Option<ConsensusEPS>, DataError> {
        let _ = stock_code;
        Ok(None)
    }

    async fn get_concept_blocks(
        &self,
        stock_code: &str,
    ) -> Result<Option<ConceptBlocks>, DataError> {
        let _ = stock_code;
        Ok(None)
    }

    async fn get_announcements(&self, stock_code: &str) -> Result<Vec<Announcement>, DataError> {
        let _ = stock_code;
        Ok(vec![])
    }

    async fn get_market_dragon_tiger(&self) -> Result<Vec<MarketDragonTiger>, DataError> {
        Ok(vec![])
    }

    async fn get_hot_stocks(&self) -> Result<Vec<HotStock>, DataError> {
        Ok(vec![])
    }

    async fn get_industry_ranking(&self) -> Result<Vec<IndustryRank>, DataError> {
        Ok(vec![])
    }

    async fn get_cls_flash(&self) -> Result<Vec<ClsFlashItem>, DataError> {
        Ok(vec![])
    }

    async fn get_north_bound_flow(&self) -> Result<Option<NorthBoundFlow>, DataError> {
        Ok(None)
    }

    /// 获取大宗交易记录
    async fn get_block_trades(&self, stock_code: &str) -> Result<Vec<BlockTrade>, DataError> {
        let _ = stock_code;
        Ok(vec![])
    }

    /// 获取机构调研记录
    async fn get_institutional_visits(
        &self,
        stock_code: &str,
    ) -> Result<Vec<InstitutionalVisit>, DataError> {
        let _ = stock_code;
        Ok(vec![])
    }

    /// 获取大盘指数行情（上证/深证/创业板）
    async fn get_index_quotes(&self) -> Result<Vec<IndexQuote>, DataError> {
        Ok(vec![])
    }

    /// 获取同行业可比公司估值
    async fn get_peers(&self, stock_code: &str) -> Result<Vec<PeerComparison>, DataError> {
        let _ = stock_code;
        Ok(vec![])
    }

    /// 获取期权PCR（看跌/看涨比率）
    async fn get_option_pcr(&self, stock_code: &str) -> Result<Option<OptionPCR>, DataError> {
        let _ = stock_code;
        Ok(None)
    }

    // ─── As-Of 能力申报 + 内部 as-of 数据获取 ───
    // vendor trait 大重构 §2.2
    //
    // 默认实现:不申报(返回 Fallthrough)+ 默认 with_asof = 调原方法
    // 这样老 vendor 零改动可继续工作(via lib.rs 截断兜底)
    // 新 vendor 重写 asof_capability() 声明自己能力 + 重写对应 with_asof() 实现真正的历史切片

    /// vendor 声明自己 (method) 的 as-of 处理能力
    /// 默认 Fallthrough(走 lib.rs "全量 + 截断" 兜底)
    ///
    /// 注:虽然能力是按类型声明的(理论上可以 `where Self: Sized` 静态),
    /// 但 Rust trait object (dyn StockVendor) 不支持静态方法,
    /// 所以加 &self 让它能通过 trait object 调用。vendor override 时函数体
    /// 完全不依赖 self,实例开销可忽略。
    fn asof_capability(&self, method: &str) -> AsOfCapability {
        let _ = method;
        AsOfCapability::Fallthrough
    }

    // ─── _with_asof 默认方法(vendor 按需 override) ───
    // 默认实现 = 调原方法,保持向后兼容

    async fn get_quote_with_asof(&self, stock_code: &str) -> Result<StockQuote, DataError> {
        self.get_quote(stock_code).await
    }

    async fn get_klines_with_asof(
        &self,
        stock_code: &str,
        period: &str,
        limit: u32,
        adj: Option<AdjType>,
    ) -> Result<Vec<KLine>, DataError> {
        self.get_klines(stock_code, period, limit, adj).await
    }

    async fn get_financials_with_asof(
        &self,
        stock_code: &str,
    ) -> Result<Vec<FinancialReport>, DataError> {
        self.get_financials(stock_code).await
    }

    async fn get_news_with_asof(
        &self,
        stock_code: &str,
        limit: u32,
    ) -> Result<Vec<NewsItem>, DataError> {
        self.get_news(stock_code, limit).await
    }

    async fn get_money_flow_with_asof(
        &self,
        stock_code: &str,
    ) -> Result<Option<MoneyFlow>, DataError> {
        self.get_money_flow(stock_code).await
    }

    async fn get_dragon_tiger_with_asof(
        &self,
        stock_code: &str,
    ) -> Result<Vec<DragonTigerEntry>, DataError> {
        self.get_dragon_tiger(stock_code).await
    }

    async fn get_lockup_schedule_with_asof(
        &self,
        stock_code: &str,
    ) -> Result<Vec<LockupSchedule>, DataError> {
        self.get_lockup_schedule(stock_code).await
    }

    async fn search_stock_with_asof(
        &self,
        keyword: &str,
    ) -> Result<Vec<StockSearchResult>, DataError> {
        self.search_stock(keyword).await
    }

    async fn get_margin_data_with_asof(
        &self,
        stock_code: &str,
    ) -> Result<Option<MarginData>, DataError> {
        self.get_margin_data(stock_code).await
    }

    async fn get_north_bound_holding_with_asof(
        &self,
        stock_code: &str,
    ) -> Result<Option<NorthBoundHolding>, DataError> {
        self.get_north_bound_holding(stock_code).await
    }

    async fn get_sector_info_with_asof(
        &self,
        stock_code: &str,
    ) -> Result<Option<SectorInfo>, DataError> {
        self.get_sector_info(stock_code).await
    }

    async fn get_shareholder_trades_with_asof(
        &self,
        stock_code: &str,
    ) -> Result<Vec<ShareholderTrade>, DataError> {
        self.get_shareholder_trades(stock_code).await
    }

    async fn get_dividend_records_with_asof(
        &self,
        stock_code: &str,
    ) -> Result<Vec<DividendRecord>, DataError> {
        self.get_dividend_records(stock_code).await
    }

    async fn get_research_reports_with_asof(
        &self,
        stock_code: &str,
    ) -> Result<Vec<ResearchReport>, DataError> {
        self.get_research_reports(stock_code).await
    }

    async fn get_consensus_eps_with_asof(
        &self,
        stock_code: &str,
    ) -> Result<Option<ConsensusEPS>, DataError> {
        self.get_consensus_eps(stock_code).await
    }

    async fn get_concept_blocks_with_asof(
        &self,
        stock_code: &str,
    ) -> Result<Option<ConceptBlocks>, DataError> {
        self.get_concept_blocks(stock_code).await
    }

    async fn get_announcements_with_asof(
        &self,
        stock_code: &str,
    ) -> Result<Vec<Announcement>, DataError> {
        self.get_announcements(stock_code).await
    }

    async fn get_market_dragon_tiger_with_asof(&self) -> Result<Vec<MarketDragonTiger>, DataError> {
        self.get_market_dragon_tiger().await
    }

    async fn get_hot_stocks_with_asof(&self) -> Result<Vec<HotStock>, DataError> {
        self.get_hot_stocks().await
    }

    async fn get_industry_ranking_with_asof(&self) -> Result<Vec<IndustryRank>, DataError> {
        self.get_industry_ranking().await
    }

    async fn get_cls_flash_with_asof(&self) -> Result<Vec<ClsFlashItem>, DataError> {
        self.get_cls_flash().await
    }

    async fn get_north_bound_flow_with_asof(&self) -> Result<Option<NorthBoundFlow>, DataError> {
        self.get_north_bound_flow().await
    }

    async fn get_block_trades_with_asof(
        &self,
        stock_code: &str,
    ) -> Result<Vec<BlockTrade>, DataError> {
        self.get_block_trades(stock_code).await
    }

    async fn get_institutional_visits_with_asof(
        &self,
        stock_code: &str,
    ) -> Result<Vec<InstitutionalVisit>, DataError> {
        self.get_institutional_visits(stock_code).await
    }

    async fn get_index_quotes_with_asof(&self) -> Result<Vec<IndexQuote>, DataError> {
        self.get_index_quotes().await
    }

    async fn get_peers_with_asof(
        &self,
        stock_code: &str,
    ) -> Result<Vec<PeerComparison>, DataError> {
        self.get_peers(stock_code).await
    }

    async fn get_option_pcr_with_asof(
        &self,
        stock_code: &str,
    ) -> Result<Option<OptionPCR>, DataError> {
        self.get_option_pcr(stock_code).await
    }
}

pub mod akshare;
pub mod baidu_stock;
pub mod browser_eastmoney;
pub mod cninfo;
pub mod eastmoney;
pub mod iwencai;
pub mod mootdx;
pub mod neodata;
pub mod sina;
pub mod tencent;
pub mod ths;
pub mod xueqiu;
