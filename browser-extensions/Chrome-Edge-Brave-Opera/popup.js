/**
 * ✦ Cosmo Downloader v1.11 — Popup Script
 */

"use strict";

const statusDot = document.getElementById("statusDot");
const statusBar = document.getElementById("statusBar");
const statusText = document.getElementById("statusText");
const launchBtn = document.getElementById("launchBtn");
const launchSpinner = document.getElementById("launchSpinner");
const launchBtnText = document.getElementById("launchBtnText");
const statusError = document.getElementById("statusError");
const toggleIntercept = document.getElementById("toggleIntercept");
const interceptedCountEl = document.getElementById("interceptedCount");
const urlInput = document.getElementById("urlInput");
const addUrlBtn = document.getElementById("addUrlBtn");
const urlFeedback = document.getElementById("urlFeedback");

const runtime = typeof browser !== "undefined" ? browser : chrome;

let isLaunching = false;

/**
 * Updates UI based on accurate 3-state detection:
 * 1. Connected (Green): App is actively running and responding on 127.0.0.1:42424
 * 2. Closed (Amber): App is stopped, but native messaging host is ready to launch it
 * 3. Disconnected (Red): Native host is not installed or registered
 */
function updateStatus() {
  if (isLaunching) return;

  runtime.runtime.sendMessage({ action: "getStatus" }, (response) => {
    const lastErr = runtime.runtime.lastError;
    if (lastErr || !response) {
      applyState("disconnected", "Native host not registered");
      return;
    }

    if (response.interceptEnabled !== undefined) {
      toggleIntercept.checked = response.interceptEnabled;
    }
    interceptedCountEl.textContent = response.interceptedCount || 0;

    if (response.appRunning) {
      applyState("connected", "Connected to Cosmo Downloader \u2726");
    } else if (response.hostInstalled) {
      applyState("amber", "Cosmo Downloader is closed");
    } else {
      applyState("disconnected", "Native host not registered");
    }
  });
}

function applyState(state, text) {
  statusDot.className = "status-dot " + state;
  statusBar.className = "status-bar " + state;
  statusText.textContent = text;

  if (state === "connected") {
    statusDot.title = "Connected to Cosmo Downloader Engine";
    if (launchBtn) launchBtn.style.display = "none";
    if (statusError) statusError.style.display = "none";
  } else if (state === "amber") {
    statusDot.title = "Cosmo Downloader is closed — click Launch App to start";
    if (launchBtn) {
      launchBtn.style.display = "inline-flex";
      launchBtn.disabled = false;
      if (launchSpinner) launchSpinner.style.display = "none";
      if (launchBtnText) launchBtnText.textContent = "\u25B6 Launch App";
    }
  } else {
    statusDot.title = "Host bridge not installed";
    if (launchBtn) launchBtn.style.display = "none";
  }
}

/**
 * Launch App Button Handler with spinner & timeout handling
 */
if (launchBtn) {
  launchBtn.addEventListener("click", () => {
    if (isLaunching) return;
    isLaunching = true;

    launchBtn.disabled = true;
    if (launchSpinner) launchSpinner.style.display = "inline-block";
    if (launchBtnText) launchBtnText.textContent = "Starting...";
    if (statusError) statusError.style.display = "none";

    runtime.runtime.sendMessage({ action: "launchApp" }, (res) => {
      isLaunching = false;
      if (res && res.success) {
        applyState("connected", "Connected to Cosmo Downloader \u2726");
      } else {
        const errMsg = res?.error || "Launch timed out. Start Cosmo Downloader manually.";
        if (statusError) {
          statusError.textContent = errMsg;
          statusError.style.display = "block";
        }
        applyState("amber", "Failed to start app");
        if (launchBtn) {
          launchBtn.disabled = false;
          if (launchSpinner) launchSpinner.style.display = "none";
          if (launchBtnText) launchBtnText.textContent = "\u25B6 Retry Launch";
        }
      }
    });
  });
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
      urlFeedback.textContent = "\u2726 Sent to CosmoDownloader!";
      urlFeedback.className = "url-feedback success";
      urlInput.value = "";
      updateStatus();
    } else {
      urlFeedback.textContent = "Failed — is CosmoDownloader running?";
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
