use crate::as_of_capability::AsOfCapability;
use crate::error::DataError;
use crate::types::*;
use crate::vendors::StockVendor;
use async_trait::async_trait;

pub struct TencentVendor {
    pub http: reqwest::Client,
}

impl TencentVendor {
    /// 带 429 检测的 GET 请求
    async fn tencent_get(&self, url: &str) -> Result<reqwest::Response, DataError> {
        let resp = self.http.get(url).send().await?;
        crate::check_response_429(&resp, "tencent")?;
        Ok(resp)
    }
}

/// 将 AxInvest 股票代码转为腾讯财经格式
/// 600519 → sh600519, 000001 → sz000001, 300750 → sz300750
fn to_tencent_code(stock_code: &str) -> String {
    let prefix = match stock_code.chars().next() {
        Some('6') => "sh",
        Some('0') | Some('3') | Some('2') => "sz",
        Some('8') | Some('4') => "bj",
        _ => "sz",
    };
    format!("{prefix}{stock_code}")
}

/// 解析腾讯财经实时行情响应
/// API 格式: v_sh600519="1~贵州茅台~600519~1272.86~1268.00~1278.00~..."
/// 字段结构 (以 ~ 分隔):
///   fields[0]  标志位(1=上海,51=深圳)
///   fields[1]  股票名称
///   fields[2]  股票代码
///   fields[3]  当前价
///   fields[4]  昨收
///   fields[5]  今开
///   fields[6]  成交量(手) — 与 fields[ts+6] 重复
///   fields[7]  外盘
///   fields[8]  内盘
///   fields[9..29] 买卖五档 (价格/数量交替)
///   fields[30] 时间戳 (YYYYMMDDHHMMSS)
///   fields[31] 涨跌额
///   fields[32] 涨跌幅%
///   fields[33] 最高价
///   fields[34] 最低价
///   fields[35] 现价/成交量/成交额 复合字段
///   fields[36] 成交量(手)
///   fields[37] 成交额(万)
///   fields[38] 换手率%
///   fields[39] 市盈率
///   fields[43] 振幅%
///   fields[44] 总市值(亿)
///   fields[45] 流通市值(亿)
///   fields[46] 市净率
///   fields[47] 涨停价
///   fields[48] 跌停价
fn parse_quote(raw: &str) -> Result<StockQuote, DataError> {
    let start = raw.find('"').ok_or_else(|| DataError::ParseError("no opening quote".into()))?;
    let end = raw[start + 1..]
        .find('"')
        .ok_or_else(|| DataError::ParseError("no closing quote".into()))?;
    let data = &raw[start + 1..start + 1 + end];
    let fields: Vec<&str> = data.split('~').collect();

    // 腾讯对不存在的股票/停牌股可能返回简版错误（如 "CODE~FAIL~" 或空引号，仅 1-4 字段）
    if fields.len() < 50 {
        if fields.len() < 5 {
            let code = fields.get(2).or_else(|| fields.first()).copied().unwrap_or("unknown");
            return Err(DataError::NotFound(code.to_string()));
        }
        return Err(DataError::ParseError(format!("expected >=50 fields, got {}", fields.len())));
    }

    let parse = |s: &str| -> f64 { s.parse().unwrap_or(0.0) };
    let parse_opt = |s: &str| -> Option<f64> {
        let v: f64 = s.parse().ok()?;
        if v == 0.0 {
            None
        } else {
            Some(v)
        }
    };

    // 从股票名称检测 ST 状态
    let name = fields[1].to_string();
    let is_st = name.contains("ST") || name.contains("*ST");

    // 通过时间戳定位后续字段（14位数字），兼容字段数可能变化的情况
    let ts_idx = fields
        .iter()
        .position(|f| f.len() == 14 && f.chars().all(|c| c.is_ascii_digit()))
        .unwrap_or_else(|| {
            // 修复 M-RES-4: fallback 30 时无日志，调试黑洞。
            // 当上游格式变更导致时间戳字段无法定位时，记录 warn 便于发现。
            tracing::warn!(
                "[tencent] 时间戳字段定位失败，回退到默认索引 30 (fields_len={})",
                fields.len()
            );
            30
        });

    if fields.len() < ts_idx + 19 {
        return Err(DataError::ParseError(format!(
            "expected >= {} fields after timestamp, got {}",
            ts_idx + 19,
            fields.len()
        )));
    }

    Ok(StockQuote {
        code: fields[2].to_string(),
        name,
        price: parse(fields[3]),
        pre_close: parse(fields[4]),
        open: parse(fields[5]),
        high: parse(fields[ts_idx + 3]),
        low: parse(fields[ts_idx + 4]),
        volume: parse(fields[ts_idx + 6]) * 100.0, // 手 → 股
        amount: parse(fields[ts_idx + 7]) * 10000.0, // 万 → 元
        change_pct: parse(fields[ts_idx + 2]),
        turnover_rate: parse(fields[ts_idx + 8]),
        pe: parse_opt(fields[ts_idx + 9]),
        pb: parse_opt(fields[ts_idx + 16]),
        total_mv: parse_opt(fields[ts_idx + 14]).map(|v| v * 1e8),
        circulating_mv: parse_opt(fields[ts_idx + 15]).map(|v| v * 1e8),
        limit_up: {
            let v = parse(fields[ts_idx + 17]);
            if v > 0.0 {
                Some(v)
            } else {
                None
            }
        },
        limit_down: {
            let v = parse(fields[ts_idx + 18]);
            if v > 0.0 {
                Some(v)
            } else {
                None
            }
        },
        is_st,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// 解析腾讯财经 K 线 JSON 响应
/// API (2026年端点):
///   不复权日/周/月: https://web.ifzq.gtimg.cn/appstock/app/kline/kline?param=sz000001,day,,,120
///   前/后复权:      https://web.ifzq.gtimg.cn/appstock/app/fqkline/get?param=sz000001,day,,,120,qfq
///   分钟K:          https://web.ifzq.gtimg.cn/appstock/app/kline/mkline?param=sz000001,m5,,320
/// 注意：日/周/月K线返回6字段[date,open,close,high,low,volume(手)]，无amount字段
///       复权K线返回key格式为 {prefix}{period}，如 qfqday/hfqweek
fn parse_klines(raw: &str, period_key: &str, fq_prefix: &str) -> Result<Vec<KLine>, DataError> {
    let json: serde_json::Value = serde_json::from_str(raw)
        .map_err(|e| DataError::ParseError(format!("K线JSON解析失败: {e}")))?;
    // 检查返回码
    let code = json["code"].as_i64().unwrap_or(-1);
    if code != 0 {
        let msg = json["msg"].as_str().unwrap_or("unknown error");
        return Err(DataError::ParseError(format!("腾讯K线API返回错误: code={code}, msg={msg}")));
    }
    let data = &json["data"];
    if data.is_string() || data.is_null() {
        return Err(DataError::ParseError("腾讯K线返回data为空".into()));
    }
    let code_key = data
        .as_object()
        .and_then(|obj| {
            obj.keys().find(|k| k.starts_with("sz") || k.starts_with("sh") || k.starts_with("bj"))
        })
        .cloned()
        .unwrap_or_default();
    if code_key.is_empty() {
        return Err(DataError::ParseError("K线数据中未找到股票代码键".into()));
    }
    let stock_data = &data[&code_key];

    // 构造可能的键名列表（优先复权键，回退非复权键）
    // 日/周/月: day/week/month, 复权键为 qfqday/hfqday/qfqweek 等
    // 分钟: m5/m15/m30/m60, 不复权时key为 m5/m15 等（注意分钟线mkline端点不支持复权）
    let fq_key = format!("{fq_prefix}{period_key}");
    let kline_list = if !fq_prefix.is_empty() {
        stock_data[&fq_key].as_array().or_else(|| stock_data[period_key].as_array())
    } else {
        stock_data[period_key].as_array()
    }
    .ok_or_else(|| DataError::ParseError(format!("未找到K线数组 (key={fq_key}/{period_key})")))?;

    let adj_marker = if !fq_prefix.is_empty() {
        Some(1.0)
    } else {
        None
    };

    let mut result = Vec::new();
    for item in kline_list {
        let arr = item.as_array().ok_or_else(|| DataError::ParseError("K线项不是数组".into()))?;
        if arr.len() < 6 {
            continue;
        }
        let parse = |i: usize| -> f64 {
            arr.get(i).and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0.0)
        };
        // 字段顺序: [date, open, close, high, low, volume(手), amount(万元)?]
        // 注意：2026年API日/周/月K线只有6个字段，无amount；分钟线可能有第7字段
        let amount = if arr.len() > 6 {
            parse(6) * 10000.0
        } else {
            0.0
        };
        result.push(KLine {
            date: arr[0].as_str().unwrap_or("").to_string(),
            open: parse(1),
            close: parse(2),
            high: parse(3),
            low: parse(4),
            volume: parse(5) * 100.0, // 腾讯 K线 volume 单位为"手"，×100 转为"股"
            amount,                   // 若有amount字段则×10000(万元→元)，否则为0
            turnover_rate: None,
            adj_factor: adj_marker,
        });
    }
    Ok(result)
}

#[async_trait]
impl StockVendor for TencentVendor {
    async fn get_quote(&self, stock_code: &str) -> Result<StockQuote, DataError> {
        let tc_code = to_tencent_code(stock_code);
        let url = format!("https://qt.gtimg.cn/q={tc_code}");
        let resp = self.tencent_get(&url).await?;
        let bytes = resp.bytes().await?;
        // 腾讯财经 API 使用 GBK 编码，需手动转 UTF-8
        let text = encoding_rs::GBK.decode(&bytes).0;

        // 上证指数代码（000xxx）可能被误映射为 sz 前缀
        // 检测到无匹配时用 sh 前缀重试
        if text.contains("pv_none_match") && stock_code.starts_with("00") {
            let sh_code = format!("sh{stock_code}");
            let sh_url = format!("https://qt.gtimg.cn/q={sh_code}");
            let sh_resp = self.tencent_get(&sh_url).await?;
            let sh_bytes = sh_resp.bytes().await?;
            let sh_text = encoding_rs::GBK.decode(&sh_bytes).0;
            if !sh_text.contains("pv_none_match") {
                return parse_quote(&sh_text);
            }
        }

        parse_quote(&text)
    }

    // 以下方法由腾讯财经 API 提供
    async fn get_klines(
        &self,
        stock_code: &str,
        period: &str,
        limit: u32,
        adj: Option<AdjType>,
    ) -> Result<Vec<KLine>, DataError> {
        let tc_code = to_tencent_code(stock_code);
        // 周期映射：支持分钟线和日/周/月线
        let (period_key, is_minute) = match period {
            "5" | "Min5" => ("m5", true),
            "15" | "Min15" => ("m15", true),
            "30" | "Min30" => ("m30", true),
            "60" | "Min60" => ("m60", true),
            "weekly" | "102" | "Weekly" => ("week", false),
            "monthly" | "103" | "Monthly" => ("month", false),
            _ => ("day", false),
        };
        // 复权参数：分钟线不支持复权（mkline端点无复权参数），日/周/月线根据adj选择
        // API端点 (2026年):
        //   分钟线:       kline/mkline (param格式: code,m5,,limit)
        //   日/周/月不复权: kline/kline  (param格式: code,day,,,limit)
        //   日/周/月复权:   fqkline/get  (param格式: code,day,,,limit,fq)
        let (fq_prefix, api_endpoint, param_suffix) = if is_minute {
            ("", "kline/mkline", format!(",{limit}"))
        } else {
            match adj {
                Some(AdjType::Forward) => ("qfq", "fqkline/get", format!(",,,{limit},qfq")),
                Some(AdjType::Backward) => ("hfq", "fqkline/get", format!(",,,{limit},hfq")),
                _ => ("", "kline/kline", format!(",,,{limit}")),
            }
        };
        let url = format!(
            "https://web.ifzq.gtimg.cn/appstock/app/{api_endpoint}?param={tc_code},{period_key}{param_suffix}"
        );

        let mut klines = match self.http.get(&url).send().await {
            Ok(r) => {
                crate::check_response_429(&r, "tencent")?;
                let body = r.text().await?;
                parse_klines(&body, period_key, fq_prefix)
            },
            Err(e) => Err(DataError::from(e)),
        };

        // 上证指数代码（000xxx）可能被误映射为 sz 前缀，解析结果为空时用 sh 前缀重试
        if klines.as_ref().is_ok_and(|v| v.is_empty()) && stock_code.starts_with("00") {
            let sh_code = format!("sh{stock_code}");
            let sh_url = format!(
                "https://web.ifzq.gtimg.cn/appstock/app/{api_endpoint}?param={sh_code},{period_key}{param_suffix}"
            );
            if let Ok(r) = self.http.get(&sh_url).send().await {
                if let Ok(body) = r.text().await {
                    if let Ok(sh_klines) = parse_klines(&body, period_key, fq_prefix) {
                        if !sh_klines.is_empty() {
                            klines = Ok(sh_klines);
                        }
                    }
                }
            }
        }

        klines
    }

    async fn get_financials(&self, _: &str) -> Result<Vec<FinancialReport>, DataError> {
        // 腾讯无直接财务 API，由其他 vendor 承担
        Ok(vec![])
    }

    async fn get_news(&self, _: &str, _: u32) -> Result<Vec<NewsItem>, DataError> {
        // 腾讯无直接新闻 API，由其他 vendor 承担
        Ok(vec![])
    }

    async fn get_money_flow(&self, stock_code: &str) -> Result<Option<MoneyFlow>, DataError> {
        let symbol = to_tencent_code(stock_code);
        let url = format!("https://qt.gtimg.cn/q=ff_{symbol}");
        let resp = self.tencent_get(&url).await?;
        let bytes = resp.bytes().await?;
        let text = encoding_rs::GBK.decode(&bytes).0;
        // 格式: v_ff_sz000858="code~main_in~main_out~main_net~main_ratio~retail_in~retail_out~retail_net~retail_ratio~total~?~?~name~date";
        if let Some(line) = text.lines().next() {
            let raw = line.trim().trim_start_matches(|c: char| c != '"').trim_matches('"');
            let parts: Vec<&str> = raw.split('~').collect();
            if parts.len() >= 14 && !parts[3].is_empty() {
                let parse =
                    |i: usize| parts.get(i).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                return Ok(Some(MoneyFlow {
                    date: parts[13].to_string(),
                    main_net_inflow: parse(3),
                    super_large_net: 0.0,
                    large_net: 0.0,
                    medium_net: 0.0,
                    small_net: parse(7), // 散户净流入
                    history: Vec::new(),
                }));
            }
        }
        Ok(None)
    }

    async fn get_dragon_tiger(&self, _: &str) -> Result<Vec<DragonTigerEntry>, DataError> {
        Ok(vec![])
    }

    async fn get_lockup_schedule(&self, _: &str) -> Result<Vec<LockupSchedule>, DataError> {
        Ok(vec![])
    }

    async fn get_index_quotes(&self) -> Result<Vec<IndexQuote>, DataError> {
        let indices: Vec<(&str, &str)> =
            vec![("sh000001", "上证指数"), ("sz399001", "深证成指"), ("sz399006", "创业板指")];
        let codes = indices.iter().map(|(c, _)| *c).collect::<Vec<_>>().join(",");
        let url = format!("https://qt.gtimg.cn/q={}", codes);
        let resp = self.tencent_get(&url).await?;
        let bytes = resp.bytes().await?;
        // 腾讯财经 API 使用 GBK 编码
        let text = encoding_rs::GBK.decode(&bytes).0;

        // 用 HashMap 按股票代码匹配，避免依赖返回行顺序
        use std::collections::HashMap;
        let name_map: HashMap<&str, &str> = indices.iter().copied().collect();

        let mut results = Vec::new();
        for line in text.lines() {
            if line.is_empty() || !line.contains('~') {
                continue;
            }
            let fields: Vec<&str> = line.split('~').collect();
            if fields.len() < 50 {
                continue;
            }

            let code_in = fields[2]; // e.g. "sh000001"
            let name = match name_map.get(code_in) {
                Some(n) => n.to_string(),
                None => continue, // 不是我们请求的指数
            };

            let parse = |s: &str| -> f64 { s.parse().unwrap_or(0.0) };

            let ts_idx = fields
                .iter()
                .position(|f| f.len() == 14 && f.chars().all(|c| c.is_ascii_digit()))
                .unwrap_or(30);

            if fields.len() < ts_idx + 8 {
                continue;
            }

            results.push(IndexQuote {
                code: code_in.to_string(),
                name,
                price: parse(fields[3]),
                pre_close: parse(fields[4]),
                change_pct: parse(fields[ts_idx + 2]),
                volume: parse(fields[ts_idx + 6]) * 100.0,
                amount: parse(fields[ts_idx + 7]) * 10000.0,
            });
        }

        // 按请求顺序排序，保证面板显示顺序一致
        let order: HashMap<&str, usize> =
            indices.iter().enumerate().map(|(i, (c, _))| (*c, i)).collect();
        results.sort_by_key(|q| order.get(q.code.as_str()).copied().unwrap_or(99));

        Ok(results)
    }

    async fn search_stock(&self, _: &str) -> Result<Vec<StockSearchResult>, DataError> {
        Ok(vec![])
    }

    // ── P3:tencent 能力申报 ──
    // get_quote/get_index_quotes:实时快照 → SynthesizeFromKline
    // get_klines:原生支持日期范围 → NativeDateParam
    // 其他 stub:Fallthrough
    fn asof_capability(&self, method: &str) -> AsOfCapability {
        match method {
            "get_quote" | "get_index_quotes" => AsOfCapability::SynthesizeFromKline,
            "get_klines" => AsOfCapability::NativeDateParam,
            _ => AsOfCapability::Fallthrough,
        }
    }
}

#[cfg(test)]
mod capability_tests {
    use super::*;

    fn make_vendor() -> TencentVendor {
        TencentVendor { http: reqwest::Client::new() }
    }

    #[test]
    fn tencent_quote_and_index_are_synthesize() {
        let v = make_vendor();
        assert_eq!(v.asof_capability("get_quote"), AsOfCapability::SynthesizeFromKline);
        assert_eq!(v.asof_capability("get_index_quotes"), AsOfCapability::SynthesizeFromKline);
    }

    #[test]
    fn tencent_klines_is_native() {
        let v = make_vendor();
        assert_eq!(v.asof_capability("get_klines"), AsOfCapability::NativeDateParam);
    }

    #[test]
    fn tencent_others_are_fallthrough() {
        let v = make_vendor();
        for m in &[
            "get_financials",
            "get_news",
            "get_money_flow",
            "get_dragon_tiger",
            "get_lockup_schedule",
            "search_stock",
        ] {
            assert_eq!(v.asof_capability(m), AsOfCapability::Fallthrough);
        }
    }
}
