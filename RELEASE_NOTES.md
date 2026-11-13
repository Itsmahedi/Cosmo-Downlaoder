# Cosmo Downloader v1.2.4

A fast, reliable multi-segment download manager for **Windows 10 and Windows 11**, paired with redesigned browser extensions for Google Chrome, Microsoft Edge, Brave, Opera, and Mozilla Firefox.

---

## What's New in v1.2.4

### Popup UI Fixes & Layout Streamlining
- **Anti-Clipping & Expanded Height**:
  - Increased popup window viewport size to $560 \times 400\text{ px}$ (with minimum size $500 \times 360\text{ px}$ and resizability enabled).
  - Adjusted dialog margins to ensure bottom action buttons (`Start download` and `Cancel`) are always 100% visible with plenty of breathing room, eliminating clipping across all Windows display DPI scales (100%, 125%, 150%).
- **Official Cosmic Emblem in Header**:
  - Replaced vector placeholder bars in the dialog header with the official circular cosmic emblem (featuring deep radial gradients and vector download arrow).
- **Streamlined, Modern Layout**:
  - Removed cluttering "Advanced options" and "Add a note" chips from the download popup to strictly align with the clean reference design in `Download_popup.html`.
  - Replaced the large-radius checkbox with a crisp, modern square checkbox with high-contrast checkmark.
  - Category selector and "Save to" folder picker stay aligned in a balanced two-column row.

---

## Release Assets

| File | Description |
| :--- | :--- |
| `CosmoDownloader-Setup-v1.2.4.exe` | **Windows Installer (Recommended)** — Installs Cosmo Downloader (`v1.2.4`), desktop/start shortcuts, `yt-dlp` / `ffmpeg`, and registers browser native messaging hosts. Seamlessly upgrades existing installations. |
| `CosmoDownloader.exe` | Standalone Windows desktop application binary (`v1.2.4`). |
| `CosmoDownloader-Chrome-Edge-Brave-Opera-Extension-v1.2.4.zip` | **All-in-One** unified browser extension package for Google Chrome, Microsoft Edge, Brave, and Opera (`v1.2.4`). |

---

## Quick Install (Windows)

1. Download and run **`CosmoDownloader-Setup-v1.2.4.exe`**.
2. **Chrome / Microsoft Edge / Brave / Opera:**
   - Extract **`CosmoDownloader-Chrome-Edge-Brave-Opera-Extension-v1.2.4.zip`**.
   - Open your browser's extensions management page (`chrome://extensions`, `edge://extensions`, `brave://extensions`, `opera://extensions`).
   - Enable **Developer mode** (top-right toggle).
   - Click **Load unpacked** and select the extracted extension folder.
3. Downloads clicked inside your browser or from video players will automatically open the redesigned Cosmo Downloader popup dialog.
