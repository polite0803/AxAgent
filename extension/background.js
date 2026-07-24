// AxAgent Wiki Clipper - Background Service Worker
// 负责：消息中转、native messaging 调用、上下文菜单管理、sidePanel 桥接、axagent:// 协议回退

const NATIVE_API_ID = "axagent.wiki.clipper";

// 上下文菜单 ID
const MENU_ASK_SELECTION = "axagent-ask-selection";
const MENU_CLIP_PAGE = "axagent-clip-page";
const MENU_OPEN_SIDEPANEL = "axagent-open-sidepanel";

// sidePanel 预设问题的 storage 键（与 sidepanel.js 保持一致）
const PENDING_QUESTION_KEY = "sidepanel_pending_question";

// ===================================================================
// 消息中转：处理来自 popup.js / sidepanel.js / content.js 的消息
// ===================================================================
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.action === "clipToWiki") {
    clipPage(message.wikiId, message.content)
      .then((result) => sendResponse({ success: true, result }))
      .catch((error) => sendResponse({ success: false, error: error.message }));
    return true;
  }

  if (message.action === "getActiveTabContent") {
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      if (tabs[0]) {
        chrome.tabs.sendMessage(
          tabs[0].id,
          { action: "getContent" },
          (response) => {
            sendResponse(response);
          },
        );
      } else {
        sendResponse(null);
      }
    });
    return true;
  }

  if (message.action === "extractContent") {
    // 来自 sidepanel.js / contextMenus 的内容提取请求
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      if (tabs[0]) {
        chrome.tabs.sendMessage(
          tabs[0].id,
          { action: "extractContent" },
          (response) => {
            sendResponse(response);
          },
        );
      } else {
        sendResponse(null);
      }
    });
    return true;
  }

  if (message.action === "askAxAgent") {
    // 来自 sidepanel.js 的提问请求
    askAxAgent(message.question, message.context)
      .then((result) => sendResponse(result))
      .catch((error) => sendResponse({ success: false, error: error.message }));
    return true;
  }

  if (message.action === "saveSettings") {
    chrome.storage.local.set(message.settings, () => {
      sendResponse({ success: true });
    });
    return true;
  }

  if (message.action === "getSettings") {
    chrome.storage.local.get(["wikiId", "autoClip"], (result) => {
      sendResponse(result);
    });
    return true;
  }
});

// ===================================================================
// Native messaging 调用
// ===================================================================

// 剪藏页面到 Wiki
async function clipPage(wikiId, pageData) {
  const payload = {
    action: "clip",
    wikiId: wikiId,
    source: {
      url: pageData.url,
      title: pageData.title,
      author: pageData.author || "",
      siteName: pageData.siteName || "",
      publishDate: pageData.publishDate || "",
      excerpt: pageData.excerpt || "",
      text: pageData.text || "",
      selection: pageData.selection || null,
    },
    clippedAt: new Date().toISOString(),
  };

  try {
    const response = await chrome.runtime.sendNativeMessage(
      NATIVE_API_ID,
      payload,
    );
    return response;
  } catch (error) {
    console.error("剪藏失败:", error);
    // 回退：通过 axagent:// 协议打开桌面端
    const fallbackUrl = `axagent://clip?wiki=${encodeURIComponent(wikiId)}&url=${
      encodeURIComponent(
        pageData.url,
      )
    }&title=${encodeURIComponent(pageData.title)}`;
    await chrome.tabs.create({ url: fallbackUrl, active: false });
    return { fallbackUsed: true, url: fallbackUrl };
  }
}

// 向 AxAgent 提问（sidePanel 用）
async function askAxAgent(question, context) {
  const payload = {
    action: "ask",
    question,
    context: {
      title: context?.title || "",
      url: context?.url || "",
    },
    askedAt: new Date().toISOString(),
  };

  try {
    const response = await chrome.runtime.sendNativeMessage(
      NATIVE_API_ID,
      payload,
    );
    // native messaging 一次性返回（当前不支持流式）
    return { success: true, answer: response?.answer || response?.text || "" };
  } catch (error) {
    console.error("提问失败，回退到 axagent:// 协议:", error);
    // 回退：构造 axagent://ask URL，由 sidepanel 展示给用户点击
    const params = new URLSearchParams();
    if (context && context.url) {
      params.set("url", context.url);
      params.set("title", context.title || "");
    }
    params.set("q", question);
    const fallbackUrl = `axagent://ask?${params.toString()}`;
    return { fallback: true, url: fallbackUrl };
  }
}

// ===================================================================
// 上下文菜单（contextMenus）
// ===================================================================

chrome.runtime.onInstalled.addListener(() => {
  // 初始化默认设置
  chrome.storage.local.get(
    ["wikiId", "autoClip", "clipDelay"],
    (result) => {
      const defaults = {};
      if (result.wikiId === undefined) { defaults.wikiId = ""; }
      if (result.autoClip === undefined) { defaults.autoClip = false; }
      if (result.clipDelay === undefined) { defaults.clipDelay = 2000; }
      if (Object.keys(defaults).length > 0) {
        chrome.storage.local.set(defaults);
      }
    },
  );

  // 创建 3 个上下文菜单
  chrome.contextMenus.removeAll(() => {
    chrome.contextMenus.create({
      id: MENU_ASK_SELECTION,
      title: "用 AxAgent 提问选中内容",
      contexts: ["selection"],
    });
    chrome.contextMenus.create({
      id: MENU_CLIP_PAGE,
      title: "剪藏到 AxAgent 知识库",
      contexts: ["page"],
    });
    chrome.contextMenus.create({
      id: MENU_OPEN_SIDEPANEL,
      title: "在侧边栏打开 AxAgent",
      contexts: ["page"],
    });
  });
});

// 上下文菜单点击处理
chrome.contextMenus.onClicked.addListener((info, tab) => {
  switch (info.menuItemId) {
    case MENU_ASK_SELECTION:
      handleAskSelection(info.selectionText, tab);
      break;
    case MENU_CLIP_PAGE:
      handleClipPage(tab);
      break;
    case MENU_OPEN_SIDEPANEL:
      openSidePanel();
      break;
    default:
      break;
  }
});

// "用 AxAgent 提问选中内容"：写入预设问题 + 打开 sidePanel
async function handleAskSelection(selectionText, tab) {
  const question = selectionText ? selectionText.trim() : "";
  if (!question) { return; }
  // 先写入预设问题，sidePanel 加载后会消费
  await chrome.storage.local.set({ [PENDING_QUESTION_KEY]: question });
  openSidePanel();
}

// "剪藏到 AxAgent 知识库"：从 content.js 提取内容 + sendNativeMessage 剪藏
async function handleClipPage(tab) {
  try {
    const { wikiId } = await chrome.storage.local.get(["wikiId"]);
    const targetWikiId = wikiId || "";
    // 通过 content.js 提取页面内容
    const content = await chrome.tabs.sendMessage(tab.id, {
      action: "extractContent",
    });
    if (!content) {
      console.warn("无法提取页面内容");
      return;
    }
    await clipPage(targetWikiId, content);
  } catch (error) {
    console.error("剪藏页面失败:", error);
    // 回退：直接用 tab 信息构造最小 payload 走 axagent://
    const fallbackUrl = `axagent://clip?url=${
      encodeURIComponent(
        tab.url || "",
      )
    }&title=${encodeURIComponent(tab.title || "")}`;
    await chrome.tabs.create({ url: fallbackUrl, active: false });
  }
}

// "在侧边栏打开 AxAgent"：打开 sidePanel
function openSidePanel() {
  // chrome.sidePanel.open() 必须在用户交互上下文中调用
  if (chrome.sidePanel && chrome.sidePanel.open) {
    chrome.sidePanel.open().catch((err) => {
      console.error("打开 sidePanel 失败:", err);
    });
  }
}

// 点击扩展图标时也打开 sidePanel（可选行为，不打开 popup）
// 注：manifest 中保留了 default_popup，所以点击图标仍打开 popup；
// 如果想让点击图标直接打开 sidePanel，可移除 default_popup 并启用下面的逻辑。
