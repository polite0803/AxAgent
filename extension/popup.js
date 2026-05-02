document.addEventListener("DOMContentLoaded", async () => {
  const wikiIdInput = document.getElementById("wikiId");
  const previewTitle = document.getElementById("previewTitle");
  const previewUrl = document.getElementById("previewUrl");
  const previewExcerpt = document.getElementById("previewExcerpt");
  const clipBtn = document.getElementById("clipBtn");
  const status = document.getElementById("status");

  let currentContent = null;

  // Load saved settings
  chrome.storage.local.get(["wikiId"], (result) => {
    if (result.wikiId) {
      wikiIdInput.value = result.wikiId;
    }
  });

  // Get current tab content
  try {
    const response = await chrome.runtime.sendMessage({ action: "getActiveTabContent" });
    if (response && response.content) {
      currentContent = response;
      previewTitle.textContent = response.content.title || "Untitled";
      previewUrl.textContent = response.content.url || "";
      previewExcerpt.textContent = response.content.excerpt || "No excerpt available";
      clipBtn.disabled = !wikiIdInput.value;
    } else {
      previewTitle.textContent = "Unable to get content";
      previewUrl.textContent = "";
      previewExcerpt.textContent = "Please try again or navigate to a page with content.";
    }
  } catch (error) {
    previewTitle.textContent = "Error loading content";
    previewExcerpt.textContent = error.message;
  }

  // Enable clip button when wiki ID is entered
  wikiIdInput.addEventListener("input", () => {
    clipBtn.disabled = !wikiIdInput.value || !currentContent;
  });

  // Save wiki ID when changed
  wikiIdInput.addEventListener("change", () => {
    chrome.storage.local.set({ wikiId: wikiIdInput.value });
  });

  // Handle clip button click
  clipBtn.addEventListener("click", async () => {
    if (!currentContent || !wikiIdInput.value) { return; }

    clipBtn.disabled = true;
    clipBtn.textContent = "Clipping...";
    status.textContent = "";
    status.className = "status";

    try {
      const response = await chrome.runtime.sendMessage({
        action: "clipToWiki",
        wikiId: wikiIdInput.value,
        content: {
          ...currentContent.content,
          selection: currentContent.selection,
        },
      });

      if (response.success) {
        status.textContent = "Clipped successfully!";
        status.className = "status success";
        clipBtn.textContent = "Clipped!";

        setTimeout(() => {
          window.close();
        }, 1500);
      } else {
        throw new Error(response.error || "Failed to clip");
      }
    } catch (error) {
      status.textContent = `Error: ${error.message}`;
      status.className = "status error";
      clipBtn.disabled = false;
      clipBtn.textContent = "Clip to Wiki";
    }
  });

  // Handle settings link
  document.getElementById("settingsLink").addEventListener("click", (e) => {
    e.preventDefault();
    chrome.runtime.openOptionsPage();
  });
});
