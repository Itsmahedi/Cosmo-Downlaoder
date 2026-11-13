# Cosmo Downloader v1.2.3

A fast, reliable multi-segment download manager for **Windows 10 and Windows 11**, paired with redesigned browser extensions for Google Chrome, Microsoft Edge, Brave, Opera, and Mozilla Firefox.

---

## What's New in v1.2.3

### Redesigned "New download" File Info Popup Dialog
- **Modern Compact Card UI**: Completely updated the "Download File Info" popup dialog across both the native Windows desktop app (`crates/app/src/dialogs/add_download.rs`) and the standalone / dashboard HTML mockup (`Download_popup.html` and Section 4 of `Cosmo downloader NEWWW.html`).
- **Branded Header**: Displays the official cosmic glowing emblem, `New download` title, and subtle top-right `✕` dismiss button.
- **Enhanced Typography & Anti-Clipping**:
  - Replaced unclipped raw text painter with auto-truncating layout blocks for both the filename and metadata line.
  - Generous vertical separation ensures complex Unicode/UTF-8 filenames (e.g. Bengali script) and adaptive stream source tags never overlap or glitch.
  - Dedicated $36 \times 36\text{ px}$ squircle badge with soft accent background housing the category vector icon.
- **Two-Column Inline Controls**:
  - **Category Selector**: Clean dropdown with perfect vertical text centering and a single clean native chevron.
  - **Save to**: Inset path field paired with an intuitive vector folder button (`📁`) for directory picking.
- **Preferences & Actions**:
  - Smart remember folder checkbox: `Use this folder for [category] next time`.
  - Right-aligned footer buttons: neutral outline `[ Cancel ]` button and high-contrast accent `[ 📥 Start download ]` CTA.

### Dashboard Showcase Integration
- Embedded **Section 4 · Download File Info popup** into `Cosmo downloader NEWWW.html`, completing the 4-part live showcase suite:
  1. Main Desktop App
  2. Browser Extension Popup
  3. Video Download Button & Quality Menu
  4. Download File Info Popup

### Build & Version Hygiene
- Bumped workspace version to `1.2.3` across `Cargo.toml`, Windows PE `.rc` resource files, Inno Setup installer, and extension manifests.

---

## Release Assets

| File | Description |
| :--- | :--- |
| `CosmoDownloader-Setup-v1.2.3.exe` | **Windows Installer (Recommended)** — Installs Cosmo Downloader (`v1.2.3`), desktop/start shortcuts, `yt-dlp` / `ffmpeg`, and registers browser native messaging hosts. Seamlessly upgrades existing installations. |
| `CosmoDownloader.exe` | Standalone Windows desktop application binary (`v1.2.3`). |
| `CosmoDownloader-Chrome-Edge-Brave-Opera-Extension-v1.2.3.zip` | **All-in-One** unified browser extension package for Google Chrome, Microsoft Edge, Brave, and Opera (`v1.2.3`). |

---

## Quick Install (Windows)

1. Download and run **`CosmoDownloader-Setup-v1.2.3.exe`**.
2. **Chrome / Microsoft Edge / Brave / Opera:**
   - Extract **`CosmoDownloader-Chrome-Edge-Brave-Opera-Extension-v1.2.3.zip`**.
   - Open your browser's extensions management page (`chrome://extensions`, `edge://extensions`, `brave://extensions`, `opera://extensions`).
   - Enable **Developer mode**.
   - Click **Load unpacked** and select the extracted extension folder.
3. **Mozilla Firefox:**
   - Distributed via Mozilla Add-ons (AMO).
