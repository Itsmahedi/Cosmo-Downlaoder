# Changelog

All notable changes to Cosmo Downloader will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.2.1] - 2026-11-13

### Fixed
- **Browser Extension Video Download Button Icon:** Replaced the legacy stacked-bars icon on the video download button with a standard downward arrow into tray SVG icon with `stroke="currentColor"` matching the button text color in both normal and sent states.
- **Repository Configuration:** Standardized all project URLs, bug trackers, and release targets exclusively to `Itsmahedi/Cosmo-Downloader`.

## [1.2.0] - 2026-11-12

### Security
- Enforced strict `http://` / `https://` URL validation before launching desktop/CLI/bridge downloads, sanitizing reserved Windows device names (`CON`, `NUL`, `COM1`, `LPT1`), path traversal sequences, and overlong filenames.
- Added collision-safe auto-renaming (`file (1).ext`, `file (2).ext`) so new downloads never overwrite existing files on disk unless resuming their active `.cosmo.part` state.
- Hardened the local HTTP bridge (`127.0.0.1:42424`) with strict `Origin` / `Sec-Fetch-Site: cross-site` CSRF blocking, `X-Cosmo-Token` authentication for `/show` and `/quit`, and redacted credentials/query strings in logs.
- Fixed browser extension manifest permissions (removed unused `scripting` and invalid wildcard hosts) and pinned deterministic Chromium extension ID `lbifcbhmdjlhmcfipfdodbdljmcbdanf`.

### Fixed
- **Download Engine (`rustget-core`)**: Propagated multi-segment worker errors, verified partial segment byte completeness before assembly, validated `Content-Length`/`ETag`/`Last-Modified` on resume, added single-stream `Range` resume, recovered interrupted `'downloading'` tasks cleanly on startup, checked free disk space before allocation, and wired per-task speed limits and proxy settings end-to-end.
- **Desktop App (`CosmoDownloader.exe`)**: Added single-instance URL argument handoff, non-blocking `--quit` / OS `WM_CLOSE` graceful queue save & shutdown (distinguishing real quits and Tray Exit from window-X close-to-tray), fixed `--popup-file` zombie instance handling, persisted all settings across restarts (concurrent limit, speed cap, category directories, proxy, connections, notifications), and reduced idle repaint/clipboard polling overhead.
- **Desktop UX**: Added toolbar live search + clear, clickable column sorting (`Added`, `Name`, `Size`, `Progress`, `Speed`, `Status`), inline URL errors, per-task connection/speed/SHA-256 Add dialog fields, failed-row tooltips & Retry action, delete confirmation modal (with optional disk file removal), keyboard shortcuts (`Ctrl+N`, `Ctrl+A`, `Delete`, `Space`, `Esc`), and injection-safe Windows toast notifications.
- **Browser Extension & Native Host**: Fixed `shouldIntercept()` trailing `return true` bug; wired configurable file-type filters, minimum size KB filter, and domain exclusion list in the popup; forwarded real `chrome.cookies` + `User-Agent` + `Referer`; restored safe browser fallback (`resume()`) on bridge/native failure; removed fabricated video file sizes; registered Chrome, Brave, Chromium, Opera, Edge, and Firefox native host manifests separately without cross-browser overwrites; and fixed AppData discovery.
- **Windows Installer (`installer/cosmodownloader.iss`)**: Replaced broken `RenameFile` lock checks with mutex + `tasklist` detection, added `--quit` graceful shutdown + 5s timeout + `taskkill` fallback, removed duplicate `Run` key, added Brave/Chromium/Opera native host registry cleanup on uninstall, added `UninstallDisplayIcon`, preserved `%LOCALAPPDATA%\CosmoDownloader` across upgrades, and restored automatic post-update relaunch for both interactive and `/SILENT` installs.
- **Build & Version Hygiene**: Extracted clean `lib.rs` library targets to eliminate Cargo warning noise, resolved all Clippy lints (`-D warnings`), and unified version metadata to `1.2.0` across workspace crates, PE `VERSIONINFO` resources, installer, and browser extensions.

### Added
- `crates/core/tests/audit_tests.rs` integration suite testing URL/filename sanitization, collision auto-rename, truncated segment error handling, single-stream resume + ETag mismatch reset, interrupted queue recovery, and bridge CSRF/token guards.
- `docs/TEST_CHECKLIST.md` Windows QA manual test checklist.

## [1.12.0] - 2026-11-11

### Added
- **Seamless In-Place Windows Upgrades (`installer/cosmodownloader.iss`):**
  - Added Inno Setup `PrepareToInstall` hook automatically detecting file locks on running `CosmoDownloader.exe`, `cosmodownloader-host.exe`, and `cosmodownloader-cli.exe`.
  - Added Restart Manager integration with graceful shutdown and verified fallback process termination so setup updates existing installs without file-in-use errors.
  - Added automatic detection and silent uninstallation (`/VERYSILENT /NORESTART`) of legacy RustGet installations from HKCU and HKLM.
  - Added semver downgrade warning dialog preventing accidental configuration rollback from newer versions.
  - Added pre-install automated backup of user SQLite database (`cosmodownloader.db.bak`), preserving all active and historical download records.
  - Migrated native messaging host registry setup directly to `[Registry]` with `uninsdeletekey`.
- **Extension Disconnection Recovery (`background.js`):**
  - Added exponential backoff reconnection engine (1s up to 30s) reconnecting to the native messaging host automatically after an app update or host restart.
- **Unified In-Place Update & Deployment Tooling (`scripts/deploy_binaries.ps1`, `scripts/update-app.ps1`):**
  - Consolidated into a single script supporting `-Mode Dev|Release` with `Set-StrictMode -Version Latest`.
  - Removed all hardcoded user paths in favor of dynamic environment (`%LOCALAPPDATA%`) and registry detection.
  - Implemented atomic rollback protection renaming active executables to `.old` before overwrite and rolling back on failure.
  - Added optional `-InstallerUrl` with SHA256 integrity verification (`-ExpectedSha256`).
- **Automated Regression Testing & Quality Checks:**
  - Added end-to-end Playwright regression test suite (`tests/test_dropdown.test.mjs`) verifying download button rendering, ARIA state, toggle open/close, Escape key dismiss, outside click dismiss, auto-flip positioning, and message payload verification for all 6 quality tiers.
  - Added `stylelint` validation preventing CSS syntax errors or orphan declarations.
  - Added `scripts/sync-extensions.mjs` ensuring 100% identical shared code between Chrome and Firefox extensions.

### Fixed
- **Video Downloader Quality Dropdown Interception (`content.css`, `content.js`):**
  - Repaired CSS syntax error in `content.css` caused by an orphaned closing brace that led browser engines to drop the `.open` display rule.
  - Added `pointer-events: none` on button and item sub-elements, ensuring reliable click targets.
  - Attached capture-phase event listeners to prevent host video players (such as YouTube's `#movie_player`) from swallowing clicks or toggling playback when clicking the download pill.
  - Added `flip-up` class positioning the menu above the button when overflowing the bottom viewport.
  - Bound `Escape` key and outside clicks (`composedPath()`) to cleanly dismiss the menu.

## [1.11.0] - 2026-11-11

### Added
- **Preview Suppression on Video Mini-Previews (`content.js`):**
  - Host-specific player targeting (`isMiniPreview(videoEl)`) preventing download button injection into YouTube home feed hover previews, search inline previews (`#inline-preview-player`, `ytd-video-preview`, `ytd-inline-preview-renderer`), sidebar recommendations, and thumbnails.
  - Attached exclusively to main player `#movie_player` on `/watch` and active shorts reel on `/shorts`.
  - Added lenient 320x180 fallback dimension check strictly to non-YouTube hosts.
  - Added dynamic mutation cleanup observing removed elements and SPA navigation changes (`yt-navigate-finish`), preventing orphaned or duplicate buttons.
- **App Lifecycle Awareness & 1-Click Launch:**
  - Browser extensions now detect whether Cosmo Downloader desktop app is running via asynchronous loopback probe to `http://127.0.0.1:42424/health` with a 500ms `AbortController` timeout.
  - Extension popup provides a 3-state UI: Green (Connected), Amber (App closed, with a prominent "▶ Launch App" button), Red (Native host missing or unregistered).
  - Native host supports `HostAction::Launch` (`find_app_exe()` without accepting paths from the extension message, spawning detached process with `CREATE_NO_WINDOW`).
  - Added single-instance mutex guard in Cosmo Downloader app so repeated launches safely refocus the existing instance without opening redundant processes.
  - Hardened loopback bridge security: restricted to loopback address, Origin header validation rejecting external web origins, and required authentication token for `/show`.
- **Desktop UI Overhaul (`crates/app`):**
  - Unified design tokens (`#0E0F1D`, `#14162A`, `#1A1C36`, `#2D3056`, `#00F0FF`, `#A855F7`, `#22C55E`, `#F59E0B`, `#EF4444`) with WCAG AA compliant text contrast.
  - `CategoryFilter` enum (`All`, `Video`, `Music`, `Documents`, `Compressed`, `Programs`, `Other`) with single unified `categorize(filename, mime)` function and unit tests.
  - Interactive sidebar categories and status tabs with real-time cached count badges, persisting filter states across restarts.
  - Redesigned download cards: truncated filename, vector category icons, percentage, downloaded/total size, EMA-smoothed speed, and ETA.
  - Segmented progress bar (emerald for completed segments, cyan for active, slate border for pending).
  - Action buttons: "Open" (`shell_open_file`), "Folder" (`shell_open_folder_and_select`), "Pause", "Resume".
  - Click-to-confirm download deletion (two-step confirmation to prevent accidental loss).
  - Custom vector icon rendering via `egui::Painter` guaranteeing crisp rendering without missing unicode font glyphs.
  - Centered empty state with cosmic star icon and "+ Add Download URL" shortcut.
  - Top toolbar with uniform button heights (30px) and aggregate live speed HUD readout.
- **Extension Popup & Content Styling:**
  - Dark cosmic card styling using solid gradient and borders (removed unsupported `backdrop-filter: blur`).
  - Strict URL validation using `new URL()` accepting only `http:` and `https:`.
  - Compact pill button on video players with subtle cyan border dropdown quality picker.

- **Packaging & Portability:**
  - Added standalone Linux AppImage packaging (`dist/CosmoDownloader-1.11.0-x86_64.AppImage`) bundling `CosmoDownloader`, `cosmodownloader-cli`, `cosmodownloader-host`, and `libxdo.so.4`.
  - Added automated AppImage build script `scripts/build-appimage.sh`.

## [1.10.0] - 2026-11-04
- Initial release of Cosmo Downloader with multi-segment downloads, streaming site support, and browser extension integration.
