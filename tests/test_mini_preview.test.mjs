import { test } from "node:test";
import assert from "node:assert/strict";
import { isMiniPreview, isYouTubeHost, getHostContainer } from "../browser-extensions/Chrome-Edge-Brave-Opera/content.js";

/**
 * Lightweight mock element supporting closest(), querySelector(), getBoundingClientRect(), classList
 */
function createMockElement({ tagName = "div", id = "", className = "", attributes = {}, parent = null, rect = { width: 640, height: 360 } } = {}) {
  const el = {
    tagName: tagName.toUpperCase(),
    id,
    className,
    attributes: { ...attributes },
    parentElement: parent,
    children: [],
    classList: {
      contains: (cls) => el.className.split(/\s+/).includes(cls),
      add: (cls) => {
        if (!el.classList.contains(cls)) {
          el.className = (el.className + " " + cls).trim();
        }
      }
    },
    clientWidth: rect.width,
    clientHeight: rect.height,
    getBoundingClientRect: () => ({ ...rect }),
    closest: (selector) => {
      const selectors = selector.split(",").map(s => s.trim());
      let cur = el;
      while (cur) {
        for (const sel of selectors) {
          if (sel.startsWith("#") && cur.id === sel.slice(1)) return cur;
          if (sel.startsWith(".") && cur.classList.contains(sel.slice(1))) return cur;
          if (sel.includes("[is-active]") && cur.attributes["is-active"]) {
            const baseTag = sel.replace("[is-active]", "").toUpperCase();
            if (!baseTag || cur.tagName === baseTag) return cur;
          }
          if (cur.tagName === sel.toUpperCase()) return cur;
        }
        cur = cur.parentElement;
      }
      return null;
    },
    querySelector: (selector) => {
      // Simple child search
      for (const child of el.children) {
        if (selector === "video.html5-main-video" && child.tagName === "VIDEO" && child.classList.contains("html5-main-video")) {
          return child;
        }
      }
      return null;
    }
  };

  if (parent) {
    parent.children.push(el);
  }
  return el;
}

test("YouTube host detection", () => {
  assert.equal(isYouTubeHost({ location: { hostname: "www.youtube.com" } }), true);
  assert.equal(isYouTubeHost({ location: { hostname: "youtube.com" } }), true);
  assert.equal(isYouTubeHost({ location: { hostname: "m.youtube.com" } }), true);
  assert.equal(isYouTubeHost({ location: { hostname: "notyoutube.com" } }), false);
  assert.equal(isYouTubeHost({ location: { hostname: "vimeo.com" } }), false);
});

test("YouTube Watch Page - Main Player should NOT be treated as mini preview", () => {
  const win = { location: { hostname: "www.youtube.com", pathname: "/watch" } };
  const moviePlayer = createMockElement({ tagName: "div", id: "movie_player" });
  const mainVideo = createMockElement({ tagName: "video", className: "html5-main-video video-stream", parent: moviePlayer });

  assert.equal(isMiniPreview(mainVideo, win), false);
  assert.equal(getHostContainer(mainVideo, win), moviePlayer);
});

test("YouTube Home Feed - Hover preview should be suppressed", () => {
  const win = { location: { hostname: "www.youtube.com", pathname: "/" } };
  const previewContainer = createMockElement({ tagName: "ytd-video-preview" });
  const previewVideo = createMockElement({ tagName: "video", parent: previewContainer });

  assert.equal(isMiniPreview(previewVideo, win), true);
});

test("YouTube Inline Preview Player - should be suppressed", () => {
  const win = { location: { hostname: "www.youtube.com", pathname: "/" } };
  const inlinePlayer = createMockElement({ tagName: "div", id: "inline-preview-player" });
  const previewVideo = createMockElement({ tagName: "video", parent: inlinePlayer });

  assert.equal(isMiniPreview(previewVideo, win), true);
});

test("YouTube Sidebar Thumbnail Preview - should be suppressed", () => {
  const win = { location: { hostname: "www.youtube.com", pathname: "/watch" } };
  const thumbnail = createMockElement({ tagName: "ytd-thumbnail" });
  const thumbVideo = createMockElement({ tagName: "video", parent: thumbnail });

  assert.equal(isMiniPreview(thumbVideo, win), true);
});

test("YouTube Search Results - Hover preview should be suppressed", () => {
  const win = { location: { hostname: "www.youtube.com", pathname: "/results" } };
  const previewRenderer = createMockElement({ tagName: "ytd-inline-preview-renderer" });
  const previewVideo = createMockElement({ tagName: "video", parent: previewRenderer });

  assert.equal(isMiniPreview(previewVideo, win), true);
});

test("YouTube Shorts - Active reel should be allowed", () => {
  const win = { location: { hostname: "www.youtube.com", pathname: "/shorts/123456789" } };
  const activeReel = createMockElement({ tagName: "ytd-reel-video-renderer", attributes: { "is-active": "true" } });
  const shortsVideo = createMockElement({ tagName: "video", parent: activeReel });

  assert.equal(isMiniPreview(shortsVideo, win), false);
  assert.equal(getHostContainer(shortsVideo, win), activeReel);
});

test("YouTube Shorts - Inactive reel should be suppressed", () => {
  const win = { location: { hostname: "www.youtube.com", pathname: "/shorts/123456789" } };
  const inactiveReel = createMockElement({ tagName: "ytd-reel-video-renderer", attributes: {} });
  const inactiveVideo = createMockElement({ tagName: "video", parent: inactiveReel });

  assert.equal(isMiniPreview(inactiveVideo, win), true);
});

test("Non-YouTube Site - Standard video should be allowed", () => {
  const win = { location: { hostname: "vimeo.com", pathname: "/12345678" } };
  const parent = createMockElement({ tagName: "div" });
  const video = createMockElement({ tagName: "video", parent, rect: { width: 800, height: 450 } });

  assert.equal(isMiniPreview(video, win), false);
  assert.equal(getHostContainer(video, win), parent);
});

test("Non-YouTube Site - Tiny video preview (<320x180) should be suppressed", () => {
  const win = { location: { hostname: "example.com", pathname: "/feed" } };
  const parent = createMockElement({ tagName: "div" });
  const tinyVideo = createMockElement({ tagName: "video", parent, rect: { width: 240, height: 135 } });

  assert.equal(isMiniPreview(tinyVideo, win), true);
});
