(function() {
  "use strict";

  function extractContent() {
    const result = {
      title: document.title || "",
      url: window.location.href,
      text: "",
      excerpt: "",
      author: "",
      publishDate: "",
      siteName: "",
    };

    // Get site name
    const siteNameMeta = document.querySelector('meta[property="og:site_name"]')
      || document.querySelector('meta[name="application-name"]');
    if (siteNameMeta) {
      result.siteName = siteNameMeta.content;
    } else {
      result.siteName = window.location.hostname;
    }

    // Get author
    const authorMeta = document.querySelector('meta[name="author"]')
      || document.querySelector('meta[property="article:author"]')
      || document.querySelector('meta[name="dc.creator"]');
    if (authorMeta) {
      result.author = authorMeta.content;
    }

    // Get publish date
    const dateMeta = document.querySelector('meta[property="article:published_time"]')
      || document.querySelector('meta[name="date"]')
      || document.querySelector('meta[name="dc.date"]')
      || document.querySelector("time[datetime]");
    if (dateMeta) {
      result.publishDate = dateMeta.content || dateMeta.getAttribute("datetime") || "";
    }

    // Extract main content
    const contentSelectors = [
      "article",
      '[role="main"]',
      "main",
      ".post-content",
      ".article-content",
      ".entry-content",
      ".content",
      "#content",
      ".post",
      ".article",
    ];

    let contentElement = null;
    for (const selector of contentSelectors) {
      const el = document.querySelector(selector);
      if (el && el.textContent.trim().length > 200) {
        contentElement = el;
        break;
      }
    }

    if (!contentElement) {
      contentElement = document.body;
    }

    // Remove unwanted elements
    const unwantedSelectors = [
      "script",
      "style",
      "nav",
      "header",
      "footer",
      "aside",
      ".advertisement",
      ".ad",
      ".sidebar",
      ".menu",
      ".comments",
      ".social-share",
      ".related-posts",
      ".author-bio",
      "iframe",
      "noscript",
      ".newsletter",
      ".subscription",
    ];

    const clone = contentElement.cloneNode(true);
    unwantedSelectors.forEach(selector => {
      clone.querySelectorAll(selector).forEach(el => el.remove());
    });

    // Get text content
    result.text = clone.textContent
      .replace(/\s+/g, " ")
      .trim()
      .substring(0, 50000);

    // Get excerpt
    const metaDesc = document.querySelector('meta[name="description"]')
      || document.querySelector('meta[property="og:description"]');
    if (metaDesc) {
      result.excerpt = metaDesc.content.substring(0, 500);
    } else {
      result.excerpt = result.text.substring(0, 300) + "...";
    }

    return result;
  }

  function getSelectedText() {
    const selection = window.getSelection();
    if (selection && selection.toString().trim()) {
      return {
        text: selection.toString().trim(),
        context: selection.anchorNode?.parentElement?.textContent?.substring(0, 200) || "",
      };
    }
    return null;
  }

  chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
    // 原有：popup.js 通过 background 中转请求完整内容
    if (request.action === "getContent") {
      const content = extractContent();
      const selection = getSelectedText();
      sendResponse({
        content,
        selection,
      });
      return true;
    }

    // 新增：响应来自 background.js / sidepanel.js 的内容提取请求
    // 仅返回轻量字段（title/url/text/excerpt），供 sidePanel 上下文与剪藏使用
    if (request.action === "extractContent") {
      const content = extractContent();
      const selection = getSelectedText();
      sendResponse({
        title: content.title,
        url: content.url,
        text: content.text,
        excerpt: content.excerpt,
        author: content.author,
        publishDate: content.publishDate,
        siteName: content.siteName,
        selection: selection ? selection.text : null,
      });
      return true;
    }

    return true;
  });
})();
