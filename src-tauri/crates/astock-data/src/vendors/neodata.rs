#![allow(non_snake_case)]

//! NeoData Financial Search vendor
//!
//! 通过 NeoData 自然语言 API 获取金融数据，覆盖美股/日股/韩股/宏观经济/外汇/期货等
//! 现有 12 个 vendor 未覆盖的新品类。作为末位 fallback vendor 使用。
//!
//! 调用方式：通过 Python 脚本 `query.py` 调用 NeoData HTTP API
//! 响应格式：JSON，结构化数据在 apiData.apiRecall[].content（文本），
//! 文档数据在 docData.docRecall（文章/新闻）。

use crate::error::DataError;
use crate::types::*;
use crate::vendors::StockVendor;
use async_trait::async_trait;
use serde_json::Value;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::sync::RwLock;

/// NeoData 脚本路径常量
const NEODATA_SCRIPT: &str = "scripts/query.py";

/// Python 命令（优先使用 managed python，回退到系统 python）
fn python_cmd() -> &'static str {
    if cfg!(target_os = "windows") {
        "python"
    } else {
        "python3"
    }
}

/// 执行 NeoData 查询并返回 JSON 响应
async fn neodata_query(query: &str, token: Option<&str>) -> Result<Value, DataError> {
    // 构造脚本路径（优先工作目录，回退到 skill 目录）
    let script_path = find_script();
    let mut cmd = tokio::process::Command::new(python_cmd());
    cmd.arg(&script_path)
        .arg("--query")
        .arg(query)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // 如果有 token，传入 --token 参数（让 Python 脚本使用而非读缓存文件）
    if let Some(t) = token {
        if !t.is_empty() {
            cmd.arg("--token").arg(t);
        }
    }
    let output = cmd.output().await.map_err(|e| DataError::VendorError {
        vendor: "neodata".into(),
        message: format!("Python 执行失败: {e}"),
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // 检查 TOKEN_EXPIRED/TOKEN_MISSING 信号
        let out = stdout.to_string();
        if out.contains("TOKEN_EXPIRED") || out.contains("TOKEN_MISSING") {
            return Err(DataError::VendorError {
                vendor: "neodata".into(),
                message: "NeoData 凭证过期或缺失，需要重新获取凭证".into(),
            });
        }
        return Err(DataError::VendorError {
            vendor: "neodata".into(),
            message: format!("脚本失败: {stderr}"),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).map_err(|e| {
        DataError::ParseError(format!(
            "NeoData JSON 解析失败: {e}, raw={}",
            &stdout.chars().take(500).collect::<String>()
        ))
    })?;

    // 检查错误码
    if let Some(code) = json["code"].as_str() {
        match code {
            "200" => {}, // 成功
            "1001" | "1006" => {
                return Err(DataError::VendorError {
                    vendor: "neodata".into(),
                    message: format!(
                        "NeoData 未命中意图: {}",
                        json["msg"].as_str().unwrap_or("unknown")
                    ),
                });
            },
            _ => {
                return Err(DataError::VendorError {
                    vendor: "neodata".into(),
                    message: format!(
                        "NeoData 错误: code={code}, msg={}",
                        json["msg"].as_str().unwrap_or("")
                    ),
                });
            },
        }
    }

    Ok(json)
}

/// 查找 query.py 脚本路径
fn find_script() -> PathBuf {
    // 优先从当前工作目录查找
    let cwd = std::env::current_dir().ok();
    if let Some(dir) = &cwd {
        let p = dir.join(NEODATA_SCRIPT);
        if p.exists() {
            return p;
        }
    }
    // fallback: WorkBuddy skill 目录
    let fallback = PathBuf::from(
        r"C:\Users\polit\AppData\Local\Programs\WorkBuddy\resources\app.asar.unpacked\resources\builtin-skills\neodata-financial-search",
    )
    .join(NEODATA_SCRIPT);
    if fallback.exists() {
        return fallback;
    }
    // 最终 fallback：用相对路径，让 Python 自行查找
    PathBuf::from(NEODATA_SCRIPT)
}

/// 从 apiData.apiRecall 中提取指定 type 的 content 文本
fn extract_api_content(data: &Value, data_type: &str) -> Option<String> {
    let recall = data["apiData"]["apiRecall"].as_array()?;
    for item in recall {
        if item["type"].as_str()? == data_type {
            return item["content"].as_str().map(|s| s.to_string());
        }
    }
    None
}

/// 解析 NeoData 文本行格式 "key:value" 或 "【标题】\nkey:value"
fn parse_kv_text(text: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(pos) = line.find(':') {
            let key = line[..pos].trim().to_string();
            let val = line[pos + 1..].trim().to_string();
            if !key.is_empty() {
                pairs.push((key, val));
            }
        }
    }
    pairs
}

/// 从 kv 列表中提取指定 key 的 f64 值
fn kv_f64(pairs: &[(String, String)], key: &str) -> Option<f64> {
    pairs
        .iter()
        .find(|(k, _v)| k.contains(key))
        .and_then(|(_, v)| {
            // 处理 "+0.56%" 格式
            let cleaned = v.replace(['%', ','], "");
            cleaned.parse::<f64>().ok()
        })
}

/// 从 kv 列表中提取指定 key 的字符串值
fn kv_str<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    pairs
        .iter()
        .find(|(k, _)| k.contains(key))
        .map(|(_, v)| v.as_str())
}

/// 从文本中提取股票代码（"股票代码:600519" 格式）
fn extract_code(text: &str) -> Option<String> {
    for prefix in &["股票代码:", "股票代码：", "代码:", "代码："] {
        if let Some(pos) = text.find(prefix) {
            let start = pos + prefix.len();
            let code_part = &text[start..];
            let end = code_part
                .find(|c: char| !c.is_alphanumeric() && c != '.' && c != '-')
                .unwrap_or(code_part.len());
            let code = code_part[..end].trim();
            if !code.is_empty() {
                return Some(code.to_string());
            }
        }
    }
    None
}

/// 从文本中提取股票名称
fn extract_name(text: &str) -> Option<String> {
    for prefix in &["股票名称:", "股票名称：", "名称:", "名称："] {
        if let Some(pos) = text.find(prefix) {
            let start = pos + prefix.len();
            let name_part = &text[start..];
            let end = name_part.find(['\n', '\r']).unwrap_or(name_part.len());
            let name = name_part[..end].trim();
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }
    None
}

// ─── NeoDataVendor ───

/// NeoData 数据源 vendor
///
/// 通过 Python 脚本调用 NeoData API。
/// **不做 as-of 声明**（默认 Fallthrough），利用 lib.rs 的截断兜底。
/// 放在 vendor 路由末位作为补充数据源。
pub struct NeoDataVendor {
    pub token: Arc<RwLock<String>>,
}

impl NeoDataVendor {
    /// 获取当前 token（可能为空）
    fn get_token(&self) -> String {
        self.token.try_read().map(|t| t.clone()).unwrap_or_default()
    }

    /// 带 token 的 NeoData 查询包装
    async fn nd_query(&self, query: &str) -> Result<Value, DataError> {
        let token = self.get_token();
        neodata_query(query, Some(&token)).await
    }

    /// 辅助：执行查询并提取指定 type 的文本内容
    async fn query_content(&self, query: &str, data_type: &str) -> Result<String, DataError> {
        let json = self.nd_query(query).await?;
        extract_api_content(&json, data_type).ok_or_else(|| {
            let types: Vec<&str> = json["apiData"]["apiRecall"]
                .as_array()
                .map(|a| a.iter().filter_map(|i| i["type"].as_str()).collect())
                .unwrap_or_default();
            tracing::warn!("[neodata] 未找到 type={data_type}，可用类型: {:?}", types);
            DataError::VendorError {
                vendor: "neodata".into(),
                message: format!("NeoData 未返回 {data_type} 数据"),
            }
        })
    }
}

#[async_trait]
impl StockVendor for NeoDataVendor {
    async fn get_quote(&self, stock_code: &str) -> Result<StockQuote, DataError> {
        let query = format!("{}最新股价行情", stock_code);
        let text = self.query_content(&query, "basic_info").await?;
        let pairs = parse_kv_text(&text);

        let price = kv_f64(&pairs, "最新价")
            .or_else(|| kv_f64(&pairs, "价格"))
            .unwrap_or(0.0);
        let pre_close = kv_f64(&pairs, "昨收").unwrap_or(0.0);
        let open = kv_f64(&pairs, "今开")
            .or_else(|| kv_f64(&pairs, "开盘"))
            .unwrap_or(0.0);
        let high = kv_f64(&pairs, "最高").unwrap_or(0.0);
        let low = kv_f64(&pairs, "最低").unwrap_or(0.0);
        let change_pct = kv_f64(&pairs, "涨跌幅")
            .or_else(|| kv_f64(&pairs, "涨跌"))
            .unwrap_or(0.0);
        let volume = kv_f64(&pairs, "成交量")
            .or_else(|| kv_f64(&pairs, "成交"))
            .unwrap_or(0.0);
        let amount = kv_f64(&pairs, "成交额").unwrap_or(0.0);
        let turnover_rate = kv_f64(&pairs, "换手率");
        let pe = kv_f64(&pairs, "PE").or_else(|| kv_f64(&pairs, "市盈率"));
        let pb = kv_f64(&pairs, "PB").or_else(|| kv_f64(&pairs, "市净率"));

        let name = extract_name(&text).unwrap_or_default();
        let code = extract_code(&text).unwrap_or_else(|| stock_code.to_string());

        Ok(StockQuote {
            code,
            name,
            price,
            pre_close,
            open,
            high,
            low,
            volume,
            amount,
            change_pct,
            turnover_rate: turnover_rate.unwrap_or(0.0) / 100.0, // NeoData 返回百分比值
            pe,
            pb,
            total_mv: None,
            circulating_mv: None,
            limit_up: None,
            limit_down: None,
            is_st: false,
            timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        })
    }

    async fn get_financials(&self, stock_code: &str) -> Result<Vec<FinancialReport>, DataError> {
        let query = format!("{}最新财报数据", stock_code);
        let text = self.query_content(&query, "basic_info").await?;
        let pairs = parse_kv_text(&text);

        let eps = kv_f64(&pairs, "EPS").or_else(|| kv_f64(&pairs, "每股收益"));
        let bps = kv_f64(&pairs, "BPS").or_else(|| kv_f64(&pairs, "每股净资产"));
        let roe = kv_f64(&pairs, "ROE").or_else(|| kv_f64(&pairs, "净资产收益率"));
        let revenue = kv_f64(&pairs, "营收").or_else(|| kv_f64(&pairs, "营业收入"));
        let net_profit = kv_f64(&pairs, "净利润").or_else(|| kv_f64(&pairs, "归母净利润"));
        let gross_margin = kv_f64(&pairs, "毛利率");
        let debt_ratio = kv_f64(&pairs, "负债率").or_else(|| kv_f64(&pairs, "资产负债率"));

        Ok(vec![FinancialReport {
            stock_code: stock_code.to_string(),
            report_date: chrono::Local::now().format("%Y-%m-%d").to_string(),
            revenue,
            net_profit,
            eps,
            bps,
            roe,
            debt_ratio,
            gross_margin,
            net_margin: None,
            revenue_yoy: None,
            profit_yoy: None,
            total_assets: None,
            operating_cash_flow: None,
            capital_expenditure: None,
            free_cash_flow: None,
            current_ratio: None,
            quick_ratio: None,
        }])
    }

    async fn get_news(&self, stock_code: &str, limit: u32) -> Result<Vec<NewsItem>, DataError> {
        let query = format!("{}最新{}条新闻", stock_code, limit.min(20));
        let json = self.nd_query(&query).await?;

        // NeoData 的新闻在 docData.docRecall 中
        let mut items = Vec::new();
        if let Some(recalls) = json["data"]["docData"]["docRecall"].as_array() {
            for group in recalls {
                if let Some(docs) = group["docList"].as_array() {
                    for doc in docs {
                        if items.len() >= limit as usize {
                            break;
                        }
                        items.push(NewsItem {
                            title: doc["title"].as_str().unwrap_or("").to_string(),
                            summary: doc["content"]
                                .as_str()
                                .unwrap_or("")
                                .chars()
                                .take(200)
                                .collect(),
                            source: doc["source"].as_str().unwrap_or("NeoData").to_string(),
                            url: doc["url"].as_str().unwrap_or("").to_string(),
                            publish_time: doc["publishTime"]
                                .as_i64()
                                .map(|ts| {
                                    chrono::DateTime::from_timestamp(ts, 0)
                                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                                        .unwrap_or_default()
                                })
                                .unwrap_or_default(),
                            sentiment_score: None,
                        });
                    }
                }
            }
        }

        if items.is_empty() {
            return Err(DataError::VendorError {
                vendor: "neodata".into(),
                message: "NeoData 新闻返回空".into(),
            });
        }
        Ok(items)
    }

    async fn search_stock(&self, keyword: &str) -> Result<Vec<StockSearchResult>, DataError> {
        let query = format!("搜索{}股票代码", keyword);
        let json = self.nd_query(&query).await?;

        let mut results = Vec::new();
        if let Some(entities) = json["data"]["apiData"]["entity"].as_array() {
            for entity in entities {
                let code = entity["name"].as_str().unwrap_or("");
                let name = entity["code"].as_str().unwrap_or("");
                // code 格式可能是 "00700.HK"，提取纯代码部分
                let pure_code = code.split('.').next().unwrap_or(code);
                results.push(StockSearchResult {
                    code: pure_code.to_string(),
                    name: name.to_string(),
                    market: if code.contains(".HK") {
                        "港股".to_string()
                    } else if code.contains(".US") || code.contains(".PS") {
                        "美股".to_string()
                    } else {
                        "A股".to_string()
                    },
                });
            }
        }

        if results.is_empty() {
            return Err(DataError::VendorError {
                vendor: "neodata".into(),
                message: "NeoData 搜索无结果".into(),
            });
        }
        Ok(results)
    }

    async fn get_hot_stocks(&self) -> Result<Vec<HotStock>, DataError> {
        let query = "今日A股热门股票排名";
        let json = self.nd_query(query).await?;

        let mut stocks = Vec::new();
        if let Some(content) = extract_api_content(&json, "basic_info") {
            // 尝试从文本中解析热门股列表
            for line in content.lines() {
                if let Some(pos) = line.find(|c: char| c.is_ascii_digit()) {
                    let number_part = &line[pos..];
                    let parts: Vec<&str> = number_part.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let change_pct = parts[1].replace('%', "").parse::<f64>().unwrap_or(0.0);
                        stocks.push(HotStock {
                            stock_code: parts[0].to_string(),
                            stock_name: line[..pos].trim().to_string(),
                            change_pct,
                            turnover_rate: None,
                            reason_tags: vec![],
                            sector: None,
                        });
                    }
                }
            }
        }
        // 也尝试从 docData 提取热点板块信息
        if stocks.is_empty() {
            if let Some(content) = extract_api_content(&json, "hot_sector") {
                tracing::debug!(
                    "[neodata] 热门板块数据: {}",
                    content.chars().take(200).collect::<String>()
                );
            }
        }

        if stocks.is_empty() {
            return Err(DataError::VendorError {
                vendor: "neodata".into(),
                message: "NeoData 热门股票数据为空".into(),
            });
        }
        Ok(stocks)
    }

    async fn get_industry_ranking(&self) -> Result<Vec<IndustryRank>, DataError> {
        let query = "今日行业板块涨跌幅排名";
        let json = self.nd_query(query).await?;

        let mut ranks = Vec::new();
        if let Some(content) = extract_api_content(&json, "sector_rank") {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with("【") || trimmed.starts_with("排名")
                {
                    continue;
                }
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 {
                    let change_pct = parts
                        .last()
                        .and_then(|s| s.replace('%', "").parse::<f64>().ok())
                        .unwrap_or(0.0);
                    let name = if parts.len() > 2 {
                        parts[..parts.len() - 1].join(" ")
                    } else {
                        parts[0].to_string()
                    };
                    ranks.push(IndustryRank {
                        industry_name: name,
                        change_pct,
                        turnover: None,
                        main_inflow: None,
                        leader_code: None,
                        leader_name: None,
                        leader_change_pct: None,
                    });
                }
            }
        }

        if ranks.is_empty() {
            return Err(DataError::VendorError {
                vendor: "neodata".into(),
                message: "NeoData 行业排名数据为空".into(),
            });
        }
        Ok(ranks)
    }

    async fn get_index_quotes(&self) -> Result<Vec<IndexQuote>, DataError> {
        let query = "A股主要指数最新行情 上证 深证 创业板";
        let text = self.query_content(query, "index_market").await?;

        let mut indices = Vec::new();
        for line in text.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let price = parts
                    .get(1)
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                let change_pct = parts
                    .last()
                    .and_then(|s| s.replace('%', "").parse::<f64>().ok())
                    .unwrap_or(0.0);
                indices.push(IndexQuote {
                    code: String::new(),
                    name: parts[0].to_string(),
                    price,
                    change_pct,
                    pre_close: 0.0,
                    volume: 0.0,
                    amount: 0.0,
                });
            }
        }

        if indices.is_empty() {
            return Err(DataError::VendorError {
                vendor: "neodata".into(),
                message: "NeoData 指数行情为空".into(),
            });
        }
        Ok(indices)
    }

    async fn get_cls_flash(&self) -> Result<Vec<ClsFlashItem>, DataError> {
        let query = "今日财经快讯";
        let json = self.nd_query(query).await?;

        let mut items = Vec::new();
        if let Some(recalls) = json["data"]["docData"]["docRecall"].as_array() {
            for group in recalls {
                if let Some(docs) = group["docList"].as_array() {
                    for doc in docs {
                        let title = doc["title"].as_str().unwrap_or("");
                        let content = doc["content"].as_str().unwrap_or("");
                        let source = doc["source"].as_str().unwrap_or("NeoData");
                        let ts = doc["publishTime"].as_i64().unwrap_or(0);
                        let time = chrono::DateTime::from_timestamp(ts, 0)
                            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                            .unwrap_or_default();

                        items.push(ClsFlashItem {
                            title: title.to_string(),
                            content: content.chars().take(300).collect(),
                            publish_time: time,
                            source: Some(source.to_string()),
                        });
                    }
                }
            }
        }

        if items.is_empty() {
            return Err(DataError::VendorError {
                vendor: "neodata".into(),
                message: "NeoData 快讯数据为空".into(),
            });
        }
        Ok(items)
    }

    // ── 通过自然语言查询获取行业/同行数据（为 baseline 提供兜底）──

    async fn get_sector_info(&self, stock_code: &str) -> Result<Option<SectorInfo>, DataError> {
        // 先尝试：查股票的行业归属（短查询，容易命中）
        let query = format!("{}的行业分类", stock_code);
        let json = self.nd_query(&query).await?;
        // 尝试从 apiData 提取
        if let Some(content) = extract_api_content(&json, "basic_info") {
            let pairs = parse_kv_text(&content);
            let sector = kv_str(&pairs, "行业")
                .or_else(|| kv_str(&pairs, "所属行业"))
                .or_else(|| kv_str(&pairs, "板块"));
            if let Some(s) = sector {
                let s = s.to_string();
                if !s.is_empty() && s != "无" && s.find("暂无").is_none() {
                    return Ok(Some(SectorInfo {
                        stock_code: stock_code.to_string(),
                        sector_name: s.clone(),
                        sub_sector: String::new(),
                        concept_tags: vec![],
                        avg_pe: None,
                        avg_pb: None,
                    }));
                }
            }
        }
        // 兜底：用 docData 中的文章关键词推断行业
        if let Some(recalls) = json["data"]["docData"]["docRecall"].as_array() {
            for group in recalls {
                if let Some(docs) = group["docList"].as_array() {
                    for doc in docs {
                        if let Some(title) = doc["title"].as_str() {
                            for kw in &[
                                "半导体",
                                "新能源",
                                "医药",
                                "军工",
                                "汽车",
                                "化工",
                                "消费电子",
                                "AI",
                                "光伏",
                                "风电",
                                "机器人",
                            ] {
                                if title.contains(kw) {
                                    return Ok(Some(SectorInfo {
                                        stock_code: stock_code.to_string(),
                                        sector_name: kw.to_string(),
                                        sub_sector: String::new(),
                                        concept_tags: vec![],
                                        avg_pe: None,
                                        avg_pb: None,
                                    }));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(None) // 无法确定行业
    }

    // ── 必须实现的 trait 方法（返回空，由上游 vendor 兜底） ──

    async fn get_klines(
        &self,
        _stock_code: &str,
        _period: &str,
        _limit: u32,
        _adj: Option<AdjType>,
    ) -> Result<Vec<KLine>, DataError> {
        Err(DataError::VendorError {
            vendor: "neodata".into(),
            message: "NeoData 不提供 K 线数据（由专业 vendor 覆盖）".into(),
        })
    }

    async fn get_money_flow(&self, _stock_code: &str) -> Result<Option<MoneyFlow>, DataError> {
        Err(DataError::VendorError {
            vendor: "neodata".into(),
            message: "NeoData 不提供资金流向数据".into(),
        })
    }

    async fn get_dragon_tiger(
        &self,
        _stock_code: &str,
    ) -> Result<Vec<DragonTigerEntry>, DataError> {
        Err(DataError::VendorError {
            vendor: "neodata".into(),
            message: "NeoData 不提供龙虎榜数据".into(),
        })
    }

    async fn get_lockup_schedule(
        &self,
        _stock_code: &str,
    ) -> Result<Vec<LockupSchedule>, DataError> {
        Err(DataError::VendorError {
            vendor: "neodata".into(),
            message: "NeoData 不提供限售解禁数据".into(),
        })
    }
}

/// 创建 NeoDataVendor 实例
pub fn create_vendor(token: Arc<RwLock<String>>) -> Box<dyn StockVendor> {
    Box::new(NeoDataVendor { token })
}

/// 将 token 保存到 Python 脚本缓存（供 Tauri 命令调用）
///
/// 当用户通过 WorkBuddy 的 connect_cloud_service 获得新 token 后，
/// 调用此函数将 token 写入脚本缓存文件，使其对后续查询自动生效。
/// 等价于在命令行执行 `python3 query.py --save-token "<token>"`。
pub async fn save_token_to_cache(token: &str) -> Result<(), DataError> {
    let script_path = find_script();
    let output = tokio::process::Command::new(python_cmd())
        .arg(&script_path)
        .arg("--save-token")
        .arg(token)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| DataError::VendorError {
            vendor: "neodata".into(),
            message: format!("Python 执行失败: {e}"),
        })?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(DataError::VendorError {
            vendor: "neodata".into(),
            message: format!("保存 token 失败: {stderr}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_kv_text() {
        let text =
            "【行情数据】\n股票名称:贵州茅台\n股票代码:600519\n最新价:1800.00\n涨跌幅:+0.56%";
        let pairs = parse_kv_text(text);
        assert_eq!(kv_str(&pairs, "股票名称"), Some("贵州茅台"));
        assert_eq!(kv_str(&pairs, "股票代码"), Some("600519"));
        assert!((kv_f64(&pairs, "最新价").unwrap() - 1800.0).abs() < 0.01);
        assert!((kv_f64(&pairs, "涨跌幅").unwrap() - 0.56).abs() < 0.01);
    }

    #[test]
    fn test_extract_code() {
        assert_eq!(extract_code("股票代码:600519").as_deref(), Some("600519"));
        assert_eq!(extract_code("股票代码：00700.HK").as_deref(), Some("00700.HK"));
        assert_eq!(extract_code("代码:600519").as_deref(), Some("600519"));
    }

    #[test]
    fn test_extract_name() {
        assert_eq!(extract_name("股票名称:贵州茅台").as_deref(), Some("贵州茅台"));
    }
}
