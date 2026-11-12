/**
 * Cosmo Downloader — Browser Extension Background Worker (MV3)
 *
 * Handles:
 *  1. Native messaging host communication (com.cosmodownloader.downloader)
 *  2. Local HTTP bridge (http://127.0.0.1:42424)
 *  3. Configurable download interception with safe browser fallback
 *  4. Real cookie, referrer, and User-Agent forwarding
 *  5. Context menus & popup controls
 */

"use strict";

const DEFAULT_EXTENSIONS = [
  "zip", "rar", "7z", "tar", "gz", "bz2", "xz", "iso", "dmg", "cab",
  "exe", "msi", "msix", "appx", "apk", "deb", "rpm", "appimage", "pkg", "run",
  "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "odt", "ods",
  "mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "m4v", "mpg", "mpeg", "ts", "3gp", "m3u8", "mpd",
  "mp3", "flac", "wav", "aac", "ogg", "wma", "m4a", "opus",
  "psd", "ai", "svg", "tif", "tiff", "bmp", "raw",
  "torrent", "img", "vhd", "vmdk", "bin", "rom"
];

const INTERCEPT_MIMES = new Set([
  "application/octet-stream", "application/x-msdownload", "application/x-msdos-program",
  "application/zip", "application/x-rar-compressed", "application/x-7z-compressed",
  "application/x-tar", "application/gzip", "application/x-bzip2",
  "application/x-iso960-image", "application/pdf", "application/msword",
  "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
  "video/mp4", "video/x-matroska", "video/avi", "video/webm", "video/mp2t", "video/quicktime",
  "application/x-mpegurl", "application/vnd.apple.mpegurl", "application/dash+xml",
  "audio/mpeg", "audio/flac", "audio/wav", "audio/ogg", "application/x-bittorrent"
]);

const HOST_NAMES = ["com.cosmodownloader.downloader", "com.rustget.downloader"];
const INITIAL_RECONNECT_DELAY_MS = 1000;
const MAX_RECONNECT_DELAY_MS = 30000;

let nativePort = null;
let isConnected = false;
let interceptEnabled = true;
let interceptedCount = 0;
let lastError = "";
let filterExtensions = new Set(DEFAULT_EXTENSIONS);
let minSizeKb = 10;
let excludedSites = [];
let hostIndex = 0;
let reconnectDelayMs = INITIAL_RECONNECT_DELAY_MS;
let reconnectTimer = null;

const pendingNativeResolvers = [];
const recentUrls = new Map();

function loadSettings() {
  chrome.storage.local.get(
    ["interceptEnabled", "filterExtensionsList", "minSizeKb", "excludedSitesList"],
    (result) => {
      if (result.interceptEnabled !== undefined) {
        interceptEnabled = Boolean(result.interceptEnabled);
      }
      if (typeof result.filterExtensionsList === "string" && result.filterExtensionsList.trim()) {
        filterExtensions = parseExtensionsSet(result.filterExtensionsList);
      }
      if (typeof result.minSizeKb === "number" && Number.isFinite(result.minSizeKb) && result.minSizeKb >= 0) {
        minSizeKb = result.minSizeKb;
      }
      if (typeof result.excludedSitesList === "string") {
        excludedSites = parseDomainList(result.excludedSitesList);
      }
    }
  );
}

function parseExtensionsSet(raw) {
  const parts = String(raw || "")
    .split(/[\s,;]+/)
    .map((s) => s.trim().replace(/^\./, "").toLowerCase())
    .filter(Boolean);
  return new Set(parts);
}

function parseDomainList(raw) {
  return String(raw || "")
    .split(/[\s,;]+/)
    .map((s) => s.trim().toLowerCase().replace(/^https?:\/\//, "").split("/")[0])
    .filter(Boolean);
}

function setBadgeError(msg) {
  lastError = msg || "";
  if (chrome.action && chrome.action.setBadgeText) {
    if (lastError) {
      chrome.action.setBadgeBackgroundColor({ color: "#d93025" }, () => {});
      chrome.action.setBadgeText({ text: "!" }, () => {});
    } else {
      chrome.action.setBadgeText({ text: "" }, () => {});
    }
  }
}

async function probeAppHealth() {
  try {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 500);
    const resp = await fetch("http://127.0.0.1:42424/health", {
      method: "GET",
      signal: controller.signal
    });
    clearTimeout(timeoutId);
    if (resp.ok) {
      const data = await resp.json();
      return Boolean(data && data.status === "ok");
    }
  } catch (_) {}
  return false;
}

function scheduleReconnect() {
  if (reconnectTimer) clearTimeout(reconnectTimer);
  const delay = reconnectDelayMs;
  reconnectDelayMs = Math.min(Math.round(reconnectDelayMs * 1.5), MAX_RECONNECT_DELAY_MS);
  reconnectTimer = setTimeout(() => {
    reconnectTimer = null;
    connectNativeHost();
  }, delay);
}

function connectNativeHost() {
  if (reconnectTimer) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }
  const hostName = HOST_NAMES[hostIndex % HOST_NAMES.length];
  try {
    nativePort = chrome.runtime.connectNative(hostName);
    isConnected = true;

    nativePort.onMessage.addListener((msg) => {
      reconnectDelayMs = INITIAL_RECONNECT_DELAY_MS;
      const resolver = pendingNativeResolvers.shift();
      if (resolver) {
        resolver(msg);
      }
    });

    nativePort.onDisconnect.addListener(() => {
      const err = chrome.runtime.lastError?.message;
      isConnected = false;
      nativePort = null;
      hostIndex++;
      while (pendingNativeResolvers.length > 0) {
        const resolver = pendingNativeResolvers.shift();
        resolver({ success: false, error: err || "Native messaging host disconnected" });
      }
      scheduleReconnect();
    });

    reconnectDelayMs = INITIAL_RECONNECT_DELAY_MS;
  } catch (e) {
    isConnected = false;
    nativePort = null;
    hostIndex++;
    scheduleReconnect();
  }
}

function sendViaNativeHost(payload, timeoutMs = 2500) {
  return new Promise((resolve) => {
    if (!nativePort || !isConnected) {
      connectNativeHost();
    }
    if (!nativePort || !isConnected) {
      resolve({ success: false, error: "Native messaging host is not registered or unavailable" });
      return;
    }

    let settled = false;
    const timer = setTimeout(() => {
      if (!settled) {
        settled = true;
        const idx = pendingNativeResolvers.indexOf(onReply);
        if (idx >= 0) pendingNativeResolvers.splice(idx, 1);
        resolve({ success: false, error: "Timed out waiting for native messaging host" });
      }
    }, timeoutMs);

    const onReply = (msg) => {
      if (!settled) {
        settled = true;
        clearTimeout(timer);
        resolve(msg || { success: false, error: "Empty response from native host" });
      }
    };

    pendingNativeResolvers.push(onReply);
    try {
      nativePort.postMessage(payload);
    } catch (e) {
      if (!settled) {
        settled = true;
        clearTimeout(timer);
        const idx = pendingNativeResolvers.indexOf(onReply);
        if (idx >= 0) pendingNativeResolvers.splice(idx, 1);
        isConnected = false;
        resolve({ success: false, error: "Failed to post message to native host" });
      }
    }
  });
}

function getCookiesForUrl(url) {
  return new Promise((resolve) => {
    if (!chrome.cookies || !chrome.cookies.getAll) {
      resolve("");
      return;
    }
    try {
      chrome.cookies.getAll({ url }, (cookies) => {
        if (chrome.runtime.lastError || !Array.isArray(cookies) || cookies.length === 0) {
          resolve("");
          return;
        }
        resolve(cookies.map((c) => `${c.name}=${c.value}`).join("; "));
      });
    } catch (_) {
      resolve("");
    }
  });
}

async function deliverToCosmo(data) {
  const url = (data.url || "").trim();
  if (!/^https?:\/\//i.test(url)) {
    return { success: false, error: "Only http:// and https:// URLs can be sent to Cosmo Downloader" };
  }

  let cookieHeader = data.cookies || "";
  if (!cookieHeader) {
    cookieHeader = await getCookiesForUrl(url);
  }
  const userAgent = data.user_agent || (typeof navigator !== "undefined" ? navigator.userAgent : "");
  const referer = data.referrer || data.referer || "";

  const bridgePayload = {
    url,
    filename: data.filename || null,
    cookies: cookieHeader || null,
    referer: referer || null,
    user_agent: userAgent || null,
  };

  // 1. Try running Cosmo Downloader desktop bridge first
  try {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 1500);
    const resp = await fetch("http://127.0.0.1:42424/download", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(bridgePayload),
      signal: controller.signal,
    });
    clearTimeout(timeoutId);

    const res = await resp.json().catch(() => null);
    if (resp.ok && res && res.success) {
      interceptedCount++;
      setBadgeError("");
      return { success: true };
    }
    if (res && res.error) {
      setBadgeError(res.error);
      return { success: false, error: res.error };
    }
  } catch (_) {
    // Desktop app is likely closed or unreachable; fall back to native messaging host.
  }

  // 2. Fallback via Native Messaging Host (launches popup or writes to DB queue)
  const nativeRes = await sendViaNativeHost({
    action: "download",
    url,
    filename: data.filename || null,
    cookies: cookieHeader || null,
    referrer: referer || null,
    user_agent: userAgent || null,
  });

  if (nativeRes && nativeRes.success) {
    interceptedCount++;
    setBadgeError("");
    return { success: true };
  }

  const errMsg = (nativeRes && nativeRes.error) || "Could not connect to Cosmo Downloader";
  setBadgeError(errMsg);
  return { success: false, error: errMsg };
}

function getExtension(urlOrFilename) {
  try {
    const pathname = new URL(urlOrFilename).pathname;
    const dot = pathname.lastIndexOf(".");
    if (dot !== -1) {
      return pathname.substring(dot + 1).toLowerCase().split(/[?#]/)[0];
    }
  } catch {
    const clean = String(urlOrFilename || "").split(/[?#]/)[0];
    const dot = clean.lastIndexOf(".");
    if (dot !== -1) {
      return clean.substring(dot + 1).toLowerCase();
    }
  }
  return "";
}

function isUrlExcluded(urlStr) {
  if (!urlStr || excludedSites.length === 0) return false;
  try {
    const host = new URL(urlStr).hostname.toLowerCase();
    return excludedSites.some((domain) => host === domain || host.endsWith("." + domain));
  } catch {
    return false;
  }
}

function shouldIntercept(downloadItem) {
  if (!interceptEnabled) return false;

  const url = downloadItem.url || "";
  if (!/^https?:\/\//i.test(url)) {
    return false;
  }
  if (isUrlExcluded(url) || isUrlExcluded(downloadItem.referrer || "")) {
    return false;
  }

  const size = (downloadItem.fileSize > 0 ? downloadItem.fileSize : downloadItem.totalBytes) || -1;
  const minBytes = minSizeKb * 1024;
  if (size > 0 && minBytes > 0 && size < minBytes) {
    return false;
  }

  const ext = getExtension(downloadItem.filename || url);
  if (ext && filterExtensions.has(ext)) return true;

  const mime = (downloadItem.mime || "").toLowerCase().split(";")[0].trim();
  if (mime && INTERCEPT_MIMES.has(mime)) return true;

  if (minBytes > 0 && size >= minBytes) return true;

  return false;
}

function isDuplicateRecentUrl(url) {
  const now = Date.now();
  const prev = recentUrls.get(url);
  recentUrls.set(url, now);
  if (recentUrls.size > 64) {
    for (const [k, ts] of recentUrls.entries()) {
      if (now - ts > 5000) recentUrls.delete(k);
    }
  }
  return prev !== undefined && (now - prev) < 400;
}

chrome.downloads.onCreated.addListener((downloadItem) => {
  if (!interceptEnabled) return;
  if (downloadItem.byExtensionId && downloadItem.byExtensionId === chrome.runtime.id) return;

  const url = downloadItem.url || "";
  if (!shouldIntercept(downloadItem)) return;
  if (isDuplicateRecentUrl(url)) return;

  // Pause browser download while handing off so browser fallback stays intact on failure.
  chrome.downloads.pause(downloadItem.id, () => {
    const _pauseErr = chrome.runtime.lastError;
    const filename = downloadItem.filename
      ? downloadItem.filename.split(/[\\/]/).pop()
      : url.split("/").pop()?.split("?")[0] || "download";

    deliverToCosmo({
      action: "download",
      url,
      filename,
      fileSize: downloadItem.fileSize || downloadItem.totalBytes || -1,
      mime: downloadItem.mime || "",
      referrer: downloadItem.referrer || "",
    }).then((res) => {
      if (res.success) {
        chrome.downloads.cancel(downloadItem.id, () => {
          const _cancelErr = chrome.runtime.lastError;
          chrome.downloads.erase({ id: downloadItem.id }, () => {
            const _eraseErr = chrome.runtime.lastError;
          });
        });
      } else {
        // Fallback: let the browser download continue
        chrome.downloads.resume(downloadItem.id, () => {
          const _resumeErr = chrome.runtime.lastError;
        });
      }
    });
  });
});

// --- Context Menus ---
chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.removeAll(() => {
    chrome.contextMenus.create({
      id: "cosmo-download-link",
      title: "Download with Cosmo Downloader",
      contexts: ["link"],
    });

    chrome.contextMenus.create({
      id: "cosmo-download-media",
      title: "Download media with Cosmo Downloader",
      contexts: ["image", "video", "audio"],
    });
  });
});

chrome.contextMenus.onClicked.addListener((info, tab) => {
  let url = "";
  if (info.menuItemId === "cosmo-download-link") {
    url = info.linkUrl || "";
  } else if (info.menuItemId === "cosmo-download-media") {
    url = info.srcUrl || "";
  }

  if (url) {
    const filename = url.split("/").pop()?.split("?")[0] || "download";
    deliverToCosmo({
      action: "download",
      url,
      filename,
      referrer: tab?.url || "",
    });
  }
});

// --- Messages ---
chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  switch (message.action) {
    case "getStatus":
      (async () => {
        const appRunning = await probeAppHealth();
        if (!nativePort) {
          connectNativeHost();
        }
        const hostInstalled = isConnected && nativePort !== null;
        sendResponse({
          appRunning,
          hostInstalled,
          connected: appRunning,
          interceptEnabled,
          interceptedCount,
          lastError,
        });
      })();
      return true;

    case "launchApp":
      (async () => {
        const launchRes = await sendViaNativeHost({ action: "launch" }, 4000);
        if (!launchRes || !launchRes.success) {
          sendResponse({
            success: false,
            error: (launchRes && launchRes.error) || "Native messaging host not reachable",
          });
          return;
        }

        const startTime = Date.now();
        let launched = false;
        let delay = 150;
        while (Date.now() - startTime < 5000) {
          await new Promise((r) => setTimeout(r, delay));
          delay = Math.min(delay * 1.4, 500);
          if (await probeAppHealth()) {
            launched = true;
            break;
          }
        }

        sendResponse({
          success: launched,
          appRunning: launched,
          error: launched ? null : "Timed out waiting for Cosmo Downloader to start",
        });
      })();
      return true;

    case "toggleIntercept":
      interceptEnabled = !interceptEnabled;
      chrome.storage.local.set({ interceptEnabled });
      sendResponse({ interceptEnabled });
      break;

    case "getConfig":
      sendResponse({
        filterExtensions: Array.from(filterExtensions).join(", "),
        minSizeKb,
        excludedSites: excludedSites.join(", "),
      });
      break;

    case "saveConfig": {
      const extSet = parseExtensionsSet(message.filterExtensions || "");
      filterExtensions = extSet.size > 0 ? extSet : new Set(DEFAULT_EXTENSIONS);
      const parsedMin = Number(message.minSizeKb);
      minSizeKb = Number.isFinite(parsedMin) && parsedMin >= 0 ? Math.floor(parsedMin) : 0;
      excludedSites = parseDomainList(message.excludedSites || "");
      chrome.storage.local.set({
        filterExtensionsList: Array.from(filterExtensions).join(", "),
        minSizeKb,
        excludedSitesList: excludedSites.join(", "),
      });
      sendResponse({
        success: true,
        filterExtensions: Array.from(filterExtensions).join(", "),
        minSizeKb,
        excludedSites: excludedSites.join(", "),
      });
      break;
    }

    case "clearError":
      setBadgeError("");
      sendResponse({ success: true });
      break;

    case "addUrl":
      if (message.url) {
        const filename = message.url.split("/").pop()?.split("?")[0] || "download";
        deliverToCosmo({
          action: "download",
          url: message.url,
          filename,
          referrer: "",
        }).then((res) => sendResponse(res));
        return true;
      }
      sendResponse({ success: false, error: "Empty URL" });
      break;

    case "downloadVideo":
      if (message.url) {
        deliverToCosmo({
          action: "download",
          url: message.url,
          filename: message.filename || "",
          referrer: message.pageUrl || "",
        }).then((res) => sendResponse(res));
        return true;
      }
      sendResponse({ success: false, error: "Empty video URL" });
      break;

    default:
      break;
  }
  return true;
});

loadSettings();
connectNativeHost();
