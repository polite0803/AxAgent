use crate::as_of_capability::AsOfCapability;
use crate::error::DataError;
use crate::types::*;
use crate::vendors::StockVendor;
use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const TDX_SERVERS: &[(&str, u16)] = &[
    ("119.147.212.81", 7709),
    ("112.74.214.43", 7709),
    ("221.231.141.60", 7709),
    ("101.227.73.20", 7709),
    ("101.227.77.254", 7709),
    ("14.215.128.18", 7709),
    ("59.173.18.140", 7709),
];

const RSP_HEADER_LEN: usize = 0x10;

pub struct MootdxVendor {
    pub host: String,
    pub port: u16,
}

impl MootdxVendor {
    pub fn new() -> Self {
        let (host, port) = TDX_SERVERS[0];
        Self {
            host: host.to_string(),
            port,
        }
    }

    #[allow(dead_code)]
    pub fn with_server(host: &str, port: u16) -> Self {
        Self {
            host: host.to_string(),
            port,
        }
    }

    async fn connect(&self) -> Result<TdxConnection, DataError> {
        let stream = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            TcpStream::connect((&*self.host, self.port)),
        )
        .await
        .map_err(|_| {
            DataError::IoError(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("TDX connect timeout: {}:{}", self.host, self.port),
            ))
        })?
        .map_err(DataError::IoError)?;

        let mut conn = TdxConnection { stream };

        conn.setup().await?;

        Ok(conn)
    }

    fn market_code(stock_code: &str) -> u8 {
        if stock_code.starts_with('6') || stock_code.starts_with('9') {
            1
        } else if stock_code.starts_with('8') || stock_code.starts_with('4') {
            2
        } else {
            0
        }
    }

    fn kline_category(period: &str) -> u16 {
        match period {
            "5" | "Min5" => 4,
            "15" | "Min15" => 5,
            "30" | "Min30" => 6,
            "60" | "Min60" => 7,
            "daily" | "101" | "Daily" | "8" => 8,
            "weekly" | "102" | "Weekly" | "9" => 9,
            "monthly" | "103" | "Monthly" | "10" => 10,
            _ => 8,
        }
    }
}

impl Default for MootdxVendor {
    fn default() -> Self {
        Self::new()
    }
}

struct TdxConnection {
    stream: TcpStream,
}

impl TdxConnection {
    async fn setup(&mut self) -> Result<(), DataError> {
        let setup1: Vec<u8> = vec![
            0x0c, 0x02, 0x18, 0x93, 0x00, 0x01, 0x03, 0x00, 0x03, 0x00, 0x0d, 0x00, 0x01,
        ];
        let setup2: Vec<u8> = vec![
            0x0c, 0x02, 0x18, 0x94, 0x00, 0x01, 0x03, 0x00, 0x03, 0x00, 0x0d, 0x00, 0x02,
        ];
        let setup3: Vec<u8> = vec![
            0x0c, 0x03, 0x18, 0x99, 0x00, 0x01, 0x20, 0x00, 0x20, 0x00, 0xdb, 0x0f, 0xd5, 0xd0,
            0xc9, 0xcc, 0xd6, 0xa4, 0xa8, 0xaf, 0x00, 0x00, 0x00, 0x8f, 0xc2, 0x25, 0x40, 0x13,
            0x00, 0x00, 0xd5, 0x00, 0xc9, 0xcc, 0xbd, 0xf0, 0xd7, 0xea, 0x00, 0x00, 0x00, 0x02,
        ];

        self.stream.write_all(&setup1).await?;
        self.read_response_body().await?;

        self.stream.write_all(&setup2).await?;
        self.read_response_body().await?;

        self.stream.write_all(&setup3).await?;
        self.read_response_body().await?;

        Ok(())
    }

    async fn send_and_recv(&mut self, pkg: &[u8]) -> Result<Vec<u8>, DataError> {
        self.stream.write_all(pkg).await?;
        self.read_response_body().await
    }

    async fn read_response_body(&mut self) -> Result<Vec<u8>, DataError> {
        let mut header = [0u8; RSP_HEADER_LEN];
        self.stream.read_exact(&mut header).await?;

        let zip_size = u16::from_le_bytes([header[12], header[13]]) as usize;
        let unzip_size = u16::from_le_bytes([header[14], header[15]]) as usize;

        let mut body = vec![0u8; zip_size];
        self.stream.read_exact(&mut body).await?;

        if zip_size != unzip_size && zip_size > 0 {
            let decompressed = decompress_zlib(&body, unzip_size)?;
            Ok(decompressed)
        } else {
            Ok(body)
        }
    }

    async fn get_security_quotes(
        &mut self,
        stocks: &[(u8, &str)],
    ) -> Result<Vec<QuoteResult>, DataError> {
        let stock_len = stocks.len() as u16;
        let pkgdatalen = stock_len * 7 + 12;

        let mut pkg = Vec::with_capacity(22 + stocks.len() * 7);
        pkg.extend_from_slice(&(0x10cu16).to_le_bytes());
        pkg.extend_from_slice(&0x02006320u32.to_le_bytes());
        pkg.extend_from_slice(&pkgdatalen.to_le_bytes());
        pkg.extend_from_slice(&pkgdatalen.to_le_bytes());
        pkg.extend_from_slice(&0x5053eu32.to_le_bytes());
        pkg.extend_from_slice(&0u32.to_le_bytes());
        pkg.extend_from_slice(&0u16.to_le_bytes());
        pkg.extend_from_slice(&stock_len.to_le_bytes());

        for (market, code) in stocks {
            pkg.push(*market);
            let code_bytes = code.as_bytes();
            for i in 0..6 {
                pkg.push(if i < code_bytes.len() {
                    code_bytes[i]
                } else {
                    0
                });
            }
        }

        let body = self.send_and_recv(&pkg).await?;
        self.parse_quotes(&body)
    }

    fn parse_quotes(&self, body: &[u8]) -> Result<Vec<QuoteResult>, DataError> {
        if body.len() < 4 {
            return Ok(vec![]);
        }

        let mut pos = 2;
        let num_stock = u16::from_le_bytes([body[pos], body[pos + 1]]) as usize;
        pos += 2;

        let mut results = Vec::with_capacity(num_stock);

        for _ in 0..num_stock {
            if pos + 9 > body.len() {
                break;
            }

            let market = body[pos];
            pos += 1;

            let code_end = (pos + 6).min(body.len());
            let code = String::from_utf8_lossy(&body[pos..code_end])
                .trim_end_matches('\0')
                .to_string();
            pos += 6;

            let _active1 = u16::from_le_bytes([body[pos], body[pos + 1]]);
            pos += 2;

            let (price, new_pos) = get_price(body, pos)?;
            pos = new_pos;

            let (last_close_diff, new_pos) = get_price(body, pos)?;
            pos = new_pos;

            let (open_diff, new_pos) = get_price(body, pos)?;
            pos = new_pos;

            let (high_diff, new_pos) = get_price(body, pos)?;
            pos = new_pos;

            let (low_diff, new_pos) = get_price(body, pos)?;
            pos = new_pos;

            let (_reversed0, new_pos) = get_price(body, pos)?;
            pos = new_pos;

            let (_reversed1, new_pos) = get_price(body, pos)?;
            pos = new_pos;

            let (vol, new_pos) = get_price(body, pos)?;
            pos = new_pos;

            let (_cur_vol, new_pos) = get_price(body, pos)?;
            pos = new_pos;

            if pos + 4 > body.len() {
                break;
            }
            let amount_raw =
                u32::from_le_bytes([body[pos], body[pos + 1], body[pos + 2], body[pos + 3]]);
            let amount = get_volume(amount_raw);
            pos += 4;

            let (_s_vol, new_pos) = get_price(body, pos)?;
            pos = new_pos;

            let (_b_vol, new_pos) = get_price(body, pos)?;
            pos = new_pos;

            let (_rev2, new_pos) = get_price(body, pos)?;
            pos = new_pos;

            let (_rev3, new_pos) = get_price(body, pos)?;
            pos = new_pos;

            for _ in 0..5 {
                let (_, new_pos) = get_price(body, pos)?;
                pos = new_pos;
                let (_, new_pos) = get_price(body, pos)?;
                pos = new_pos;
                let (_, new_pos) = get_price(body, pos)?;
                pos = new_pos;
                let (_, new_pos) = get_price(body, pos)?;
                pos = new_pos;
            }

            if pos + 2 > body.len() {
                break;
            }
            pos += 2;

            for _ in 0..4 {
                let (_, new_pos) = get_price(body, pos)?;
                pos = new_pos;
            }

            if pos + 4 > body.len() {
                break;
            }
            pos += 4;

            let last_close = cal_price(price, last_close_diff);
            let open = cal_price(price, open_diff);
            let high = cal_price(price, high_diff);
            let low = cal_price(price, low_diff);

            results.push(QuoteResult {
                market,
                code,
                price: cal_price(price, 0),
                last_close,
                open,
                high,
                low,
                vol: vol as f64,
                amount,
            });
        }

        Ok(results)
    }

    async fn get_security_bars(
        &mut self,
        category: u16,
        market: u16,
        code: &str,
        start: u16,
        count: u16,
    ) -> Result<Vec<KLineResult>, DataError> {
        let mut pkg = Vec::with_capacity(44);
        pkg.extend_from_slice(&(0x10cu16).to_le_bytes());
        pkg.extend_from_slice(&0x01016408u32.to_le_bytes());
        pkg.extend_from_slice(&0x1cu16.to_le_bytes());
        pkg.extend_from_slice(&0x1cu16.to_le_bytes());
        pkg.extend_from_slice(&0x052du16.to_le_bytes());
        pkg.extend_from_slice(&market.to_le_bytes());

        let code_bytes = code.as_bytes();
        for i in 0..6 {
            pkg.push(if i < code_bytes.len() {
                code_bytes[i]
            } else {
                0
            });
        }

        pkg.extend_from_slice(&category.to_le_bytes());
        pkg.extend_from_slice(&1u16.to_le_bytes());
        pkg.extend_from_slice(&start.to_le_bytes());
        pkg.extend_from_slice(&count.to_le_bytes());
        pkg.extend_from_slice(&0u32.to_le_bytes());
        pkg.extend_from_slice(&0u32.to_le_bytes());
        pkg.extend_from_slice(&0u16.to_le_bytes());

        let body = self.send_and_recv(&pkg).await?;
        self.parse_bars(category, &body)
    }

    fn parse_bars(&self, category: u16, body: &[u8]) -> Result<Vec<KLineResult>, DataError> {
        if body.len() < 2 {
            return Ok(vec![]);
        }

        let ret_count = u16::from_le_bytes([body[0], body[1]]) as usize;
        let mut pos = 2;
        let mut klines = Vec::with_capacity(ret_count);
        let mut pre_diff_base: i64 = 0;

        for _ in 0..ret_count {
            if pos + 4 > body.len() {
                break;
            }

            let (year, month, day, hour, minute) = parse_datetime(category, body, pos);
            pos += 4;

            let (open_diff, new_pos) = get_price(body, pos)?;
            pos = new_pos;

            let (close_diff, new_pos) = get_price(body, pos)?;
            pos = new_pos;

            let (high_diff, new_pos) = get_price(body, pos)?;
            pos = new_pos;

            let (low_diff, new_pos) = get_price(body, pos)?;
            pos = new_pos;

            if pos + 8 > body.len() {
                break;
            }

            let vol_raw =
                u32::from_le_bytes([body[pos], body[pos + 1], body[pos + 2], body[pos + 3]]);
            let vol = get_volume(vol_raw);
            pos += 4;

            let amount_raw =
                u32::from_le_bytes([body[pos], body[pos + 1], body[pos + 2], body[pos + 3]]);
            let amount = get_volume(amount_raw);
            pos += 4;

            let open = cal_price1000(open_diff, pre_diff_base);
            let actual_open = open_diff + pre_diff_base;
            let close = cal_price1000(actual_open, close_diff);
            let high = cal_price1000(actual_open, high_diff);
            let low = cal_price1000(actual_open, low_diff);
            pre_diff_base = actual_open + close_diff;

            let date = if category < 4 || category == 7 || category == 8 {
                format!("{year:04}-{month:02}-{day:02}")
            } else {
                format!("{year:04}-{month:02}-{day:02}")
            };

            klines.push(KLineResult {
                date,
                open,
                close,
                high,
                low,
                vol,
                amount,
                _hour: hour,
                _minute: minute,
            });
        }

        Ok(klines)
    }
}

struct QuoteResult {
    #[allow(dead_code)]
    market: u8,
    code: String,
    price: f64,
    last_close: f64,
    open: f64,
    high: f64,
    low: f64,
    vol: f64,
    amount: f64,
}

struct KLineResult {
    date: String,
    open: f64,
    close: f64,
    high: f64,
    low: f64,
    vol: f64,
    amount: f64,
    _hour: u32,
    _minute: u32,
}

fn get_price(data: &[u8], mut pos: usize) -> Result<(i64, usize), DataError> {
    if pos >= data.len() {
        return Err(DataError::ParseError("get_price out of bounds".into()));
    }

    let mut pos_byte: usize = 6;
    let bdata = data[pos];
    let mut intdata = (bdata & 0x3f) as i64;
    let sign = (bdata & 0x40) != 0;

    if (bdata & 0x80) != 0 {
        loop {
            pos += 1;
            if pos >= data.len() {
                return Err(DataError::ParseError("get_price continuation out of bounds".into()));
            }
            let b = data[pos];
            intdata += ((b & 0x7f) as i64) << pos_byte;
            pos_byte += 7;
            if (b & 0x80) == 0 {
                break;
            }
        }
    }

    pos += 1;

    if sign {
        intdata = -intdata;
    }

    Ok((intdata, pos))
}

fn get_volume(ivol: u32) -> f64 {
    let _logpoint = (ivol >> 24) as i32;
    let hheax = (ivol >> 24) as i32;
    let hleax = ((ivol >> 16) & 0xff) as i32;
    let lheax = ((ivol >> 8) & 0xff) as i32;
    let lleax = (ivol & 0xff) as i32;

    let dw_ecx = hheax * 2 - 0x7f;
    let dw_edx = hheax * 2 - 0x86;
    let dw_esi = hheax * 2 - 0x8e;
    let dw_eax = hheax * 2 - 0x96;

    let dbl_xmm6 = if dw_ecx >= 0 {
        2f64.powi(dw_ecx)
    } else {
        1.0 / 2f64.powi(-dw_ecx)
    };

    let dbl_xmm4 = if hleax > 0x80 {
        let tmp1 = 2f64.powi(dw_edx + 1);
        let dbl_xmm0 = 2f64.powi(dw_edx) * 128.0 + (hleax & 0x7f) as f64 * tmp1;
        dbl_xmm0
    } else if dw_edx >= 0 {
        2f64.powi(dw_edx) * hleax as f64
    } else {
        (1.0 / 2f64.powi(-dw_edx)) * hleax as f64
    };

    let mut dbl_xmm3 = 2f64.powi(dw_esi) * lheax as f64;
    let mut dbl_xmm1 = 2f64.powi(dw_eax) * lleax as f64;

    if hleax & 0x80 != 0 {
        dbl_xmm3 *= 2.0;
        dbl_xmm1 *= 2.0;
    }

    dbl_xmm6 + dbl_xmm4 + dbl_xmm3 + dbl_xmm1
}

fn cal_price(base_p: i64, diff: i64) -> f64 {
    (base_p + diff) as f64 / 100.0
}

fn cal_price1000(base_p: i64, diff: i64) -> f64 {
    (base_p + diff) as f64 / 1000.0
}

fn parse_datetime(category: u16, buffer: &[u8], pos: usize) -> (u32, u32, u32, u32, u32) {
    if pos + 4 > buffer.len() {
        return (0, 0, 0, 15, 0);
    }

    if category < 4 || category == 7 || category == 8 {
        let zipday = u16::from_le_bytes([buffer[pos], buffer[pos + 1]]);
        let tminutes = u16::from_le_bytes([buffer[pos + 2], buffer[pos + 3]]);
        let year = ((zipday as u32) >> 11) + 2004;
        let month = ((zipday as u32) % 2048) / 100;
        let day = (zipday as u32) % 2048 % 100;
        let hour = (tminutes as u32) / 60;
        let minute = (tminutes as u32) % 60;
        (year, month, day, hour, minute)
    } else {
        let zipday = u32::from_le_bytes([
            buffer[pos],
            buffer[pos + 1],
            buffer[pos + 2],
            buffer[pos + 3],
        ]);
        let year = zipday / 10000;
        let month = (zipday % 10000) / 100;
        let day = zipday % 100;
        (year, month, day, 15, 0)
    }
}

fn decompress_zlib(data: &[u8], expected_size: usize) -> Result<Vec<u8>, DataError> {
    use std::io::Read;

    let mut decoder = flate2::read::ZlibDecoder::new(data);
    let mut result = Vec::with_capacity(expected_size);
    decoder
        .read_to_end(&mut result)
        .map_err(|e| DataError::ParseError(format!("zlib decompress failed: {e}")))?;
    Ok(result)
}

#[async_trait]
impl StockVendor for MootdxVendor {
    async fn get_quote(&self, stock_code: &str) -> Result<StockQuote, DataError> {
        let market = Self::market_code(stock_code);
        let mut last_err = None;
        for (host, port) in TDX_SERVERS.iter() {
            let vendor = MootdxVendor {
                host: host.to_string(),
                port: *port,
            };
            match vendor.connect().await {
                Ok(mut conn) => {
                    let stocks = vec![(market, stock_code)];
                    match conn.get_security_quotes(&stocks).await {
                        Ok(quotes) if !quotes.is_empty() => {
                            let q = &quotes[0];
                            let change_pct = if q.last_close > 0.0 {
                                (q.price - q.last_close) / q.last_close * 100.0
                            } else {
                                0.0
                            };
                            return Ok(StockQuote {
                                code: q.code.clone(),
                                name: String::new(),
                                price: q.price,
                                pre_close: q.last_close,
                                open: q.open,
                                high: q.high,
                                low: q.low,
                                volume: q.vol,
                                amount: q.amount,
                                change_pct,
                                turnover_rate: 0.0,
                                pe: None,
                                pb: None,
                                total_mv: None,
                                circulating_mv: None,
                                limit_up: None,
                                limit_down: None,
                                is_st: false,
                                timestamp: chrono::Utc::now()
                                    .format("%Y-%m-%d %H:%M:%S")
                                    .to_string(),
                            });
                        },
                        Ok(_) => {
                            last_err = Some(DataError::VendorError {
                                vendor: "mootdx".into(),
                                message: "no quote data from TDX server".into(),
                            });
                        },
                        Err(e) => {
                            last_err = Some(e);
                        },
                    }
                },
                Err(e) => {
                    last_err = Some(e);
                },
            }
        }
        Err(last_err.unwrap_or_else(|| DataError::VendorError {
            vendor: "mootdx".into(),
            message: "all TDX servers failed".into(),
        }))
    }

    async fn get_klines(
        &self,
        stock_code: &str,
        period: &str,
        limit: u32,
        _adj: Option<AdjType>,
    ) -> Result<Vec<KLine>, DataError> {
        let market = Self::market_code(stock_code) as u16;
        let category = Self::kline_category(period);
        let mut last_err = None;
        for (host, port) in TDX_SERVERS.iter() {
            let vendor = MootdxVendor {
                host: host.to_string(),
                port: *port,
            };
            match vendor.connect().await {
                Ok(mut conn) => {
                    match conn
                        .get_security_bars(category, market, stock_code, 0, limit as u16)
                        .await
                    {
                        Ok(bars) if !bars.is_empty() => {
                            return Ok(bars
                                .into_iter()
                                .map(|b| KLine {
                                    date: b.date,
                                    open: b.open,
                                    high: b.high,
                                    low: b.low,
                                    close: b.close,
                                    volume: b.vol,
                                    amount: b.amount,
                                    turnover_rate: None,
                                    // P1-4: vendor 默认不复权
                                    adj_factor: None,
                                })
                                .collect());
                        },
                        Ok(_) => {
                            last_err = Some(DataError::VendorError {
                                vendor: "mootdx".into(),
                                message: "no kline data from TDX server".into(),
                            });
                        },
                        Err(e) => {
                            last_err = Some(e);
                        },
                    }
                },
                Err(e) => {
                    last_err = Some(e);
                },
            }
        }
        Err(last_err.unwrap_or_else(|| DataError::VendorError {
            vendor: "mootdx".into(),
            message: "all TDX servers failed for klines".into(),
        }))
    }

    async fn get_financials(&self, _: &str) -> Result<Vec<FinancialReport>, DataError> {
        Ok(vec![])
    }

    async fn get_news(&self, _: &str, _: u32) -> Result<Vec<NewsItem>, DataError> {
        Ok(vec![])
    }

    async fn get_money_flow(&self, _: &str) -> Result<Option<MoneyFlow>, DataError> {
        Ok(None)
    }

    async fn get_dragon_tiger(&self, _: &str) -> Result<Vec<DragonTigerEntry>, DataError> {
        Ok(vec![])
    }

    async fn get_lockup_schedule(&self, _: &str) -> Result<Vec<LockupSchedule>, DataError> {
        Ok(vec![])
    }

    async fn search_stock(&self, _: &str) -> Result<Vec<StockSearchResult>, DataError> {
        Ok(vec![])
    }

    // ── P3:mootdx 能力申报 ──
    // get_quote:实时TCP行情 → SynthesizeFromKline
    // get_klines:TCP协议,无日期参数 → Fallthrough(lib.rs按date字段截断)
    // 其他 stub:Fallthrough
    fn asof_capability(&self, method: &str) -> AsOfCapability {
        match method {
            "get_quote" => AsOfCapability::SynthesizeFromKline,
            _ => AsOfCapability::Fallthrough,
        }
    }
}

#[cfg(test)]
mod capability_tests {
    use super::*;

    fn make_vendor() -> MootdxVendor {
        MootdxVendor::default()
    }

    #[test]
    fn mootdx_quote_is_synthesize() {
        let v = make_vendor();
        assert_eq!(v.asof_capability("get_quote"), AsOfCapability::SynthesizeFromKline);
    }

    #[test]
    fn mootdx_others_are_fallthrough() {
        let v = make_vendor();
        for m in &[
            "get_klines",
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
