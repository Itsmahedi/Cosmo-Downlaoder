#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, "..");

const srcDir = path.join(rootDir, "browser-extensions", "Chrome-Edge-Brave-Opera");
const destDir = path.join(rootDir, "browser-extensions", "Mozilla-Firefox");

const sharedFiles = [
  "content.css",
  "content.js",
  "background.js",
  "popup.css",
  "popup.html",
  "popup.js"
];

console.log("Synchronizing extension shared files from Chrome-Edge-Brave-Opera to Mozilla-Firefox...");

for (const file of sharedFiles) {
  const srcPath = path.join(srcDir, file);
  const destPath = path.join(destDir, file);
  if (fs.existsSync(srcPath)) {
    fs.copyFileSync(srcPath, destPath);
    console.log(`  ✓ ${file}`);
  } else {
    console.warn(`  ⚠ Warning: ${file} not found in source directory.`);
  }
}

console.log("Extension synchronization complete.");

