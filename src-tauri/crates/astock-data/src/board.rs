use crate::error::DataError;
use crate::types::{BoardMember, ConceptBoard};

pub(crate) async fn search_concept_boards(
    http: &reqwest::Client,
    keyword: &str,
) -> Result<Vec<ConceptBoard>, DataError> {
    let url = format!(
        "https://searchapi.eastmoney.com/api/suggest/get?input={}&type=14&token=D43BF722C8E33BDC906FB84D85E326E86&count=20",
        urlencoding::encode(keyword)
    );

    let resp = http
        .get(&url)
        .header("Referer", "https://so.eastmoney.com/")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
        )
        .send()
        .await;

    match resp {
        Ok(r) => {
            let json: serde_json::Value = r.json().await.unwrap_or_default();
            let data = json["QuotationCodeTable"]["Data"]
                .as_array()
                .or_else(|| json["Data"].as_array())
                .cloned()
                .unwrap_or_default();

            if data.is_empty() {
                return Ok(vec![]);
            }

            Ok(data
                .iter()
                .filter_map(|item| {
                    let board_code = item
                        .get("Code")
                        .or_else(|| item.get("code"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let board_name = item
                        .get("Name")
                        .or_else(|| item.get("name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let stock_count = item
                        .get("Count")
                        .or_else(|| item.get("count"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32;

                    if board_code.is_empty() || board_name.is_empty() {
                        return None;
                    }

                    Some(ConceptBoard { board_code, board_name, stock_count })
                })
                .collect())
        },
        Err(e) => {
            tracing::warn!("[search_concept_boards] 请求失败: {e}");
            Ok(vec![])
        },
    }
}

/// 获取概念板块成分股（东财 push2 clist，按板块代码过滤，翻页拉全）
///
/// 接口：`push2.eastmoney.com/api/qt/clist/get?fs=b:{board_code}`，
/// 返回板块内全部成分股（f12 代码 / f14 名称 / f3 涨跌幅，fltt=2 时 f3 为百分比数值）。
/// 与 `dataapi/bkzj`（板块资金流，仅资金流覆盖的个股）不同，此处为完整成分股列表。
///
/// 注：本方法带 board_code 参数、按板块粒度缓存语义无法用 method 级日快照
/// （SNAPSHOT_METHODS 为全市场方法），故不登记 daily_snapshot。
pub(crate) async fn get_concept_board_members(
    http: &reqwest::Client,
    board_code: &str,
) -> Result<Vec<BoardMember>, DataError> {
    let mut all: Vec<BoardMember> = Vec::new();
    let page_size: u32 = 100;
    let mut page: u32 = 1;

    loop {
        let url = format!(
            "https://push2.eastmoney.com/api/qt/clist/get?pn={page}&pz={page_size}&po=1&np=1&fltt=2&invt=2&fid=f3&fs=b:{board_code}&fields=f12,f14,f3"
        );

        let resp = http
            .get(&url)
            .header("Referer", "https://quote.eastmoney.com/")
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
            )
            .send()
            .await;

        match resp {
            Ok(r) => {
                let json: serde_json::Value = r.json().await.unwrap_or_default();
                let total = json["data"]["total"].as_u64().unwrap_or(0) as u32;
                let diff = json["data"]["diff"].as_array().cloned().unwrap_or_default();

                if diff.is_empty() {
                    break;
                }

                for item in &diff {
                    let stock_code = item["f12"].as_str().unwrap_or("").to_string();
                    let stock_name = item["f14"].as_str().unwrap_or("").to_string();
                    // fltt=2：f3 已是百分比数值（如 1.5 表示 1.5%），勿再除 100
                    let change_pct = item["f3"].as_f64();
                    if !stock_code.is_empty() && !stock_name.is_empty() {
                        all.push(BoardMember { stock_code, stock_name, change_pct });
                    }
                }

                // 翻页终止：已取完或单页不足（末页）
                if (page * page_size) >= total || diff.len() < page_size as usize {
                    break;
                }
            },
            Err(e) => {
                tracing::warn!("[get_concept_board_members] 请求失败: {e}");
                break;
            },
        }

        // 防御性上限，避免异常 total 导致死循环
        if page >= 50 {
            tracing::warn!("[get_concept_board_members] 超过 50 页上限，截断 (board={board_code})");
            break;
        }
        page += 1;
    }

    Ok(all)
}
