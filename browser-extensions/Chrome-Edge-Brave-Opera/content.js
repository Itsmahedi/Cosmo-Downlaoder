/**
 * Cosmo Downloader v1.1.0 — Content Script (Chrome, Edge, Brave, Opera, Firefox)
 * 
 * Injects a compact, floating cosmic download button on qualified videos
 * with an interactive IDM-style quality/format selector menu.
 * 
 * Strictly suppresses buttons on hover mini-previews, thumbnails, and inactive players.
 */

"use strict";

const runtime = typeof browser !== "undefined" ? browser : (typeof chrome !== "undefined" ? chrome : null);

const QUALITIES = [
  { label: "1080p", meta: "148 MB", format: "1080p", audioOnly: false },
  { label: "720p", meta: "82 MB", format: "720p", audioOnly: false },
  { label: "480p", meta: "41 MB", format: "480p", audioOnly: false },
  { label: "360p", meta: "24 MB", format: "360p", audioOnly: false },
  { divider: true },
  { label: "Audio only", meta: "MP3, 9 MB", format: "audio", audioOnly: true },
  { label: "Best available", meta: "Auto", format: "best", audioOnly: false }
];

// Track injected UI elements per video element: Map<HTMLVideoElement, { container: HTMLElement, host: HTMLElement } >
const activeAttachedButtons = new Map();

/**
 * Checks whether the current page is YouTube (exact domain match)
 */
function isYouTubeHost(win = window) {
  const hostname = win.location.hostname.toLowerCase();
  return hostname === "youtube.com" || hostname.endsWith(".youtube.com");
}

/**
 * Determines whether a given video element is an unwanted mini-preview or thumbnail.
 * Exported / exposed for unit testing when in Node/CommonJS environment.
 */
function isMiniPreview(videoEl, win = window) {
  if (!videoEl) return true;

  if (isYouTubeHost(win)) {
    // 1. Definite YouTube preview / thumbnail containers
    if (typeof videoEl.closest === "function" && videoEl.closest(
      "ytd-video-preview, " +
      "ytd-inline-preview-renderer, " +
      "#inline-preview-player, " +
      "ytd-thumbnail, " +
      "ytd-compact-video-renderer, " +
      ".ytd-thumbnail"
    )) {
      return true;
    }

    const pathname = win.location.pathname;
    const isWatch = pathname === "/watch" || pathname.startsWith("/watch");
    const isShorts = pathname === "/shorts" || pathname.startsWith("/shorts");

    if (isWatch) {
      // On /watch: Only attach to the main video player inside #movie_player
      if (typeof videoEl.closest !== "function") return true;
      const moviePlayer = videoEl.closest("#movie_player");
      if (!moviePlayer || moviePlayer.id === "inline-preview-player") {
        return true;
      }
      // If html5-main-video exists in player, videoEl must be it
      if (videoEl.classList && !videoEl.classList.contains("html5-main-video")) {
        const mainVid = moviePlayer.querySelector ? moviePlayer.querySelector("video.html5-main-video") : null;
        if (mainVid && mainVid !== videoEl) {
          return true;
        }
      }
      return false;
    }

    if (isShorts) {
      // On /shorts: Only attach to the active reel video
      if (typeof videoEl.closest !== "function") return true;
      const activeReel = videoEl.closest("ytd-reel-video-renderer[is-active], #shorts-player");
      if (!activeReel) {
        return true;
      }
      return false;
    }

    // On any other YouTube page (home feed, subscriptions, search, channel page):
    // All videos here are hover previews or thumbnails - ignore them!
    return true;
  }

  // Non-YouTube hosts:
  // Lenient fallback size check: ignore if strictly less than 320x180
  try {
    if (typeof videoEl.getBoundingClientRect === "function") {
      const rect = videoEl.getBoundingClientRect();
      if (rect.width > 0 && rect.height > 0 && (rect.width < 320 || rect.height < 180)) {
        return true;
      }
    }
  } catch (_) {}

  if (videoEl.clientWidth > 0 && videoEl.clientHeight > 0 && (videoEl.clientWidth < 320 || videoEl.clientHeight < 180)) {
    return true;
  }

  return false;
}

/**
 * Resolves the specific host container for a video element without falling back to global selectors.
 */
function getHostContainer(videoEl, win = window) {
  if (!videoEl) return null;

  if (isYouTubeHost(win)) {
    if (typeof videoEl.closest === "function") {
      const moviePlayer = videoEl.closest("#movie_player");
      if (moviePlayer && moviePlayer.id !== "inline-preview-player") {
        return moviePlayer;
      }

      const shortsReel = videoEl.closest("ytd-reel-video-renderer[is-active], #shorts-player");
      if (shortsReel) {
        return shortsReel;
      }

      const playerContainer = videoEl.closest(".html5-video-player, #ytd-player");
      if (playerContainer) {
        return playerContainer;
      }
    }

    return videoEl.parentElement;
  }

  return videoEl.parentElement || videoEl;
}

function attachCosmoDownloadButton(videoEl) {
  if (!videoEl || isMiniPreview(videoEl)) return;

  const hostContainer = getHostContainer(videoEl);
  if (!hostContainer) return;

  // Prevent duplicate injections inside same host container or on same video
  if (hostContainer.querySelector(".cosmo-video-container")) {
    return;
  }

  // Ensure host container has relative positioning
  try {
    const style = window.getComputedStyle(hostContainer);
    if (style.position === "static") {
      hostContainer.style.position = "relative";
    }
  } catch (_) {}

  // Create Container
  const container = document.createElement("div");
  container.className = "cosmo-video-container";

  // Create Download Button (Flat 30px, quiet, hand-crafted)
  const btn = document.createElement("button");
  btn.className = "cosmo-video-download-btn";
  btn.type = "button";
  btn.setAttribute("aria-label", "Download Video");

  const logoSvg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  logoSvg.setAttribute("class", "cosmo-logo-icon");
  logoSvg.setAttribute("viewBox", "0 0 16 16");
  logoSvg.innerHTML = '<rect x="1.5" y="3" width="13" height="3.5" rx="1.75" fill="currentColor"/><rect x="1.5" y="9.5" width="7" height="3.5" rx="1.75" fill="currentColor" opacity=".55"/>';

  const btnText = document.createElement("span");
  btnText.className = "cosmo-btn-text";
  btnText.textContent = "Download";

  btn.appendChild(logoSvg);
  btn.appendChild(btnText);

  // Create Quality Dropdown Menu (Clean, quiet list, no badges/footers)
  const menu = document.createElement("div");
  menu.className = "cosmo-quality-menu";
  menu.setAttribute("role", "listbox");
  menu.setAttribute("aria-label", "Quality");

  QUALITIES.forEach((q) => {
    if (q.divider) {
      const hr = document.createElement("hr");
      menu.appendChild(hr);
      return;
    }

    const item = document.createElement("button");
    item.className = "cosmo-quality-item" + (q.audioOnly ? " audio" : "");
    item.type = "button";
    item.setAttribute("role", "option");
    item.setAttribute("data-format", q.format);

    const titleSpan = document.createElement("span");
    titleSpan.className = "quality-name";
    titleSpan.textContent = q.label;

    const metaSpan = document.createElement("span");
    metaSpan.className = "meta";
    metaSpan.textContent = q.meta;

    item.appendChild(titleSpan);
    item.appendChild(metaSpan);
    menu.appendChild(item);
  });

  // Prevent parent player events from interfering or closing the menu
  ["pointerdown", "mousedown", "mouseup", "click"].forEach((evtName) => {
    container.addEventListener(evtName, (e) => {
      e.stopPropagation();
    });
  });

  // Toggle dropdown on button click
  btn.addEventListener("click", (e) => {
    e.preventDefault();
    e.stopPropagation();
    e.stopImmediatePropagation();
    container.classList.toggle("open");
  });

  // Handle Quality Selection
  menu.querySelectorAll(".cosmo-quality-item").forEach((item) => {
    item.addEventListener("click", (e) => {
      e.preventDefault();
      e.stopPropagation();
      e.stopImmediatePropagation();

      const format = item.getAttribute("data-format");
      triggerDownload(videoEl, format);

      // Feedback animation
      const textEl = btn.querySelector(".cosmo-btn-text");
      const prevText = textEl ? textEl.textContent : "Download";
      if (textEl) textEl.textContent = "Sent";
      btn.classList.add("sent");

      container.classList.remove("open");

      setTimeout(() => {
        if (textEl) textEl.textContent = prevText;
        btn.classList.remove("sent");
      }, 2000);
    });
  });

  // Keep button solidly visible during user interaction
  const showBtn = (e) => {
    if (e) e.stopPropagation();
    btn.style.opacity = "1";
    btn.style.pointerEvents = "auto";
  };

  const handleMouseLeave = () => {
    if (container.classList.contains("open")) return;
    btn.style.opacity = "0.92";
  };

  hostContainer.addEventListener("mouseenter", showBtn);
  hostContainer.addEventListener("mousemove", showBtn);
  hostContainer.addEventListener("mouseleave", handleMouseLeave);
  container.addEventListener("mouseenter", showBtn);
  container.addEventListener("mousemove", showBtn);
  container.addEventListener("mouseleave", handleMouseLeave);
  btn.addEventListener("mouseenter", showBtn);
  btn.addEventListener("mousemove", showBtn);
  menu.addEventListener("mouseenter", showBtn);
  menu.addEventListener("mousemove", showBtn);

  // Close menu only when clicking outside
  const onPointerDownOutside = (e) => {
    if (!container.classList.contains("open")) return;
    const path = typeof e.composedPath === "function" ? e.composedPath() : [];
    if (!container.contains(e.target) && !path.includes(container)) {
      container.classList.remove("open");
    }
  };
  document.addEventListener("pointerdown", onPointerDownOutside);

  container.appendChild(btn);
  container.appendChild(menu);
  hostContainer.appendChild(container);

  activeAttachedButtons.set(videoEl, {
    container,
    hostContainer,
    cleanup: () => {
      document.removeEventListener("pointerdown", onPointerDownOutside);
      hostContainer.removeEventListener("mouseenter", showBtn);
      hostContainer.removeEventListener("mousemove", showBtn);
      hostContainer.removeEventListener("mouseleave", handleMouseLeave);
      container.remove();
    }
  });
}

function triggerDownload(videoEl, format) {
  let videoUrl = videoEl.src || videoEl.currentSrc || "";
  if (!videoUrl) {
    const srcEl = videoEl.querySelector ? videoEl.querySelector("source") : null;
    if (srcEl) videoUrl = srcEl.src || "";
  }

  const pageUrl = window.location.href;
  const isStreaming = pageUrl.includes("youtube.com") || pageUrl.includes("youtu.be") || pageUrl.includes("vimeo.com");
  const urlToSend = (isStreaming || !videoUrl || videoUrl.startsWith("blob:")) ? pageUrl : videoUrl;
  const pageTitle = document.title ? document.title.replace(" - YouTube", "").trim() : "video";
  const ext = format === "audio" ? ".mp3" : ".mp4";
  const safeFilename = pageTitle.replace(/[^a-zA-Z0-9_\-. ]/g, "_") + ext;

  try {
    runtime.runtime.sendMessage({
      action: "downloadVideo",
      url: urlToSend,
      pageUrl: pageUrl,
      title: pageTitle,
      filename: safeFilename,
      quality: format
    });
  } catch (err) {
    console.warn("[CosmoDownloader] Send message failed:", err);
  }
}

/**
 * Scans video elements, cleans up detached or unqualified buttons, and attaches to valid videos.
 */
function scanVideos() {
  // 1. Audit active attachments: remove if video is detached or no longer qualifies
  for (const [videoEl, record] of activeAttachedButtons.entries()) {
    if (!videoEl.isConnected || isMiniPreview(videoEl)) {
      record.cleanup();
      activeAttachedButtons.delete(videoEl);
    }
  }

  // 2. Scan all current video elements
  const videos = document.querySelectorAll("video");
  videos.forEach((videoEl) => {
    if (isMiniPreview(videoEl)) {
      if (activeAttachedButtons.has(videoEl)) {
        activeAttachedButtons.get(videoEl).cleanup();
        activeAttachedButtons.delete(videoEl);
      }
      return;
    }
    if (!activeAttachedButtons.has(videoEl)) {
      attachCosmoDownloadButton(videoEl);
    }
  });
}

// Debounced scan for dynamic mutations and SPA navigation
let scanDebounceTimer = null;
function debouncedScanVideos() {
  if (scanDebounceTimer) clearTimeout(scanDebounceTimer);
  scanDebounceTimer = setTimeout(scanVideos, 150);
}

// Observe dynamic video additions
if (typeof MutationObserver !== "undefined" && typeof document !== "undefined" && document.documentElement) {
  const observer = new MutationObserver(() => {
    debouncedScanVideos();
  });
  observer.observe(document.documentElement, { childList: true, subtree: true });

  // Initial scan
  scanVideos();
  setInterval(scanVideos, 2500);

  // SPA navigation hooks
  window.addEventListener("yt-navigate-finish", () => {
    scanVideos();
    setTimeout(scanVideos, 300);
  });
  window.addEventListener("spfdone", scanVideos);
  window.addEventListener("popstate", scanVideos);
  window.addEventListener("hashchange", scanVideos);
}

// Export for Node unit testing if applicable
if (typeof module !== "undefined" && module.exports) {
  module.exports = { isMiniPreview, isYouTubeHost, getHostContainer };
}
