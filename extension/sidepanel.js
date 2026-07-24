// AxAgent 侧边栏脚本
// 负责：消息列表 UI、通过 background 中转调用 native messaging、
// 自动获取当前标签页上下文、消息历史持久化、native messaging 失败回退到 axagent:// 协议

(function() {
  "use strict";

  // 消息历史在 chrome.storage.local 中的键
  const STORAGE_KEY = "sidepanel_messages";
  // 预设问题在 chrome.storage.local 中的键（由 contextMenus 写入）
  const PENDING_QUESTION_KEY = "sidepanel_pending_question";
  // native messaging 失败时回退用的协议前缀
  const FALLBACK_PROTOCOL = "axagent://ask";

  // DOM 元素引用
  const messageList = document.getElementById("messageList");
  const userInput = document.getElementById("userInput");
  const sendBtn = document.getElementById("sendBtn");
  const clearBtn = document.getElementById("clearBtn");
  const emptyTip = document.getElementById("emptyTip");
  const contextBar = document.getElementById("contextBar");
  const contextTitle = document.getElementById("contextTitle");
  const contextUrl = document.getElementById("contextUrl");

  // 当前标签页上下文
  let currentContext = null;
  // 是否正在等待响应
  let isWaiting = false;

  // 获取当前 active tab 的 title + url 作为上下文
  async function loadCurrentTabContext() {
    try {
      const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
      if (tabs && tabs[0]) {
        const tab = tabs[0];
        currentContext = {
          title: tab.title || "",
          url: tab.url || "",
        };
        if (currentContext.title || currentContext.url) {
          contextTitle.textContent = currentContext.title || "(无标题)";
          contextUrl.textContent = currentContext.url || "";
          contextBar.hidden = false;
        }
      }
    } catch (error) {
      // 获取上下文失败不阻塞对话，仅记录日志
      console.warn("获取当前标签页上下文失败:", error);
    }
  }

  // 从 chrome.storage.local 读取消息历史
  function loadHistory() {
    return new Promise((resolve) => {
      chrome.storage.local.get([STORAGE_KEY], (result) => {
        resolve(result[STORAGE_KEY] || []);
      });
    });
  }

  // 保存消息历史到 chrome.storage.local
  function saveHistory(messages) {
    return new Promise((resolve) => {
      chrome.storage.local.set({ [STORAGE_KEY]: messages }, resolve);
    });
  }

  // 渲染单条消息
  function appendMessage(message) {
    if (emptyTip && emptyTip.parentNode) {
      emptyTip.parentNode.removeChild(emptyTip);
    }
    const wrap = document.createElement("div");
    wrap.className = `message ${message.role}`;
    const role = document.createElement("div");
    role.className = "message-role";
    role.textContent = message.role === "user" ? "我" : "AxAgent";
    const bubble = document.createElement("div");
    bubble.className = "message-bubble";
    if (message.kind) {
      bubble.classList.add(message.kind); // error / fallback
    }
    if (message.role === "assistant" && message.fallbackUrl) {
      // 回退模式：显示可点击的 axagent:// 链接
      const link = document.createElement("a");
      link.href = message.fallbackUrl;
      link.textContent = message.text;
      link.target = "_blank";
      bubble.appendChild(link);
    } else {
      bubble.textContent = message.text;
    }
    wrap.appendChild(role);
    wrap.appendChild(bubble);
    messageList.appendChild(wrap);
    // 自动滚动到底部
    messageList.scrollTop = messageList.scrollHeight;
  }

  // 渲染全部历史
  function renderHistory(messages) {
    messages.forEach(appendMessage);
  }

  // 调用 background 中转，向 AxAgent 桌面端发送问题
  // 复用 popup.js 的 native messaging 中转模式（background.js 负责实际 sendNativeMessage）
  async function askAxAgent(question) {
    const payload = {
      action: "askAxAgent",
      question,
      context: currentContext,
    };
    return await chrome.runtime.sendMessage(payload);
  }

  // 构造 native messaging 失败时的回退 URL
  function buildFallbackUrl(question) {
    const params = new URLSearchParams();
    if (currentContext && currentContext.url) {
      params.set("url", currentContext.url);
      params.set("title", currentContext.title);
    }
    params.set("q", question);
    return `${FALLBACK_PROTOCOL}?${params.toString()}`;
  }

  // 发送一条消息
  async function sendMessage() {
    const question = userInput.value.trim();
    if (!question || isWaiting) {
      return;
    }
    isWaiting = true;
    sendBtn.disabled = true;
    userInput.value = "";

    // 追加用户消息
    const userMsg = { role: "user", text: question, ts: Date.now() };
    appendMessage(userMsg);

    // 追加占位助手消息
    const placeholderWrap = document.createElement("div");
    placeholderWrap.className = "message assistant";
    const placeholderRole = document.createElement("div");
    placeholderRole.className = "message-role";
    placeholderRole.textContent = "AxAgent";
    const placeholderBubble = document.createElement("div");
    placeholderBubble.className = "message-bubble";
    placeholderBubble.textContent = "正在思考…";
    placeholderWrap.appendChild(placeholderRole);
    placeholderWrap.appendChild(placeholderBubble);
    messageList.appendChild(placeholderWrap);
    messageList.scrollTop = messageList.scrollHeight;

    // 持久化（用户消息先入库）
    const history = await loadHistory();
    history.push(userMsg);

    let assistantMsg;
    try {
      const resp = await askAxAgent(question);
      if (resp && resp.fallback) {
        // native messaging 失败，回退到 axagent:// 协议
        const fallbackUrl = resp.url || buildFallbackUrl(question);
        assistantMsg = {
          role: "assistant",
          text: "本地通信不可用，已通过 axagent:// 协议回退。点击打开桌面端：",
          fallbackUrl,
          kind: "fallback",
          ts: Date.now(),
        };
      } else if (resp && resp.success) {
        // 一次性返回（native messaging 当前不支持流式，这里用一次性响应）
        assistantMsg = {
          role: "assistant",
          text: resp.answer || resp.result?.answer || "(空响应)",
          ts: Date.now(),
        };
      } else {
        assistantMsg = {
          role: "assistant",
          text: resp && resp.error ? resp.error : "未知错误",
          kind: "error",
          ts: Date.now(),
        };
      }
    } catch (error) {
      assistantMsg = {
        role: "assistant",
        text: error.message || String(error),
        kind: "error",
        ts: Date.now(),
      };
    }

    // 替换占位气泡
    messageList.removeChild(placeholderWrap);
    appendMessage(assistantMsg);

    // 持久化助手消息
    history.push(assistantMsg);
    await saveHistory(history);

    isWaiting = false;
    sendBtn.disabled = false;
    userInput.focus();
  }

  // 清空对话
  async function clearConversation() {
    if (!confirm("确定清空全部对话历史？")) {
      return;
    }
    await saveHistory([]);
    messageList.innerHTML = "";
    const tip = document.createElement("div");
    tip.className = "empty-tip";
    tip.id = "emptyTip";
    tip.textContent = "向 AxAgent 提问，可结合当前网页上下文。";
    messageList.appendChild(tip);
  }

  // 读取并消费 contextMenus 写入的预设问题
  async function consumePendingQuestion() {
    return new Promise((resolve) => {
      chrome.storage.local.get([PENDING_QUESTION_KEY], async (result) => {
        const q = result[PENDING_QUESTION_KEY];
        if (q) {
          // 消费后清除
          await chrome.storage.local.remove([PENDING_QUESTION_KEY]);
        }
        resolve(q || "");
      });
    });
  }

  // 监听 storage 变化，支持 sidePanel 已打开时再触发"提问选中内容"
  chrome.storage.onChanged.addListener((changes, area) => {
    if (area !== "local") { return; }
    if (changes[PENDING_QUESTION_KEY] && changes[PENDING_QUESTION_KEY].newValue) {
      const q = changes[PENDING_QUESTION_KEY].newValue;
      // 立即清除，避免重复触发
      chrome.storage.local.remove([PENDING_QUESTION_KEY]);
      userInput.value = q;
      sendMessage();
    }
  });

  // 初始化
  async function init() {
    await loadCurrentTabContext();
    const history = await loadHistory();
    if (history.length > 0) {
      renderHistory(history);
    }
    // 消费预设问题（来自 contextMenus 的"用 AxAgent 提问选中内容"）
    const pending = await consumePendingQuestion();
    if (pending) {
      userInput.value = pending;
      // 自动发送
      sendMessage();
    }
  }

  // 事件绑定
  sendBtn.addEventListener("click", sendMessage);
  clearBtn.addEventListener("click", clearConversation);
  userInput.addEventListener("keydown", (e) => {
    // 回车发送，Shift+回车换行
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  });

  // 启动
  document.addEventListener("DOMContentLoaded", init);
})();
