# Cosmo Downloader — Manual & Automated QA Test Checklist (Windows)

Use this checklist to verify releases across the desktop app (`CosmoDownloader.exe`), CLI (`cosmodownloader-cli.exe`), browser extensions (Chrome, Edge, Brave, Opera, Firefox), and Inno Setup installer.

---

## 1. Automated Verification
- [ ] `cargo test --workspace` — all unit and integration tests pass (0 failures)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` — 0 warnings across workspace & manifests
- [ ] Extension manifests validated cleanly as JSON

## 2. Desktop App Features & Download Engine
- [ ] Add download with valid HTTP/HTTPS URL (default & custom save path)
- [ ] Reject empty, invalid, non-HTTP (`javascript:`, `file:`) URLs in Add dialog with inline error (dialog stays open)
- [ ] File collision auto-renames `file.ext` -> `file (1).ext` without clobbering existing files
- [ ] Pause and Resume active multi-segment (8+ connections) download; verify final byte count and SHA-256 hash match
- [ ] Resume single-stream (`Accept-Ranges: none`) download cleanly using `Range: bytes=<offset>-`
- [ ] Force-kill (`Stop` / crash / power loss) mid-download -> relaunch app -> interrupted `"downloading"` tasks recover as `"queued"` and resume cleanly
- [ ] Retry failed download via double-click or context-menu `"Retry"`; inspect hover tooltip on `"Failed"` row for failure reason
- [ ] Delete download via toolbar `Delete`, `Delete` key, or context menu -> confirmation modal appears -> test both keeping file and checking `"Also delete downloaded file(s) from disk"`
- [ ] Large file > 4 GB (`> 4294967296` bytes) sparse preallocation / progress percentage accuracy
- [ ] Disk space precheck rejects targets exceeding free volume space with clear error
- [ ] Categories sidebar (`All`, `Programs`, `Video`, `Music`, `Documents`, `Compressed`/`Archives`), live toolbar search box, column header sort (`File name`, `Size`, `Progress`, `Speed`, `Status`), dark/light mode toggle
- [ ] Keyboard shortcuts: `Ctrl+N` (Add), `Ctrl+A` (Select all), `Delete` (Confirm delete), `Space` (Pause/Resume selected), `Esc` (Close dialog/clear selection)
- [ ] Settings persistence across restart: Max concurrent downloads, Global speed limit, Default connections, Default save folder, Proxy URL, Clipboard monitoring toggle, Desktop notifications toggle, Minimize to tray, Show tray icon, Start with Windows (`--minimized`)

## 3. Windows Integration & Security
- [ ] Single instance: second launch focuses existing window and forwards optional positional URL
- [ ] Special characters, spaces, and non-English Unicode filenames & save paths work cleanly
- [ ] Path traversal (`..\\..`) and Windows reserved names (`CON`, `NUL`, `COM1`, `LPT9`) sanitized safely
- [ ] Bridge server (`127.0.0.1:42424`) rejects cross-origin (`Origin: https://evil.example`, `Origin: null`, `Sec-Fetch-Site: cross-site`), unauthenticated `/show` or `/quit`, and invalid `/download` URLs
- [ ] Tray `"Exit"` and `--quit` cleanly flush database & stop active transfers; tray `"Close button"` hides window only when tray icon is active
- [ ] DPI scaling at 100%, 125%, and 150%, multi-monitor placement, and standard non-admin Windows 10/11 user account

## 4. Browser Extensions & Native Messaging
- [ ] Manifest V3 valid in Chrome, Brave, Edge, and Firefox (`140.0+`), fixed Chromium extension ID `lbifcbhmdjlhmcfipfdodbdljmcbdanf`
- [ ] Native host manifests registered separately per browser under `HKCU\Software\...\NativeMessagingHosts\com.cosmodownloader.downloader` (Chrome, Brave, Chromium, Opera, Edge, Firefox) without overwriting Firefox with Chrome JSON
- [ ] Intercept click download when app is running (focuses/opens popup cleanly), minimized to tray, and closed (native host launches `--popup-file` / app cleanly)
- [ ] Context menu (`Download with Cosmo Downloader`, `Download media with Cosmo Downloader`)
- [ ] Popup controls: on/off interception toggle, file extension filter list, min size (KB) filter, excluded domains list, manual quick URL add, and error banner on bridge failure
- [ ] Real `Cookie` header (`cookies.getAll`), `Referer`, and `User-Agent` forwarded for authenticated downloads
- [ ] Fallback: when desktop app + native host are unavailable, browser download resumes and completes normally; `blob:` / `data:` URLs stay in browser

## 5. Installer, Upgrade, Downgrade & Uninstall
- [ ] Running installer (`CosmoDownloader-Setup-v1.2.exe` or `/SILENT`) while old version is running (visible, minimized to tray, or helper processes active):
  - Prompts clearly `"Cosmo Downloader is running. It will be closed to continue."` (interactive)
  - Sends `--quit` for graceful state save, falls back to forceful termination after timeout if old version refuses
  - Completes with Exit Code `0`, preserves SQLite DB & settings/downloads, and automatically relaunches if previously running
- [ ] Downgrade from newer version prompts confirmation cleanly
- [ ] Apps & Features / Add or Remove Programs displays `Cosmo Downloader`, version `1.2.0`, publisher `Mahedi`, and app icon
- [ ] Uninstall closes running app, removes binaries, shortcuts, `Run` startup entry, and all browser `NativeMessagingHosts` keys, keeps downloaded files, and prompts before deleting `%LOCALAPPDATA%\CosmoDownloader` settings/history
