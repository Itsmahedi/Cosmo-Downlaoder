# ✦ Cosmo Downloader v1.12

A blazing-fast, modern multi-threaded download manager with cosmic acceleration, seamless browser integration, and intelligent media interception.

Available for **Windows 10/11** and **Linux (AppImage)**.

---

## ⚡ Key Features

- **Blazing Speed & Multi-Segment Acceleration**: Dynamically segments large downloads into parallel threads to maximize network bandwidth saturation with resume capability.
- **Smart Video Interception & Quality Picker**: Injects an interactive, floating IDM-style download pill onto video players (YouTube, Vimeo, and direct HTML5 videos) with a fast quality selector (1080p, 720p, 480p, 360p, MP3 audio extraction, or best available).
- **Intelligent Mini-Preview Suppression**: Prevents unwanted download buttons on feed previews, hover cards, thumbnails, and search snippets.
- **Persistent Background Integration**: Seamless native messaging host bridge continues intercepting downloads even when the main GUI window is closed.
- **1-Click App Lifecycle Awareness**: Extension popup reflects app health in real-time and enables 1-click desktop app launch directly from the browser.
- **Seamless In-Place Updates**: Update smoothly using the installer or update script without uninstalling the previous version. Running background processes are automatically unlocked, and your download history and settings are 100% preserved.
- **Cosmic Dark Theme**: Crafted with fluent dark aesthetics, live segment progress bars, vector category icons, and speed metrics.

---

## 🚀 Installation & Upgrading

### Windows Installation & In-Place Update

1. Download and run **`CosmoDownloader-Setup-v1.12.exe`**.
2. Follow the setup wizard:
   - The installer automatically detects existing installations, stops running background processes without file-in-use errors, and upgrades in place.
   - User database and settings in `%LOCALAPPDATA%\CosmoDownloader` are preserved (with automatic `.bak` backups).
   - If upgrading from legacy RustGet installations, the installer cleanly migrates your setup.
   - Browser native messaging hosts and desktop shortcuts are automatically registered.

#### Silent / Unattended Upgrades:
```cmd
CosmoDownloader-Setup-v1.12.exe /VERYSILENT /NORESTART /SUPPRESSMSGBOXES
```

### Linux (AppImage)

1. Download the standalone executable:
   ```bash
   chmod +x dist/CosmoDownloader-1.12.0-x86_64.AppImage
   ./dist/CosmoDownloader-1.12.0-x86_64.AppImage
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
