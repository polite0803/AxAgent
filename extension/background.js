// AxAgent Wiki Clipper - Background Service Worker

const TARIFF_API_ID = "axagent.wiki.clipper";

// Listen for messages from popup or content script
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.action === "clipToWiki") {
    clipPage(message.wikiId, message.content)
      .then(result => sendResponse({ success: true, result }))
      .catch(error => sendResponse({ success: false, error: error.message }));
    return true;
  }

  if (message.action === "getActiveTabContent") {
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      if (tabs[0]) {
        chrome.tabs.sendMessage(tabs[0].id, { action: "getContent" }, (response) => {
          sendResponse(response);
        });
      }
    });
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
    const response = await chrome.runtime.sendNativeMessage(TARIFF_API_ID, payload);
    return response;
  } catch (error) {
    console.error("Failed to clip page:", error);

    // Fallback: send via external protocol if native messaging fails
    const fallbackUrl = `axagent://clip?wiki=${encodeURIComponent(wikiId)}&url=${
      encodeURIComponent(pageData.url)
    }&title=${encodeURIComponent(pageData.title)}`;

    // Try to open the fallback URL
    await chrome.tabs.create({ url: fallbackUrl, active: false });

    return { fallbackUsed: true, url: fallbackUrl };
  }
}

// Handle extension icon click
chrome.action.onClicked.addListener(async (tab) => {
  try {
    const response = await chrome.tabs.sendMessage(tab.id, { action: "getContent" });
    if (response && response.content) {
      // Open popup or process immediately
      chrome.storage.local.get(["wikiId"], (result) => {
        if (result.wikiId) {
          clipPage(result.wikiId, {
            ...response.content,
            selection: response.selection,
          });
        }
      });
    }
  } catch (error) {
    console.error("Failed to get tab content:", error);
  }
});

// Listen for installation
chrome.runtime.onInstalled.addListener((details) => {
  if (details.reason === "install") {
    chrome.storage.local.set({
      wikiId: "",
      autoClip: false,
      clipDelay: 2000,
    });
  }
});
