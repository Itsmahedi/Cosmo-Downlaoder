/**
 * âœ¦ Cosmo Downloader v1.7 â€” Popup Script
 */

"use strict";

const statusDot = document.getElementById("statusDot");
const statusBar = document.getElementById("statusBar");
const statusText = document.getElementById("statusText");
const toggleIntercept = document.getElementById("toggleIntercept");
const interceptedCountEl = document.getElementById("interceptedCount");
const urlInput = document.getElementById("urlInput");
const addUrlBtn = document.getElementById("addUrlBtn");
const urlFeedback = document.getElementById("urlFeedback");

const runtime = typeof browser !== "undefined" ? browser : chrome;

/**
 * Update UI with current status
 */
function updateStatus() {
  runtime.runtime.sendMessage({ action: "getStatus" }, (response) => {
    const lastErr = runtime.runtime.lastError;
    if (lastErr || !response) {
      checkHttpFallback();
      return;
    }

    if (response.connected) {
      setConnected(true);
    } else {
      checkHttpFallback();
    }

    if (response.interceptEnabled !== undefined) {
      toggleIntercept.checked = response.interceptEnabled;
    }
    interceptedCountEl.textContent = response.interceptedCount || 0;
  });
}

function checkHttpFallback() {
  fetch("http://127.0.0.1:42424/health", { method: "GET" })
    .then((r) => r.json())
    .then((data) => {
      if (data && data.status === "ok") {
        setConnected(true);
      } else {
        setConnected(false);
      }
    })
    .catch(() => {
      setConnected(false);
    });
}

function setConnected(connected) {
  if (connected) {
    statusDot.classList.add("connected");
    statusDot.title = "Connected to Cosmo Downloader Engine";
    statusBar.classList.add("connected");
    statusText.textContent = "Connected to Cosmo Downloader \u2726";
  } else {
    statusDot.classList.remove("connected");
    statusDot.title = "Disconnected";
    statusBar.classList.remove("connected");
    statusText.textContent = "Cosmo Downloader not running \u2014 start the app";
  }
}

/**
 * Toggle download interception
 */
toggleIntercept.addEventListener("change", () => {
  runtime.runtime.sendMessage({ action: "toggleIntercept" }, (result) => {
    if (result && result.interceptEnabled !== undefined) {
      toggleIntercept.checked = result.interceptEnabled;
    }
  });
});

/**
 * Add URL button
 */
function submitUrl() {
  const url = urlInput.value.trim();
  if (!url) {
    urlFeedback.textContent = "Please enter a URL";
    urlFeedback.className = "url-feedback error";
    return;
  }

  try {
    new URL(url);
  } catch {
    urlFeedback.textContent = "Invalid URL format";
    urlFeedback.className = "url-feedback error";
    return;
  }

  runtime.runtime.sendMessage({ action: "addUrl", url: url }, (result) => {
    if (result && result.success) {
      urlFeedback.textContent = "\u2726 Sent to CosmoDownloader!";
      urlFeedback.className = "url-feedback success";
      urlInput.value = "";
      updateStatus();
    } else {
      // Fallback direct HTTP POST to CosmoDownloader
      fetch("http://127.0.0.1:42424/download", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ url: url })
      })
      .then((r) => r.json())
      .then((res) => {
        if (res && res.success) {
          urlFeedback.textContent = "\u2726 Sent to CosmoDownloader!";
          urlFeedback.className = "url-feedback success";
          urlInput.value = "";
          updateStatus();
        } else {
          urlFeedback.textContent = "Failed \u2014 is CosmoDownloader running?";
          urlFeedback.className = "url-feedback error";
        }
      })
      .catch(() => {
        urlFeedback.textContent = "Failed \u2014 is CosmoDownloader running?";
        urlFeedback.className = "url-feedback error";
      });
    }
  });
}

addUrlBtn.addEventListener("click", submitUrl);
urlInput.addEventListener("keydown", (e) => {
  if (e.key === "Enter") submitUrl();
});

// Initial check & auto-refresh
updateStatus();

