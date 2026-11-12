/**
 * Cosmo Downloader — Popup Script
 */

"use strict";

const statusDot = document.getElementById("statusDot");
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
const extFilterInput = document.getElementById("extFilterInput");
const minSizeInput = document.getElementById("minSizeInput");
const excludedSitesInput = document.getElementById("excludedSitesInput");
const saveFiltersBtn = document.getElementById("saveFiltersBtn");
const filterSavedMsg = document.getElementById("filterSavedMsg");

const runtime = typeof browser !== "undefined" ? browser : chrome;

let isLaunching = false;

/**
 * Updates UI based on 3-state detection:
 * 1. Connected: App is running and responding on 127.0.0.1:42424
 * 2. Closed (Amber): App is stopped, but native messaging host is ready to launch it
 * 3. Disconnected: Native host is not installed or registered
 */
function updateStatus() {
  if (isLaunching) return;

  runtime.runtime.sendMessage({ action: "getStatus" }, (response) => {
    const lastErr = runtime.runtime.lastError;
    if (lastErr || !response) {
      applyState("disconnected", "Host bridge not detected");
      return;
    }

    if (response.interceptEnabled !== undefined) {
      toggleIntercept.checked = response.interceptEnabled;
    }
    if (interceptedCountEl) {
      interceptedCountEl.textContent = response.interceptedCount || 0;
    }

    if (response.appRunning) {
      applyState("connected", "Connected");
    } else if (response.hostInstalled) {
      applyState("amber", "App closed");
    } else {
      applyState("disconnected", "Host bridge not detected");
    }

    if (statusError && response.lastError) {
      statusError.textContent = response.lastError;
      statusError.style.display = "block";
    }
  });
}

function applyState(state, text) {
  statusDot.className = "dot " + state;
  if (statusText) statusText.textContent = text;

  if (state === "connected") {
    statusDot.title = "Connected to Cosmo Downloader";
    if (launchBtn) launchBtn.style.display = "none";
    if (statusError) statusError.style.display = "none";
  } else if (state === "amber") {
    statusDot.title = "Cosmo Downloader is closed — click Launch Cosmo to start";
    if (launchBtn) {
      launchBtn.style.display = "inline-flex";
      launchBtn.disabled = false;
      if (launchSpinner) launchSpinner.style.display = "none";
      if (launchBtnText) launchBtnText.textContent = "Launch Cosmo";
    }
  } else {
    statusDot.title = "Host bridge not registered";
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
        applyState("connected", "Connected");
      } else {
        const errMsg = res?.error || "Launch timed out. Start Cosmo Downloader manually.";
        if (statusError) {
          statusError.textContent = errMsg;
          statusError.style.display = "block";
        }
        applyState("amber", "Failed to start");
        if (launchBtn) {
          launchBtn.disabled = false;
          if (launchSpinner) launchSpinner.style.display = "none";
          if (launchBtnText) launchBtnText.textContent = "Retry Launch";
        }
      }
    });
  });
}

/**
 * Toggle download interception
 */
if (toggleIntercept) {
  toggleIntercept.addEventListener("change", () => {
    runtime.runtime.sendMessage({ action: "toggleIntercept" }, (result) => {
      if (result && result.interceptEnabled !== undefined) {
        toggleIntercept.checked = result.interceptEnabled;
      }
    });
  });
}

/**
 * Add URL with strict HTTP/HTTPS validation
 */
function submitUrl() {
  const urlStr = urlInput.value.trim();
  if (!urlStr) {
    if (urlFeedback) {
      urlFeedback.textContent = "Please enter a URL";
      urlFeedback.className = "url-feedback error";
      urlFeedback.style.display = "block";
    }
    return;
  }

  try {
    const parsedUrl = new URL(urlStr);
    if (parsedUrl.protocol !== "http:" && parsedUrl.protocol !== "https:") {
      if (urlFeedback) {
        urlFeedback.textContent = "Only http:// and https:// URLs are supported";
        urlFeedback.className = "url-feedback error";
        urlFeedback.style.display = "block";
      }
      return;
    }
  } catch {
    if (urlFeedback) {
      urlFeedback.textContent = "Invalid URL format";
      urlFeedback.className = "url-feedback error";
      urlFeedback.style.display = "block";
    }
    return;
  }

  runtime.runtime.sendMessage({ action: "addUrl", url: urlStr }, (result) => {
    if (result && result.success) {
      if (urlFeedback) {
        urlFeedback.textContent = "Sent to Cosmo Downloader";
        urlFeedback.className = "url-feedback";
        urlFeedback.style.display = "block";
      }
      urlInput.value = "";
      setTimeout(() => {
        if (urlFeedback) urlFeedback.style.display = "none";
      }, 3000);
    } else {
      if (urlFeedback) {
        urlFeedback.textContent = result?.error || "Failed to send link";
        urlFeedback.className = "url-feedback error";
        urlFeedback.style.display = "block";
      }
    }
  });
}

if (addUrlBtn) {
  addUrlBtn.addEventListener("click", submitUrl);
}

if (urlInput) {
  urlInput.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
      submitUrl();
    }
  });
}

function loadFilterConfig() {
  runtime.runtime.sendMessage({ action: "getConfig" }, (cfg) => {
    if (runtime.runtime.lastError || !cfg) return;
    if (extFilterInput && typeof cfg.filterExtensions === "string") {
      extFilterInput.value = cfg.filterExtensions;
    }
    if (minSizeInput && typeof cfg.minSizeKb === "number") {
      minSizeInput.value = String(cfg.minSizeKb);
    }
    if (excludedSitesInput && typeof cfg.excludedSites === "string") {
      excludedSitesInput.value = cfg.excludedSites;
    }
  });
}

if (saveFiltersBtn) {
  saveFiltersBtn.addEventListener("click", () => {
    runtime.runtime.sendMessage(
      {
        action: "saveConfig",
        filterExtensions: extFilterInput ? extFilterInput.value : "",
        minSizeKb: minSizeInput ? Number(minSizeInput.value) : 0,
        excludedSites: excludedSitesInput ? excludedSitesInput.value : "",
      },
      (res) => {
        if (res && res.success) {
          if (extFilterInput) extFilterInput.value = res.filterExtensions;
          if (minSizeInput) minSizeInput.value = String(res.minSizeKb);
          if (excludedSitesInput) excludedSitesInput.value = res.excludedSites;
          if (filterSavedMsg) {
            filterSavedMsg.textContent = "Saved";
            setTimeout(() => { filterSavedMsg.textContent = ""; }, 2000);
          }
        }
      }
    );
  });
}

// Initial status & config loading
loadFilterConfig();
updateStatus();
const interval = setInterval(updateStatus, 3000);

window.addEventListener("unload", () => {
  clearInterval(interval);
});
