use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::message_gateway::platform_config::PlatformConfig;
use crate::message_gateway::platforms::PlatformAdapter;

pub struct WeChatAdapter {
    connected: Arc<AtomicBool>,
    poll_task: Mutex<Option<JoinHandle<()>>>,
    running: Arc<AtomicBool>,
}

impl WeChatAdapter {
    pub fn new() -> Self {
        Self {
            connected: Arc::new(AtomicBool::new(false)),
            poll_task: Mutex::new(None),
            running: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[async_trait::async_trait]
impl PlatformAdapter for WeChatAdapter {
    fn name(&self) -> &'static str {
        "wechat"
    }

    fn is_enabled(&self, config: &PlatformConfig) -> bool {
        config.wechat_enabled
            && config.wechat_app_id.is_some()
            && config.wechat_app_secret.is_some()
    }

    async fn start(&self, config: &PlatformConfig) -> anyhow::Result<()> {
        if !self.is_enabled(config) {
            anyhow::bail!("WeChat is not enabled or missing credentials");
        }
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        let mode = resolve_wechat_mode(config);

        if mode == "customer_service" {
            start_customer_service_polling(
                &self.connected,
                &self.poll_task,
                &self.running,
                config,
            )
            .await
        } else {
            // official_account 模式: 依赖 webhook server 接收消息
            // 适配器只需要准备好发送消息的能力
            tracing::info!(
                "WeChat: official_account mode — 消息接收依赖 webhook endpoint"
            );
            self.running.store(true, Ordering::SeqCst);
            self.connected.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    async fn stop(&self) -> anyhow::Result<()> {
        self.running.store(false, Ordering::SeqCst);
        if let Some(task) = self.poll_task.lock().await.take() {
            task.abort();
            let _ = task.await;
        }
        self.connected.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    async fn send_message(
        &self,
        config: &PlatformConfig,
        open_id: &str,
        text: &str,
        _parse_mode: Option<&str>,
    ) -> anyhow::Result<()> {
        let app_id = config.wechat_app_id.clone().unwrap_or_default();
        let app_secret = config.wechat_app_secret.clone().unwrap_or_default();

        let access_token = fetch_wechat_token(&reqwest::Client::new(), &app_id, &app_secret)
            .await
            .ok_or_else(|| anyhow::anyhow!("WeChat: failed to fetch access_token"))?;

        send_wechat_custom_message(&access_token, open_id, text).await
    }
}

impl Default for WeChatAdapter {
    fn default() -> Self {
        Self::new()
    }
}

/// 解析 wechat_mode 配置，默认为 official_account
fn resolve_wechat_mode(config: &PlatformConfig) -> &str {
    config
        .wechat_mode
        .as_deref()
        .unwrap_or("official_account")
}

async fn start_customer_service_polling(
    connected: &Arc<AtomicBool>,
    poll_task: &Mutex<Option<JoinHandle<()>>>,
    running: &Arc<AtomicBool>,
    config: &PlatformConfig,
) -> anyhow::Result<()> {
    let app_id = config.wechat_app_id.clone().unwrap_or_default();
    let app_secret = config.wechat_app_secret.clone().unwrap_or_default();

    let connected = connected.clone();
    let running = running.clone();

    running.store(true, Ordering::SeqCst);

    let task = tokio::spawn(async move {
        let client = reqwest::Client::new();

        let mut access_token = match fetch_wechat_token(&client, &app_id, &app_secret).await {
            Some(t) => {
                tracing::info!("WeChat: access_token obtained");
                connected.store(true, Ordering::SeqCst);
                t
            },
            None => {
                tracing::error!("WeChat: failed to obtain access_token");
                connected.store(false, Ordering::SeqCst);
                running.store(false, Ordering::SeqCst);
                return;
            },
        };

        let mut token_expiry = chrono::Utc::now().timestamp() + 7000;

        loop {
            if !running.load(Ordering::SeqCst) {
                break;
            }

            let now = chrono::Utc::now().timestamp();
            if now > token_expiry {
                match fetch_wechat_token(&client, &app_id, &app_secret).await {
                    Some(t) => {
                        access_token = t;
                        token_expiry = now + 7000;
                        connected.store(true, Ordering::SeqCst);
                        tracing::info!("WeChat: access_token refreshed");
                    },
                    None => {
                        tracing::error!("WeChat: token refresh failed");
                        connected.store(false, Ordering::SeqCst);
                        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                        continue;
                    },
                }
            }

            if let Ok(msgs) = poll_wechat_cs_messages(&client, &access_token).await {
                for (openid, text) in msgs {
                    tracing::info!("WeChat CS msg: {} from {}", text, openid);

                    if let Some(cb) = crate::message_gateway::platforms::get_message_callback() {
                        let at = access_token.clone();
                        let oid = openid.clone();
                        let t = text.clone();
                        tokio::spawn(async move {
                            let reply = cb.on_message("wechat", &oid, None, &oid, &t).await;
                            if let Some(reply_text) = reply {
                                let _ = send_wechat_custom_message(&at, &oid, &reply_text).await;
                            }
                        });
                    }
                }
            }

            if running.load(Ordering::SeqCst) {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
        }

        connected.store(false, Ordering::SeqCst);
        tracing::info!("WeChat CS poll loop stopped");
    });

    *poll_task.lock().await = Some(task);
    Ok(())
}

// ── Public webhook handlers for WeChat official_account mode ──

/// 处理微信服务器验证请求 (GET)
/// 微信在配置服务器 URL 时会发送 GET 请求验证
pub fn verify_server(
    token: &str,
    signature: &str,
    timestamp: &str,
    nonce: &str,
    echostr: &str,
) -> Result<String, String> {
    let mut items = vec![token.to_string(), timestamp.to_string(), nonce.to_string()];
    items.sort();
    let combined = items.join("");
    use sha1::Digest;
    let mut hasher = sha1::Sha1::new();
    hasher.update(combined.as_bytes());
    let digest = format!("{:x}", hasher.finalize());

    if digest != signature.to_lowercase() {
        return Err("签名验证失败".to_string());
    }

    Ok(echostr.to_string())
}

/// 处理微信服务器 POST 的消息 XML
/// 解析 XML 并将消息通过回调转发给 Agent 处理
pub async fn handle_official_account_message(
    config: &PlatformConfig,
    xml_body: &str,
) -> Result<String, String> {
    let doc = roxmltree::Document::parse(xml_body)
        .map_err(|e| format!("XML 解析失败: {}", e))?;

    let root = doc.root();
    let msg_type = root
        .children()
        .find(|n| n.has_tag_name("MsgType"))
        .and_then(|n| n.text())
        .unwrap_or("");

    if msg_type != "text" {
        // 非文本消息返回空（微信会忽略空回复）
        return Ok("success".to_string());
    }

    let from_user = root
        .children()
        .find(|n| n.has_tag_name("FromUserName"))
        .and_then(|n| n.text())
        .unwrap_or("");
    let content = root
        .children()
        .find(|n| n.has_tag_name("Content"))
        .and_then(|n| n.text())
        .unwrap_or("");

    if from_user.is_empty() || content.is_empty() {
        return Ok("success".to_string());
    }

    tracing::info!("WeChat official account: {} — {}", from_user, content);

    if let Some(cb) = crate::message_gateway::platforms::get_message_callback() {
        let app_id = config.wechat_app_id.clone().unwrap_or_default();
        let app_secret = config.wechat_app_secret.clone().unwrap_or_default();
        let from = from_user.to_string();
        let text = content.to_string();

        tokio::spawn(async move {
            let reply = cb.on_message("wechat", &from, None, &from, &text).await;
            if let Some(reply_text) = reply {
                let client = reqwest::Client::new();
                if let Some(token) =
                    fetch_wechat_token(&client, &app_id, &app_secret).await
                {
                    let _ = send_wechat_custom_message(&token, &from, &reply_text).await;
                }
            }
        });
    }

    // 返回 success 让微信知道已处理，实际回复通过客服消息异步发送
    Ok("success".to_string())
}

// ── helper functions ──

async fn fetch_wechat_token(
    client: &reqwest::Client,
    app_id: &str,
    app_secret: &str,
) -> Option<String> {
    let url = format!(
        "https://api.weixin.qq.com/cgi-bin/token?grant_type=client_credential&appid={}&secret={}",
        app_id, app_secret
    );
    let resp = client.get(&url).send().await.ok()?;
    let body: serde_json::Value = resp.json().await.ok()?;
    body.get("access_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

async fn poll_wechat_cs_messages(
    client: &reqwest::Client,
    access_token: &str,
) -> anyhow::Result<Vec<(String, String)>> {
    let url = format!(
        "https://api.weixin.qq.com/cgi-bin/message/custom/receive?access_token={}",
        access_token
    );

    let body = serde_json::json!({
        "action": "get_message",
        "limit": 10
    });

    let resp = client.post(&url).json(&body).send().await?;
    let json: serde_json::Value = resp.json().await?;

    let mut results = Vec::new();

    if let Some(records) = json["record_list"].as_array() {
        for record in records {
            let openid = record["openid"].as_str().unwrap_or("");
            let text = record["text"]["content"].as_str().unwrap_or("");
            if !openid.is_empty() && !text.is_empty() {
                results.push((openid.to_string(), text.to_string()));
            }
        }
    }

    Ok(results)
}

async fn send_wechat_custom_message(
    access_token: &str,
    open_id: &str,
    text: &str,
) -> anyhow::Result<()> {
    let url = format!(
        "https://api.weixin.qq.com/cgi-bin/message/custom/send?access_token={}",
        access_token
    );
    let body = serde_json::json!({
        "touser": open_id,
        "msgtype": "text",
        "text": {
            "content": text
        }
    });

    let client = reqwest::Client::new();
    let resp = client.post(&url).json(&body).send().await?;
    let resp_body: serde_json::Value = resp.json().await?;

    if let Some(errcode) = resp_body.get("errcode").and_then(|v| v.as_i64()) {
        if errcode != 0 {
            anyhow::bail!(
                "WeChat send failed: errcode={}, errmsg={:?}",
                errcode,
                resp_body.get("errmsg")
            );
        }
    }
    Ok(())
}
