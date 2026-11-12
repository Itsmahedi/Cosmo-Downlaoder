/**
 * Cosmo Downloader v1.1.0 — Service Worker (Manifest V3)
 * 
 * Handles:
 *  1. Native messaging connection (com.cosmodownloader.downloader / com.rustget.downloader)
 *  2. Local HTTP bridge fallback (http://127.0.0.1:42424)
 *  3. Download interception (all file types)
 *  4. Context menu ("Download with CosmoDownloader")
 *  5. Video stream interception
 *  6. Popup communication
 */

"use strict";

let nativePort = null;
let isConnected = false;
let interceptEnabled = true;
let interceptedCount = 0;

const HOST_NAMES = ["com.cosmodownloader.downloader", "com.rustget.downloader"];
let hostIndex = 0;

/**
 * Probes the local CosmoDownloader loopback health endpoint on-demand.
 * Timeout is constrained to 500ms to avoid freezing background service worker.
 */
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
      return data && data.status === "ok";
    }
  } catch (_) {}
  return false;
}

const INTERCEPT_EXTENSIONS = new Set([
  "zip", "rar", "7z", "tar", "gz", "bz2", "xz", "iso", "dmg", "cab",
  "exe", "msi", "apk", "deb", "rpm", "appimage", "pkg", "run",
  "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "odt", "ods",
  "mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "m4v", "mpg", "mpeg", "ts", "3gp", "m3u8", "mpd",
  "mp3", "flac", "wav", "aac", "ogg", "wma", "m4a", "opus",
  "psd", "ai", "svg", "tif", "tiff", "bmp", "raw",
  "torrent", "magnet", "img", "vhd", "vmdk", "bin", "dat", "rom"
]);

const INTERCEPT_MIMES = new Set([
  "application/octet-stream", "application/x-msdownload", "application/x-msdos-program",
  "application/zip", "application/x-rar-compressed", "application/x-7z-compressed",
  "application/x-tar", "application/gzip", "application/x-bzip2",
  "application/x-iso9660-image", "application/pdf", "application/msword",
  "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
  "video/mp4", "video/x-matroska", "video/avi", "video/webm", "video/mp2t", "video/quicktime",
  "application/x-mpegurl", "application/vnd.apple.mpegurl", "application/dash+xml",
  "audio/mpeg", "audio/flac", "audio/wav", "audio/ogg", "application/x-bittorrent"
]);

const MIN_SIZE_TO_INTERCEPT = 10240;

let reconnectDelayMs = 1000;
const INITIAL_RECONNECT_DELAY_MS = 1000;
const MAX_RECONNECT_DELAY_MS = 30000;
let reconnectTimer = null;

function scheduleReconnect() {
  if (reconnectTimer) clearTimeout(reconnectTimer);
  const delay = reconnectDelayMs;
  reconnectDelayMs = Math.min(Math.round(reconnectDelayMs * 1.5), MAX_RECONNECT_DELAY_MS);
  console.log(`[CosmoDownloader] Native host disconnected. Scheduling reconnect in ${delay}ms...`);
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
      console.log("[CosmoDownloader] Host message:", msg);
      // Reset backoff on successful active communication
      reconnectDelayMs = INITIAL_RECONNECT_DELAY_MS;
    });

    nativePort.onDisconnect.addListener(() => {
      const err = chrome.runtime.lastError?.message;
      if (err) console.warn("[CosmoDownloader] Native host disconnected with error:", err);
      isConnected = false;
      nativePort = null;
      hostIndex++;
      scheduleReconnect();
    });

    console.log(`[CosmoDownloader] Connected to native host: ${hostName}`);
    reconnectDelayMs = INITIAL_RECONNECT_DELAY_MS;
  } catch (e) {
    isConnected = false;
    nativePort = null;
    hostIndex++;
    scheduleReconnect();
  }
}

function sendToCosmo(data) {
  const bridgePayload = {
    url: data.url,
    filename: data.filename || null,
    cookies: data.cookies || null,
    referer: data.referrer || data.referer || null
  };

  // Always attempt local HTTP bridge first for instantaneous IDM popup response
  if (data.url) {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 3000);

    fetch("http://127.0.0.1:42424/download", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(bridgePayload),
      signal: controller.signal
    })
    .then((r) => r.json())
    .then((res) => {
      clearTimeout(timeoutId);
      if (res && res.success) {
        interceptedCount++;
      }
    })
    .catch(() => {
      clearTimeout(timeoutId);
      // If local bridge is offline or timed out, send via native messaging host to launch CosmoDownloader
      if (!nativePort || !isConnected) {
        connectNativeHost();
      }
      if (nativePort && isConnected) {
        try {
          nativePort.postMessage(data);
          interceptedCount++;
        } catch (e) {
          isConnected = false;
        }
      }
    });

    return true;
  }

  if (nativePort && isConnected) {
    try {
      nativePort.postMessage(data);
      interceptedCount++;
      return true;
    } catch (e) {
      isConnected = false;
    }
  }

  return false;
}

function getExtension(urlOrFilename) {
  try {
    const pathname = new URL(urlOrFilename).pathname;
    const dot = pathname.lastIndexOf(".");
    if (dot !== -1) {
      return pathname.substring(dot + 1).toLowerCase().split(/[?#]/)[0];
    }
  } catch {
    const dot = urlOrFilename.lastIndexOf(".");
    if (dot !== -1) {
      return urlOrFilename.substring(dot + 1).toLowerCase();
    }
  }
  return "";
}

function shouldIntercept(downloadItem) {
  if (!interceptEnabled) return false;

  const url = downloadItem.url || "";
  if (url.startsWith("blob:") || url.startsWith("data:") || url.startsWith("chrome-extension:")) {
    return false;
  }

  const ext = getExtension(downloadItem.filename || url);
  if (ext && INTERCEPT_EXTENSIONS.has(ext)) return true;

  const mime = (downloadItem.mime || "").toLowerCase();
  if (mime && INTERCEPT_MIMES.has(mime)) return true;

  if (downloadItem.fileSize && downloadItem.fileSize > MIN_SIZE_TO_INTERCEPT) return true;
  if (downloadItem.totalBytes && downloadItem.totalBytes > MIN_SIZE_TO_INTERCEPT) return true;

  return true;
}

chrome.downloads.onCreated.addListener((downloadItem) => {
  if (!interceptEnabled) return;

  const url = downloadItem.url || "";
  if (url.startsWith("blob:") || url.startsWith("data:") || url.startsWith("chrome-extension:")) {
    return;
  }

  if (shouldIntercept(downloadItem)) {
    chrome.downloads.cancel(downloadItem.id, () => {
      chrome.downloads.erase({ id: downloadItem.id }, () => {});
    });

    const filename = downloadItem.filename
      ? downloadItem.filename.split(/[\\/]/).pop()
      : url.split("/").pop()?.split("?")[0] || "download";

    sendToCosmo({
      action: "download",
      url: url,
      filename: filename,
      fileSize: downloadItem.fileSize || downloadItem.totalBytes || -1,
      mime: downloadItem.mime || "",
      referrer: downloadItem.referrer || "",
    });

    console.log(`[CosmoDownloader] Intercepted: ${filename}`);
  }
});

// ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ Context Menus ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬

chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create({
    id: "cosmo-download-link",
    title: "Download with CosmoDownloader",
    contexts: ["link"],
  });

  chrome.contextMenus.create({
    id: "cosmo-download-media",
    title: "Download media with CosmoDownloader",
    contexts: ["image", "video", "audio"],
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
    sendToCosmo({
      action: "download",
      url: url,
      filename: filename,
      referrer: tab?.url || "",
    });
  }
});

// ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ Messages ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬ÃƒÂ¢Ã¢â‚¬ÂÃ¢â€šÂ¬

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
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
        });
      })();
      return true;

    case "launchApp":
      (async () => {
        if (!nativePort || !isConnected) {
          connectNativeHost();
        }
        if (!nativePort || !isConnected) {
          sendResponse({ success: false, error: "Native messaging host not reachable" });
          return;
        }

        try {
          nativePort.postMessage({ action: "launch" });
        } catch (err) {
          sendResponse({ success: false, error: "Failed to send launch command: " + err });
          return;
        }

        // Poll /health with backoff for up to ~5s
        const startTime = Date.now();
        let launched = false;
        let delay = 150;

        while (Date.now() - startTime < 5000) {
          await new Promise((r) => setTimeout(r, delay));
          delay = Math.min(delay * 1.4, 500);
          const healthy = await probeAppHealth();
          if (healthy) {
            launched = true;
            break;
          }
        }

        sendResponse({
          success: launched,
          appRunning: launched,
          error: launched ? null : "Timed out waiting for CosmoDownloader to start"
        });
      })();
      return true;

    case "toggleIntercept":
      interceptEnabled = !interceptEnabled;
      chrome.storage.local.set({ interceptEnabled });
      sendResponse({ interceptEnabled });
      break;

    case "addUrl":
      if (message.url) {
        const filename = message.url.split("/").pop()?.split("?")[0] || "download";
        const sent = sendToCosmo({
          action: "download",
          url: message.url,
          filename: filename,
          referrer: "",
        });
        sendResponse({ success: sent });
      }
      break;

    case "downloadVideo":
      if (message.url) {
        const sent = sendToCosmo({
          action: "download",
          url: message.url,
          filename: message.filename || "",
          referrer: message.pageUrl || "",
        });
        sendResponse({ success: sent });
      }
      break;

    default:
      break;
  }
  return true;
});

chrome.storage.local.get("interceptEnabled", (result) => {
  if (result.interceptEnabled !== undefined) {
    interceptEnabled = result.interceptEnabled;
  }
});

connectNativeHost();
console.log("[Cosmo Downloader] Service worker active - v1.1.0");

