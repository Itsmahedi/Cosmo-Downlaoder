/**
 * Cosmo Downloader v1.12 — Popup Script
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
 * Theme support: read stored preference and apply
 */
function applyTheme(theme) {
  if (theme === "dark" || theme === "light") {
    document.documentElement.dataset.theme = theme;
  } else {
    delete document.documentElement.dataset.theme;
  }
}

// Load saved theme
try {
  runtime.storage.local.get("theme", (result) => {
    if (runtime.runtime.lastError) return;
    applyTheme(result.theme);
  });

  // Listen for theme changes
  runtime.storage.onChanged.addListener((changes, area) => {
    if (area === "local" && changes.theme) {
      applyTheme(changes.theme.newValue);
    }
  });
} catch (_) {}

/**
 * Updates UI based on accurate 3-state detection:
 * 1. Connected (Green): App is actively running and responding on 127.0.0.1:42424
 * 2. Closed (Amber): App is stopped, but native messaging host is ready to launch it
 * 3. Disconnected (Red): Native host is not installed or registered
 */
function updateStatus() {
  runtime.runtime.sendMessage({ action: "getStatus" }, (response) => {
    const lastErr = runtime.runtime.lastError;
    if (lastErr || !response) {
      applyState("disconnected", "Disconnected");
      return;
    }

    if (response.interceptEnabled !== undefined) {
      toggleIntercept.checked = response.interceptEnabled;
    }
    interceptedCountEl.textContent = response.interceptedCount || 0;

    if (response.appRunning) {
      applyState("connected", "Connected to Cosmo Downloader");
    } else if (response.hostInstalled) {
      applyState("amber", "Cosmo Downloader is closed");
    } else {
      applyState("disconnected", "Disconnected");
    }
  });
}

function applyState(state, text) {
  // Update dot
  statusDot.className = "dot " + state;

  // Update connection banner
  if (state === "connected") {
    statusBar.className = "conn";
  } else {
    statusBar.className = "conn " + state;
  }

  statusText.textContent = text;

  if (state === "connected") {
    statusDot.title = "Connected to Cosmo Downloader";
  } else if (state === "amber") {
    statusDot.title = "Cosmo Downloader is closed";
  } else {
    statusDot.title = "Disconnected";
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
 * Add URL with strict HTTP/HTTPS validation
 */
function submitUrl() {
  const urlStr = urlInput.value.trim();
  if (!urlStr) {
    urlFeedback.textContent = "Please enter a URL";
    urlFeedback.className = "url-feedback error";
    return;
  }

  let parsedUrl;
  try {
    parsedUrl = new URL(urlStr);
    if (parsedUrl.protocol !== "http:" && parsedUrl.protocol !== "https:") {
      urlFeedback.textContent = "Only http:// and https:// URLs are supported";
      urlFeedback.className = "url-feedback error";
      return;
    }
  } catch {
    urlFeedback.textContent = "Invalid URL format";
    urlFeedback.className = "url-feedback error";
    return;
  }

  runtime.runtime.sendMessage({ action: "addUrl", url: urlStr }, (result) => {
    if (result && result.success) {
      urlFeedback.textContent = "Sent to Cosmo Downloader";
      urlFeedback.className = "url-feedback success";
      urlInput.value = "";
      updateStatus();
    } else {
      urlFeedback.textContent = "Failed — is Cosmo Downloader running?";
      urlFeedback.className = "url-feedback error";
    }
  });
}

addUrlBtn.addEventListener("click", submitUrl);
urlInput.addEventListener("keydown", (e) => {
  if (e.key === "Enter") {
    submitUrl();
  }
});

// Initial status check & periodic 2-second polling while popup is open
updateStatus();
const statusInterval = setInterval(updateStatus, 2000);
window.addEventListener("unload", () => {
  clearInterval(statusInterval);
});
