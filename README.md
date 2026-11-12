# Cosmo Downloader v1.13.0

A modern, high-speed multi-threaded download manager with a calm, professional steel design system, seamless browser integration, and intelligent video download capability.

Available for **Windows 10/11** and **Linux (AppImage)**.

---

## ⚡ Key Features

- **New Unified Design System**: Minimalist, professional steel/gray aesthetic matching `Cosmo downloader NEWWW.html`, with native light & dark themes, local Geist typography, and one-color vector line icons.
- **Blazing Speed & Multi-Segment Acceleration**: Dynamically segments large downloads into parallel threads to maximize network bandwidth saturation with resume capability.
- **Columnar Download Table**: Responsive table showing checkbox selection, category icon, filename, size, progress bar with percentage, transfer speed, and status.
- **Smart Video Interception & Shadow DOM Quality Picker**: Injects an isolated Shadow DOM download button on video players with a 176px quality menu (1080p, 720p, 480p, 360p, Audio only). Host website CSS cannot break it.
- **340px Browser Extension Popup**: Clean popup with connection indicator, download interception toggle, intercepted count, and fast URL submission.
- **Persistent Background Integration**: Native messaging host bridge continues intercepting downloads even when the main GUI window is closed.
- **1-Click App Lifecycle Awareness**: Extension popup reflects app health in real-time and enables instant URL sending to the engine.

---

## 🚀 Installation & Upgrading

### Windows Installation & In-Place Update

1. Download and run **`CosmoDownloader-Setup-v1.13.exe`** (or build from source with `cargo build --release`).
2. Follow the setup wizard:
   - The installer automatically detects existing installations, stops running background processes without file-in-use errors, and upgrades in place.
   - User database and settings in `%LOCALAPPDATA%\CosmoDownloader` are preserved (with automatic `.bak` backups).
   - Browser native messaging hosts and desktop shortcuts are automatically registered.

#### Silent / Unattended Upgrades:
```cmd
CosmoDownloader-Setup-v1.13.exe /VERYSILENT /NORESTART /SUPPRESSMSGBOXES
```

### Linux (AppImage)

1. Download the standalone executable:
   ```bash
   chmod +x dist/CosmoDownloader-1.13.0-x86_64.AppImage
   ./dist/CosmoDownloader-1.13.0-x86_64.AppImage
   ```
2. To build the AppImage from source on Linux:
   ```bash
   ./scripts/build-appimage.sh
   ```

---

## 🧩 Browser Extension Setup

The extension connects your browser directly to Cosmo Downloader for automatic download interception and floating video download buttons.

Extension source files are located in **`browser-extensions/`**:

### Google Chrome / Brave / Microsoft Edge / Opera

1. Open your browser's extension manager:
   - **Chrome**: `chrome://extensions`
   - **Brave**: `brave://extensions`
   - **Edge**: `edge://extensions`
   - **Opera**: `opera://extensions`
2. Toggle **Developer mode** on (top-right corner).
3. Click **Load unpacked** and select the folder:  
   `browser-extensions/Chrome-Edge-Brave-Opera`

### Mozilla Firefox

1. Open Firefox and navigate to `about:debugging#/runtime/this-firefox` (or `about:addons`).
2. Click **Load Temporary Add-on...**
3. Select any file (e.g. `manifest.json`) in:  
   `browser-extensions/Mozilla-Firefox/manifest.json`

---

## 🛠️ Developer & Power-User Scripts

- **In-Place Update / Binary Deployment (PowerShell)**:
  ```powershell
  # Deploy release binaries into local installation without uninstalling
  .\scripts\deploy_binaries.ps1 -Mode Release

  # Upgrade from a downloaded installer with SHA-256 verification
  .\scripts\update-app.ps1 -InstallerUrl "https://.../CosmoDownloader-Setup-v1.12.exe" -ExpectedSha256 "<HASH>"
  ```

- **Synchronize Browser Extensions**:
  ```bash
  npm run sync-extensions
  ```

- **Run Automated Tests**:
  ```bash
  # Rust core and CLI tests
  cargo test --workspace

  # Browser extension and Playwright end-to-end regression tests
  npm test

  # CSS stylelint check
  npm run lint:css
  ```

---

## 📄 License

Cosmo Downloader is open source software licensed under the [MIT License](LICENSE).
