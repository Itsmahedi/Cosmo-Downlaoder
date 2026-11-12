# Cosmo Downloader v1.2.1

A fast, multi-segment desktop download manager for **Windows 10 and Windows 11**, paired with browser extensions for Chrome, Microsoft Edge, Brave, Opera, and Mozilla Firefox.

---

## Key Features

- **Steel Slate Light & Dark UI**: Clean desktop interface with category filtering (`All downloads`, `Programs`, `Video`, `Music`, `Documents`, `Archives`), live search, sortable columns, and light/dark mode toggle.
- **Multi-Segment & Single-Stream Resume Engine**: Parallel byte-range downloads (up to 32 segments), automatic metadata validation (`Content-Length` / `ETag` / `Last-Modified`), SHA-256 verification, global & per-task speed limits, proxy support, collision auto-renaming, and crash/restart recovery.
- **Desktop Ergonomics**:
  - Shortcuts: `Ctrl+N` (Add URL), `Ctrl+A` (Select All), `Delete` (Delete confirmation with optional file removal), `Space` (Pause/Resume selected), `Esc` (Close dialog).
  - Single-instance mutex with CLI URL forwarding and tray minimize/restore support.
  - Persistent settings in SQLite (`%LOCALAPPDATA%\CosmoDownloader\CosmoDownloader\data\cosmodownloader.db`) and logs in `%LOCALAPPDATA%\CosmoDownloader\CosmoDownloader\logs\cosmodownloader.log`.
- **Browser Extension Integration**:
  - Configurable interception toggle, file extension filter, minimum size filter (KB), and excluded-domains list directly in the extension popup.
  - Real browser cookies, `Referer`, and `User-Agent` forwarding via loopback bridge (`127.0.0.1:42424`) and Native Messaging Host (`cosmodownloader-host.exe`) with automatic browser download fallback if the desktop app is unavailable.
  - "Download with Cosmo" context menu and Shadow DOM video download pill.

---

## Windows Installation & In-Place Upgrades

1. Build or run **`CosmoDownloader-Setup-v1.2.1.exe`** (compiled from `installer/cosmodownloader.iss` via Inno Setup 6).
2. Upgrading over a running installation:
   - Setup detects running `CosmoDownloader.exe`, `cosmodownloader-host.exe`, and `cosmodownloader-cli.exe`, sends `--quit` for a clean queue flush (with a 5s wait and `taskkill` fallback), installs cleanly, and automatically relaunches Cosmo Downloader if it was open before the upgrade.
   - User database, queue state, and settings in `%LOCALAPPDATA%\CosmoDownloader` are preserved automatically.
3. Silent install / update:
   ```cmd
   CosmoDownloader-Setup-v1.2.1.exe /VERYSILENT /NORESTART
   ```

---

## Browser Extension Setup

Extension source folders are in `browser-extensions/`:

- **Chrome / Brave / Microsoft Edge / Opera**:
  1. Open `chrome://extensions`, `brave://extensions`, `edge://extensions`, or `opera://extensions`.
  2. Enable **Developer mode**, click **Load unpacked**, and select `browser-extensions/Chrome-Edge-Brave-Opera`.
  3. The included manifest public key keeps extension ID `lbifcbhmdjlhmcfipfdodbdljmcbdanf` consistent across unpacked installs so the installed native messaging host matches automatically.
- **Mozilla Firefox**:
  1. Open `about:debugging#/runtime/this-firefox`.
  2. Click **Load Temporary Add-on...** and select `browser-extensions/Mozilla-Firefox/manifest.json`.

---

## Building & Testing from Source (Windows)

```powershell
# Run workspace unit and integration tests
cargo test --workspace

# Run Clippy strict lint checks
cargo clippy --workspace --all-targets -- -D warnings

# Build release binaries (CosmoDownloader.exe, cosmodownloader-cli.exe, cosmodownloader-host.exe)
cargo build --release --workspace
```

See [docs/TEST_CHECKLIST.md](docs/TEST_CHECKLIST.md) for the complete Windows QA manual verification checklist.

---

## License

Cosmo Downloader is open-source software licensed under the [MIT License](LICENSE).
