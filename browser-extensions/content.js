/**
 * âœ¦ Cosmo Downloader v1.6 â€” Chrome/Edge/Brave/Opera Content Script
 * 
 * Injects a floating cosmic download button on videos (YouTube, Vimeo, Dailymotion, HTML5)
 * with an interactive IDM-style quality/format selector menu.
 */

"use strict";

const runtime = typeof browser !== "undefined" ? browser : chrome;

const QUALITIES = [
  { label: "1080p Full HD", badge: "60fps MP4", format: "1080p", audioOnly: false },
  { label: "720p High Def", badge: "HD MP4", format: "720p", audioOnly: false },
  { label: "480p Standard", badge: "SD MP4", format: "480p", audioOnly: false },
  { label: "360p Low Bandwidth", badge: "MP4", format: "360p", audioOnly: false },
  { label: "Audio Only", badge: "MP3 320k", format: "audio", audioOnly: true },
  { label: "Best Available", badge: "Auto Engine", format: "best", audioOnly: false }
];

function attachCosmoDownloadButton(videoEl) {
  if (!videoEl) return;

  // Identify best host container for the player
  let hostContainer = null;
  const isYouTube = window.location.hostname.includes("youtube.com");
  
  if (isYouTube) {
    hostContainer = document.querySelector("#movie_player") || 
                    document.querySelector(".html5-video-player") || 
                    document.querySelector("#ytd-player") || 
                    videoEl.parentElement;
  } else {
    hostContainer = videoEl.parentElement || videoEl;
  }

  if (!hostContainer) return;

  // Prevent duplicate injections inside same container
  if (hostContainer.querySelector(".cosmo-video-container")) return;

  // Ensure host container has relative positioning
  const style = window.getComputedStyle(hostContainer);
  if (style.position === "static") {
    hostContainer.style.position = "relative";
  }

  // Create Container
  const container = document.createElement("div");
  container.className = "cosmo-video-container";

  // Create Download Button
  const btn = document.createElement("button");
  btn.className = "cosmo-video-download-btn";
  btn.type = "button";
  btn.setAttribute("aria-label", "Download Video with Cosmo Downloader");

  const logoImg = document.createElement("img");
  logoImg.src = runtime.runtime.getURL("icons/icon-32.png");
  logoImg.alt = "Cosmo Downloader";
  logoImg.className = "cosmo-btn-logo";

  const btnText = document.createElement("span");
  btnText.className = "cosmo-btn-text";
  btnText.textContent = "Download Video";

  const svgNS = "http://www.w3.org/2000/svg";
  const arrowSvg = document.createElementNS(svgNS, "svg");
  arrowSvg.setAttribute("class", "cosmo-arrow-down");
  arrowSvg.setAttribute("viewBox", "0 0 24 24");
  arrowSvg.setAttribute("fill", "none");
  arrowSvg.setAttribute("stroke", "currentColor");
  arrowSvg.setAttribute("stroke-width", "2.5");
  arrowSvg.setAttribute("stroke-linecap", "round");
  arrowSvg.setAttribute("stroke-linejoin", "round");

  const arrowPl = document.createElementNS(svgNS, "polyline");
  arrowPl.setAttribute("points", "6 9 12 15 18 9");
  arrowSvg.appendChild(arrowPl);

  btn.appendChild(logoImg);
  btn.appendChild(btnText);
  btn.appendChild(arrowSvg);

  // Create Quality Dropdown Menu
  const menu = document.createElement("div");
  menu.className = "cosmo-quality-menu";

  const headerDiv = document.createElement("div");
  headerDiv.className = "cosmo-menu-header";

  const headerLeft = document.createElement("div");
  headerLeft.style.display = "flex";
  headerLeft.style.alignItems = "center";
  headerLeft.style.gap = "6px";

  const menuLogo = document.createElement("img");
  menuLogo.src = runtime.runtime.getURL("icons/icon-32.png");
  menuLogo.style.width = "16px";
  menuLogo.style.height = "16px";
  menuLogo.style.borderRadius = "50%";
  menuLogo.alt = "Cosmo Downloader";

  const headerSpan1 = document.createElement("span");
  headerSpan1.textContent = "SELECT QUALITY";

  headerLeft.appendChild(menuLogo);
  headerLeft.appendChild(headerSpan1);

  const headerSpan2 = document.createElement("span");
  headerSpan2.textContent = "COSMO DOWNLOADER";
  headerDiv.appendChild(headerLeft);
  headerDiv.appendChild(headerSpan2);
  menu.appendChild(headerDiv);

  QUALITIES.forEach((q) => {
    const item = document.createElement("div");
    item.className = "cosmo-quality-item" + (q.audioOnly ? " audio" : "");
    item.setAttribute("data-format", q.format);

    const titleSpan = document.createElement("span");
    titleSpan.className = "quality-name";
    titleSpan.textContent = q.label;

    const badgeSpan = document.createElement("span");
    badgeSpan.className = "badge";
    badgeSpan.textContent = q.badge;

    item.appendChild(titleSpan);
    item.appendChild(badgeSpan);
    menu.appendChild(item);
  });

  const footerDiv = document.createElement("div");
  footerDiv.className = "cosmo-menu-footer";
  footerDiv.textContent = "\u2726 Warp Engine Acceleration Active";
  menu.appendChild(footerDiv);

  // Prevent YouTube or player wrapper events from interfering or closing the menu
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
      const btnText = btn.querySelector(".cosmo-btn-text");
      const prevText = btnText ? btnText.textContent : "Download Video";
      if (btnText) btnText.textContent = "\u2726 Sent to CosmoDownloader!";
      btn.style.background = "linear-gradient(135deg, #00C853 0%, #00f0ff 100%)";

      container.classList.remove("open");

      setTimeout(() => {
        if (btnText) btnText.textContent = prevText;
        btn.style.background = "";
      }, 2500);
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
  document.addEventListener("pointerdown", (e) => {
    if (!container.classList.contains("open")) return;
    const path = typeof e.composedPath === "function" ? e.composedPath() : [];
    if (!container.contains(e.target) && !path.includes(container)) {
      container.classList.remove("open");
    }
  });

  container.appendChild(btn);
  container.appendChild(menu);
  hostContainer.appendChild(container);
}

function triggerDownload(videoEl, format) {
  let videoUrl = videoEl.src || videoEl.currentSrc || "";
  if (!videoUrl) {
    const srcEl = videoEl.querySelector("source");
    if (srcEl) videoUrl = srcEl.src || "";
  }

  const pageUrl = window.location.href;
  const isStreaming = pageUrl.includes("youtube.com") || pageUrl.includes("youtu.be") || pageUrl.includes("vimeo.com");
  const urlToSend = (isStreaming || !videoUrl || videoUrl.startsWith("blob:")) ? pageUrl : videoUrl;
  const pageTitle = document.title ? document.title.replace(" - YouTube", "").trim() : "video";
  const ext = format === "audio" ? ".mp3" : ".mp4";
  const safeFilename = pageTitle.replace(/[^a-zA-Z0-9_\-. ]/g, "_") + ext;

  // Send message directly to background script for 0ms instantaneous popup.
  // The background worker has native permissions and communicates with CosmoDownloader
  // without triggering any browser Private Network Access security prompts.
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

function scanVideos() {
  const videos = document.querySelectorAll("video");
  videos.forEach(attachCosmoDownloadButton);
}

// Observe dynamic video additions
const observer = new MutationObserver(() => {
  scanVideos();
});

observer.observe(document.documentElement, { childList: true, subtree: true });
scanVideos();
setInterval(scanVideos, 2000);

// SPA navigation hooks
window.addEventListener("yt-navigate-finish", scanVideos);
window.addEventListener("spfdone", scanVideos);
window.addEventListener("popstate", scanVideos);
