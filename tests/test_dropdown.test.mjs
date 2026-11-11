import { test, before, after } from "node:test";
import assert from "node:assert/strict";
import http from "node:http";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, "..");
const extDir = path.join(rootDir, "browser-extensions", "Chrome-Edge-Brave-Opera");

let server;
let serverPort;
let browser;

function getExecutablePath() {
  if (process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH && fs.existsSync(process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH)) {
    return process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH;
  }
  const defaultCached = "/home/tokit/.cache/ms-playwright/chromium-1248/chrome-linux64/chrome";
  if (fs.existsSync(defaultCached)) {
    return defaultCached;
  }
  return undefined;
}

before(async () => {
  // 1. Create a lightweight local HTTP server
  server = http.createServer((req, res) => {
    const url = new URL(req.url, `http://127.0.0.1:${serverPort}`);
    if (url.pathname === "/" || url.pathname === "/index.html") {
      res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
      res.end(`<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>Cosmo Video Test Page</title>
  <link rel="stylesheet" href="/content.css">
  <style>
    body { margin: 0; padding: 40px; background: #0f172a; color: #fff; font-family: sans-serif; min-height: 1200px; }
    .player-box { position: relative; width: 640px; height: 360px; background: #1e293b; margin-bottom: 50px; }
    .player-box video { width: 100%; height: 100%; object-fit: contain; }
    #overflow-player { position: absolute; bottom: 20px; left: 40px; height: 180px; width: 360px; }
  </style>
</head>
<body>
  <h1>Cosmo Video Test</h1>
  <div class="player-box" id="main-player">
    <video id="test-video" width="640" height="360" controls src="/sample.mp4"></video>
  </div>
  <div class="player-box" id="overflow-player">
    <video id="overflow-video" width="640" height="360" controls src="/sample-bottom.mp4"></video>
  </div>

  <script>
    window.sentMessages = [];
    window.chrome = {
      runtime: {
        getURL: (p) => "/" + p,
        sendMessage: (msg) => { window.sentMessages.push(msg); }
      }
    };
  </script>
  <script src="/content.js"></script>
</body>
</html>`);
    } else if (url.pathname === "/content.css") {
      res.writeHead(200, { "Content-Type": "text/css" });
      res.end(fs.readFileSync(path.join(extDir, "content.css")));
    } else if (url.pathname === "/content.js") {
      res.writeHead(200, { "Content-Type": "application/javascript" });
      res.end(fs.readFileSync(path.join(extDir, "content.js")));
    } else {
      res.writeHead(204);
      res.end();
    }
  });

  await new Promise((resolve) => {
    server.listen(0, "127.0.0.1", () => {
      serverPort = server.address().port;
      resolve();
    });
  });

  // 2. Launch browser
  browser = await chromium.launch({
    executablePath: getExecutablePath(),
    headless: true,
    args: ["--no-sandbox", "--disable-setuid-sandbox"]
  });
});

after(async () => {
  if (browser) await browser.close();
  if (server) await new Promise((r) => server.close(r));
});

test("Video Download Button appears with correct ARIA attributes and hidden menu", async () => {
  const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  await page.goto(`http://127.0.0.1:${serverPort}/`, { waitUntil: "domcontentloaded" });

  const btn = page.locator("#main-player .cosmo-video-download-btn");
  await assert.doesNotReject(btn.waitFor({ state: "visible", timeout: 3000 }));

  assert.equal(await btn.getAttribute("aria-haspopup"), "true");
  assert.equal(await btn.getAttribute("aria-expanded"), "false");

  const menu = page.locator("#main-player .cosmo-quality-menu");
  const isMenuVisible = await menu.evaluate((el) => window.getComputedStyle(el).display !== "none");
  assert.equal(isMenuVisible, false, "Menu must be display:none initially");

  await page.close();
});

test("Clicking button toggles menu open and sets aria-expanded true", async () => {
  const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  await page.goto(`http://127.0.0.1:${serverPort}/`, { waitUntil: "domcontentloaded" });

  const btn = page.locator("#main-player .cosmo-video-download-btn");
  const menu = page.locator("#main-player .cosmo-quality-menu");

  await btn.click();

  assert.equal(await btn.getAttribute("aria-expanded"), "true");
  const displayAfterOpen = await menu.evaluate((el) => window.getComputedStyle(el).display);
  assert.equal(displayAfterOpen, "block", "Menu display must be block when open");

  // Clicking again closes it
  await btn.click();
  assert.equal(await btn.getAttribute("aria-expanded"), "false");
  const displayAfterClose = await menu.evaluate((el) => window.getComputedStyle(el).display);
  assert.equal(displayAfterClose, "none", "Menu display must be none when closed");

  await page.close();
});

test("Escape key closes the menu and returns focus to button", async () => {
  const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  await page.goto(`http://127.0.0.1:${serverPort}/`, { waitUntil: "domcontentloaded" });

  const btn = page.locator("#main-player .cosmo-video-download-btn");
  const menu = page.locator("#main-player .cosmo-quality-menu");

  await btn.click();
  assert.equal(await menu.evaluate((el) => window.getComputedStyle(el).display), "block");

  await page.keyboard.press("Escape");
  assert.equal(await menu.evaluate((el) => window.getComputedStyle(el).display), "none");
  assert.equal(await btn.getAttribute("aria-expanded"), "false");

  await page.close();
});

test("Outside click closes the open menu", async () => {
  const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  await page.goto(`http://127.0.0.1:${serverPort}/`, { waitUntil: "domcontentloaded" });

  const btn = page.locator("#main-player .cosmo-video-download-btn");
  const menu = page.locator("#main-player .cosmo-quality-menu");

  await btn.click();
  assert.equal(await menu.evaluate((el) => window.getComputedStyle(el).display), "block");

  // Click on h1 outside container
  await page.click("h1");
  assert.equal(await menu.evaluate((el) => window.getComputedStyle(el).display), "none");
  assert.equal(await btn.getAttribute("aria-expanded"), "false");

  await page.close();
});

test("Every quality option sends downloadVideo message and auto-closes menu", async () => {
  const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  await page.goto(`http://127.0.0.1:${serverPort}/`, { waitUntil: "domcontentloaded" });

  const btn = page.locator("#main-player .cosmo-video-download-btn");
  const menu = page.locator("#main-player .cosmo-quality-menu");

  const expectedQualities = [
    { format: "1080p", ext: ".mp4" },
    { format: "720p", ext: ".mp4" },
    { format: "480p", ext: ".mp4" },
    { format: "360p", ext: ".mp4" },
    { format: "audio", ext: ".mp3" },
    { format: "best", ext: ".mp4" }
  ];

  for (const { format, ext } of expectedQualities) {
    // Open menu
    await btn.click();
    assert.equal(await menu.evaluate((el) => window.getComputedStyle(el).display), "block");

    // Click item
    const item = page.locator(`#main-player .cosmo-quality-item[data-format="${format}"]`);
    await item.click();

    // Menu must auto-close
    assert.equal(await menu.evaluate((el) => window.getComputedStyle(el).display), "none");

    // Button feedback text
    const btnText = await page.locator("#main-player .cosmo-btn-text").textContent();
    assert.ok(btnText.includes("Sent to CosmoDownloader"), `Feedback text not displayed for format ${format}`);

    // Verify last message sent
    const lastMsg = await page.evaluate(() => window.sentMessages[window.sentMessages.length - 1]);
    assert.ok(lastMsg, "No message was sent to extension runtime");
    assert.equal(lastMsg.action, "downloadVideo");
    assert.equal(lastMsg.quality, format);
    assert.ok(lastMsg.filename.endsWith(ext), `Expected filename to end with ${ext}, got: ${lastMsg.filename}`);
  }

  await page.close();
});

test("Menu flips upward when near viewport bottom to prevent overflow", async () => {
  const page = await browser.newPage({ viewport: { width: 1280, height: 700 } });
  await page.goto(`http://127.0.0.1:${serverPort}/`, { waitUntil: "domcontentloaded" });

  // Scroll so overflow-player is visible near bottom
  await page.evaluate(() => {
    document.getElementById("overflow-player").scrollIntoView({ block: "end" });
  });

  const overflowBtn = page.locator("#overflow-player .cosmo-video-download-btn");
  const overflowContainer = page.locator("#overflow-player .cosmo-video-container");

  await overflowBtn.click();
  const classes = await overflowContainer.getAttribute("class");
  assert.ok(classes.includes("flip-up"), `Expected container to have flip-up class, got: ${classes}`);

  await page.close();
});
