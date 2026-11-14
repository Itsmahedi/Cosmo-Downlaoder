# Cosmo Downloader v1.2.5

A fast, reliable multi-segment download manager for **Windows 10 and Windows 11**, paired with redesigned browser extensions for Google Chrome, Microsoft Edge, Brave, Opera, and Mozilla Firefox.

---

## What's New in v1.2.5

### 🚀 Standalone Portability & Toolchain Dependency Fix
- **Static Linking of `libunwind`**:
  - Resolved startup crash on target PCs where the application failed with `"The code execution cannot proceed because libunwind.dll was not found."`
  - Configured workspace cargo target flags (`-C target-feature=+crt-static`) so that `CosmoDownloader.exe`, `cosmodownloader-cli.exe`, and `cosmodownloader-host.exe` statically embed the C runtime and LLVM unwinder.
  - Zero dependencies on external toolchain DLLs (`libunwind.dll`, `libc++.dll`, `libwinpthread-1.dll`, `libgcc_s_seh-1.dll`, `libstdc++-6.dll`).
- **Dual-Safeguard Distribution**:
  - Bundled `libunwind.dll` into the installer and portable packages as an extra safety fallback.

### 🎯 Download Info Popup Dialog Behavior Fixes
- **Independent Window Management**:
  - Closing the "Download File Info" popup dialog (via **Cancel**) closes only the dialog. The main window state remains unchanged (stays hidden in tray or closed if it was closed).
  - Pressing **Start download** begins the transfer and closes the dialog without opening or stealing focus from the main window.
- **System Tray Notification**:
  - Displays a clean Windows notification (`"Download started: <file name>"`) when a download starts in the background.
- **New User Preference**:
  - Added setting in the Settings dialog: *"Open the main window when a download starts"* (default: **OFF**).
- **Add URL Dialog Stability**:
  - Opening the download dialog from the main window's "+ Add URL" button retains the main window's exact size and position upon closing.

---

## Release Assets

| File | Description |
| :--- | :--- |
| `CosmoDownloader-Setup-v1.2.5.exe` | **Windows Installer (Recommended)** — Installs Cosmo Downloader (`v1.2.5`), desktop/start shortcuts, `yt-dlp` / `ffmpeg`, and registers browser native messaging hosts. Seamlessly upgrades existing installations. |
| `CosmoDownloader.exe` | Standalone Windows desktop application binary (`v1.2.5`). |
| `CosmoDownloader-Chrome-Edge-Brave-Opera-Extension-v1.2.5.zip` | **All-in-One** unified browser extension package for Google Chrome, Microsoft Edge, Brave, and Opera (`v1.2.5`). |

---

## Quick Install (Windows)

1. Download and run **`CosmoDownloader-Setup-v1.2.5.exe`**.
2. **Chrome / Microsoft Edge / Brave / Opera:**
   - Extract **`CosmoDownloader-Chrome-Edge-Brave-Opera-Extension-v1.2.5.zip`**.
   - Open your browser's extensions management page (`chrome://extensions`, `edge://extensions`, `brave://extensions`, `opera://extensions`).
   - Enable **Developer mode** (top-right toggle).
   - Click **Load unpacked** and select the extracted extension folder.
3. Downloads clicked inside your browser or from video players will automatically open the redesigned Cosmo Downloader popup dialog.
