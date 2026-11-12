/**
 * Cosmo Downloader v1.11 — Content Script (Chrome, Edge, Brave, Opera, Firefox)
 * 
 * Injects a compact, floating cosmic download button on qualified videos
 * with an interactive IDM-style quality/format selector menu.
 * 
 * Strictly suppresses buttons on hover mini-previews, thumbnails, and inactive players.
 */

"use strict";

const runtime = typeof browser !== "undefined" ? browser : (typeof chrome !== "undefined" ? chrome : null);

const QUALITIES = [
  { label: "1080p Full HD", badge: "60fps MP4", format: "1080p", audioOnly: false },
  { label: "720p High Def", badge: "HD MP4", format: "720p", audioOnly: false },
  { label: "480p Standard", badge: "SD MP4", format: "480p", audioOnly: false },
  { label: "360p Low Bandwidth", badge: "MP4", format: "360p", audioOnly: false },
  { label: "Audio Only", badge: "MP3 320k", format: "audio", audioOnly: true },
  { label: "Best Available", badge: "Auto Engine", format: "best", audioOnly: false }
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
  if (hostContainer.querySelector(".cosmo-video-host")) {
    return;
  }

  // Ensure host container has relative positioning
  try {
    const style = window.getComputedStyle(hostContainer);
    if (style.position === "static") {
      hostContainer.style.position = "relative";
    }
  } catch (_) {}

  // Create Host Container with Shadow DOM for CSS isolation
  const host = document.createElement("div");
  host.className = "cosmo-video-host";
  host.style.position = "absolute";
  host.style.top = "12px";
  host.style.right = "12px";
  host.style.zIndex = "2147483647";
  host.style.display = "block";
  host.style.pointerEvents = "auto";
  host.style.userSelect = "none";

  const shadow = host.attachShadow({ mode: "open" });

  const styleEl = document.createElement("style");
  styleEl.textContent = `
    :host {
      --bg:#f3f4f6; --surface:#ffffff; --raise:#eceef1; --line:rgba(20,24,30,.1);
      --text:#1b1d21; --muted:#646b75; --accent:#44546a; --on-accent:#ffffff; --accent-soft:rgba(68,84,106,.12);
      --danger:#d93025;
      --s1:4px; --s2:8px; --s3:12px; --s4:16px;
      --r-sm:6px; --r:10px;
      --font:'Geist','Segoe UI',system-ui,-apple-system,Roboto,sans-serif;
      --t-body:14px; --t-small:12px;
      --ease:160ms ease-out;
      display: block;
      font-family: var(--font);
    }
    @media (prefers-color-scheme:dark){
      :host{
        --bg:#1a1c20; --surface:#22252a; --raise:#2b2f35; --line:rgba(255,255,255,.08);
        --text:#e6e8eb; --muted:#9097a1; --accent:#8fa6c4; --on-accent:#0f1a2a; --accent-soft:rgba(143,166,196,.16);
      }
    }
    *, *::before, *::after {
      box-sizing: border-box;
      margin: 0;
      padding: 0;
    }
    .dl {
      position: relative;
      display: flex;
      flex-direction: column;
      align-items: flex-end;
      gap: var(--s1);
    }
    .btn {
      height: 28px;
      padding: 0 var(--s2);
      gap: 6px;
      font-size: var(--t-small);
      font-family: var(--font);
      background: var(--surface);
      border: 1px solid var(--line);
      border-radius: var(--r-sm);
      color: var(--text);
      font-weight: 500;
      display: inline-flex;
      align-items: center;
      cursor: pointer;
      box-shadow: 0 1px 3px rgba(0,0,0,.08);
      transition: background var(--ease), border-color var(--ease);
      white-space: nowrap;
    }
    .btn:hover {
      background: var(--raise);
    }
    .btn:focus-visible {
      outline: 2px solid var(--accent);
      outline-offset: 2px;
    }
    .btn img {
      width: 16px;
      height: 16px;
      border-radius: 3px;
      flex-shrink: 0;
    }
    .btn svg.i {
      width: 14px;
      height: 14px;
      flex: none;
      fill: none;
      stroke: currentColor;
      stroke-width: 1.6;
      stroke-linecap: round;
      stroke-linejoin: round;
      transition: transform var(--ease);
    }
    .dl.open .btn svg.i {
      transform: rotate(180deg);
    }
    .menu {
      position: absolute;
      top: calc(100% + var(--s1));
      right: 0;
      width: 176px;
      background: var(--surface);
      border: 1px solid var(--line);
      border-radius: var(--r);
      padding: var(--s1);
      box-shadow: 0 8px 24px rgba(0,0,0,.18);
      display: flex;
      flex-direction: column;
      gap: 1px;
      z-index: 1000;
    }
    .menu[hidden] {
      display: none;
    }
    .dl.flip-up .menu {
      top: auto;
      bottom: calc(100% + var(--s1));
    }
    .menu button {
      display: flex;
      justify-content: space-between;
      gap: var(--s3);
      width: 100%;
      height: 30px;
      align-items: center;
      padding: 0 var(--s2);
      font-size: 13px;
      font-family: var(--font);
      color: var(--text);
      border: 0;
      border-radius: var(--r-sm);
      background: transparent;
      text-align: left;
      cursor: pointer;
      transition: background var(--ease);
    }
    .menu button:hover, .menu button:focus-visible {
      background: var(--accent-soft);
      outline: none;
    }
    .menu button .size {
      font-size: var(--t-small);
      color: var(--muted);
      font-variant-numeric: tabular-nums;
    }
    .menu hr {
      border: 0;
      border-top: 1px solid var(--line);
      margin: var(--s1) var(--s2);
    }
    @media (prefers-reduced-motion: reduce) {
      *, *::before, *::after {
        transition-duration: 0ms !important;
        animation-duration: 0ms !important;
      }
    }
  `;
  shadow.appendChild(styleEl);

  const dlContainer = document.createElement("div");
  dlContainer.className = "dl";

  const btn = document.createElement("button");
  btn.className = "btn";
  btn.type = "button";
  btn.setAttribute("aria-label", "Download");
  btn.setAttribute("aria-haspopup", "listbox");
  btn.setAttribute("aria-expanded", "false");

  const logoImg = document.createElement("img");
  try {
    logoImg.src = runtime.runtime.getURL("icons/icon-16.png");
  } catch (_) {
    logoImg.src = "";
  }
  logoImg.alt = "Cosmo";

  const btnText = document.createElement("span");
  btnText.textContent = "Download";

  const svgNS = "http://www.w3.org/2000/svg";
  const chevSvg = document.createElementNS(svgNS, "svg");
  chevSvg.setAttribute("class", "i");
  chevSvg.setAttribute("viewBox", "0 0 24 24");
  const chevPath = document.createElementNS(svgNS, "path");
  chevPath.setAttribute("d", "M7 10l5 5 5-5");
  chevSvg.appendChild(chevPath);

  btn.appendChild(logoImg);
  btn.appendChild(btnText);
  btn.appendChild(chevSvg);

  const menu = document.createElement("div");
  menu.className = "menu";
  menu.setAttribute("role", "listbox");
  menu.setAttribute("aria-label", "Select quality");
  menu.hidden = true;

  // Reference items: 1080p, 720p, 480p, 360p, divider, Audio only
  const MENU_ITEMS = [
    { label: "1080p", size: "148 MB", format: "1080p" },
    { label: "720p", size: "82 MB", format: "720p" },
    { label: "480p", size: "41 MB", format: "480p" },
    { label: "360p", size: "24 MB", format: "360p" },
    null,
    { label: "Audio only", size: "9 MB", format: "audio" }
  ];

  MENU_ITEMS.forEach((item) => {
    if (!item) {
      menu.appendChild(document.createElement("hr"));
      return;
    }

    const rowBtn = document.createElement("button");
    rowBtn.type = "button";
    rowBtn.setAttribute("role", "option");
    rowBtn.dataset.format = item.format;

    const labelSpan = document.createElement("span");
    labelSpan.textContent = item.label;

    const sizeSpan = document.createElement("span");
    sizeSpan.className = "size";
    sizeSpan.textContent = item.size;

    rowBtn.appendChild(labelSpan);
    rowBtn.appendChild(sizeSpan);
    menu.appendChild(rowBtn);
  });

  const CAPTURE_STOP_EVENTS = ["pointerdown", "mousedown", "mouseup", "pointerup", "click"];
  CAPTURE_STOP_EVENTS.forEach((evtName) => {
    host.addEventListener(evtName, (e) => e.stopPropagation());
    btn.addEventListener(evtName, (e) => e.stopPropagation(), { capture: true });
  });

  const openMenu = () => {
    menu.hidden = false;
    dlContainer.classList.add("open");
    btn.setAttribute("aria-expanded", "true");

    try {
      const btnRect = btn.getBoundingClientRect();
      const spaceBelow = window.innerHeight - btnRect.bottom;
      if (spaceBelow < 220 && btnRect.top > spaceBelow) {
        dlContainer.classList.add("flip-up");
      } else {
        dlContainer.classList.remove("flip-up");
      }
    } catch (_) {}
  };

  const closeMenu = () => {
    menu.hidden = true;
    dlContainer.classList.remove("open", "flip-up");
    btn.setAttribute("aria-expanded", "false");
  };

  const toggleMenu = () => {
    if (menu.hidden) {
      openMenu();
    } else {
      closeMenu();
    }
  };

  btn.addEventListener("click", (e) => {
    e.preventDefault();
    e.stopPropagation();
    toggleMenu();
  }, { capture: true });

  menu.querySelectorAll("button").forEach((row) => {
    CAPTURE_STOP_EVENTS.forEach((evtName) => {
      row.addEventListener(evtName, (e) => e.stopPropagation(), { capture: true });
    });

    row.addEventListener("click", (e) => {
      e.preventDefault();
      e.stopPropagation();

      const format = row.dataset.format;
      triggerDownload(videoEl, format);

      closeMenu();

      btnText.textContent = "Sent to Cosmo";
      setTimeout(() => {
        btnText.textContent = "Download";
      }, 1600);
    }, { capture: true });
  });

  const onPointerDownOutside = (e) => {
    if (menu.hidden) return;
    const path = typeof e.composedPath === "function" ? e.composedPath() : [];
    if (!path.includes(host) && !host.contains(e.target)) {
      closeMenu();
    }
  };
  document.addEventListener("pointerdown", onPointerDownOutside);

  const onKeyDown = (e) => {
    if (e.key === "Escape" && !menu.hidden) {
      closeMenu();
      btn.focus();
    }
  };
  document.addEventListener("keydown", onKeyDown);

  dlContainer.appendChild(btn);
  dlContainer.appendChild(menu);
  shadow.appendChild(dlContainer);
  hostContainer.appendChild(host);

  activeAttachedButtons.set(videoEl, {
    container: host,
    hostContainer,
    cleanup: () => {
      document.removeEventListener("pointerdown", onPointerDownOutside);
      document.removeEventListener("keydown", onKeyDown);
      host.remove();
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
