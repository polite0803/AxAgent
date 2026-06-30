import { chromium } from "playwright";

let browser = null;
let page = null;

async function init() {
  // 反检测启动参数：隐藏 headless Chromium 特征
  browser = await chromium.launch({
    headless: true,
    args: [
      "--disable-blink-features=AutomationControlled",
      "--no-sandbox",
      "--disable-dev-shm-usage",
      "--disable-web-security",
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

process.stdin.on("data", async (data) => {
  const msg = JSON.parse(data.toString().trim());
  let result;

  try {
    switch (msg.method) {
      case "navigate": {
        await page.goto(msg.params.url, { waitUntil: "domcontentloaded", timeout: 30000 });
        result = { url: page.url(), title: await page.title() };
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
          if (resp.body && resp.body.length > 3000) {
            resp.body = resp.body.slice(0, 3000);
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
        result = { html: html.slice(0, 100000) };
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
});

init().then(() => {
  process.stdout.write(JSON.stringify({ ready: true }) + "\n");
});
