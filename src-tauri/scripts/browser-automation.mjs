import { chromium } from "playwright";
import { createInterface } from "readline";

let browser = null;
let page = null;

async function init() {
  // 反检测启动参数：隐藏 headless Chromium 特征
  // 注意：不要使用 --disable-web-security，否则会关闭同源策略，
  // 放大 SSRF/跨站风险（SSRF 已在 Rust 侧通过 validate_browser_url 兜底）。
  browser = await chromium.launch({
    headless: true,
    args: [
      "--disable-blink-features=AutomationControlled",
      "--no-sandbox",
      "--disable-dev-shm-usage",
      "--disable-features=IsolateOrigins,site-per-process",
    ],
  });
  const context = await browser.newContext({
    viewport: { width: 1280, height: 720 },
    locale: "zh-CN",
    userAgent:
      "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
  });
  page = await context.newPage();
  // 注入反检测脚本：覆盖 WebDriver / Chrome / Plugins / Languages 等特征
  await page.addInitScript(() => {
    // 隐藏 navigator.webdriver（最关键的检测项）
    Object.defineProperty(navigator, "webdriver", { get: () => false });
    // 伪装 navigator.plugins（WAF 检测无插件=自动化）
    Object.defineProperty(navigator, "plugins", {
      get: () => [1, 2, 3, 4, 5],
    });
    // 伪装 navigator.languages
    Object.defineProperty(navigator, "languages", {
      get: () => ["zh-CN", "zh"],
    });
    // 覆盖 chrome.runtime（某些 WAF 检测 chrome 对象）
    window.chrome = { runtime: {} };
  });
}

// 按行分帧：Rust 端每条请求以 '\n' 结尾。readline 保证整行边界，
// 避免 TCP 分片/粘包导致 JSON 解析失败或响应错位（修复 #5）。
// 通过串行 promise 链保证响应顺序与请求 id 一一对应。
const rl = createInterface({ input: process.stdin, crlfDelay: Infinity });

let chain = Promise.resolve();
rl.on("line", (line) => {
  const trimmed = line.trim();
  if (!trimmed) return;
  chain = chain
    .then(() => handleLine(trimmed))
    .catch((e) => {
      process.stdout.write(
        JSON.stringify({ id: null, error: `handler error: ${e.message}` }) + "\n",
      );
    });
});

async function handleLine(line) {
  let msg;
  try {
    msg = JSON.parse(line);
  } catch (parseErr) {
    process.stdout.write(
      JSON.stringify({ id: null, error: `JSON parse error: ${parseErr.message}` }) + "\n",
    );
    return;
  }
  let result;

  try {
    switch (msg.method) {
      case "navigate": {
        await page.goto(msg.params.url, { waitUntil: "domcontentloaded", timeout: 30000 });
        result = { url: page.url(), title: await page.title() };
        break;
      }
      case "evaluate": {
        // 参数化执行：code 为函数体字符串，arg 作为第二个参数传入，避免字符串拼接注入
        const out = await page.evaluate(msg.params.code, msg.params.arg);
        result = out;
        break;
      }
      case "http_json": {
        // 通过当前页面的 fetch() 发送 GET 请求获取 JSON。
        // 不导航离当前页面，保持 cookies/fingerprint 有效，
        // 比 page.goto() 更不容易触发 WAF。
        try {
          const resp = await page.evaluate(async (url) => {
            const r = await fetch(url, {
              credentials: "include",
              headers: { "Accept": "application/json, text/plain, */*" },
            });
            return { ok: r.ok, status: r.status, body: await r.text() };
          }, msg.params.url);
          // 不再静默截断 JSON（避免下游解析失败）；超大响应做有损截断并显式标记
          const MAX_BODY = 4_000_000;
          if (resp.body && resp.body.length > MAX_BODY) {
            resp.body = resp.body.slice(0, MAX_BODY);
            resp.truncated = true;
          }
          result = resp;
        } catch (fetchErr) {
          result = { ok: false, status: 0, body: `FETCH_ERROR: ${fetchErr.message}` };
        }
        break;
      }
      case "screenshot": {
        const buffer = await page.screenshot({ type: "png", fullPage: msg.params.fullPage || false });
        result = { image_base64: buffer.toString("base64") };
        break;
      }
      case "click": {
        await page.click(msg.params.selector, { timeout: 10000 });
        result = { success: true };
        break;
      }
      case "fill": {
        await page.fill(msg.params.selector, msg.params.value);
        result = { success: true };
        break;
      }
      case "type": {
        await page.locator(msg.params.selector).pressSequentially(msg.params.text, { delay: 50 });
        result = { success: true };
        break;
      }
      case "select": {
        await page.selectOption(msg.params.selector, msg.params.value);
        result = { success: true };
        break;
      }
      case "extract_text": {
        const text = await page.locator(msg.params.selector).textContent();
        result = { text };
        break;
      }
      case "extract_all": {
        const elements = await page.$$eval(msg.params.selector, (els) =>
          els.map((el) => ({
            tag: el.tagName.toLowerCase(),
            text: el.textContent?.trim().slice(0, 200),
            href: el.getAttribute("href"),
            type: el.getAttribute("type"),
            placeholder: el.getAttribute("placeholder"),
          })));
        result = { elements, count: elements.length };
        break;
      }
      case "wait_for": {
        await page.waitForSelector(msg.params.selector, { timeout: msg.params.timeout || 10000 });
        result = { success: true };
        break;
      }
      case "get_content": {
        const html = await page.content();
        // 不再静默截断；超大 HTML 做有损截断并显式标记
        const MAX_HTML = 4_000_000;
        if (html.length > MAX_HTML) {
          result = { html: html.slice(0, MAX_HTML), truncated: true };
        } else {
          result = { html };
        }
        break;
      }
      case "close": {
        await browser.close();
        result = { success: true };
        break;
      }
      default:
        throw new Error(`Unknown method: ${msg.method}`);
    }

    process.stdout.write(JSON.stringify({ id: msg.id, result }) + "\n");
  } catch (error) {
    process.stdout.write(JSON.stringify({ id: msg.id, error: error.message }) + "\n");
  }
}

init().then(() => {
  process.stdout.write(JSON.stringify({ ready: true }) + "\n");
});
