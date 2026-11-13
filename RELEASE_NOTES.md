# Cosmo Downloader v1.2.2

A fast, reliable multi-segment download manager for **Windows 10 and Windows 11**, paired with redesigned browser extensions for Google Chrome, Microsoft Edge, Brave, Opera, and Mozilla Firefox.

---

## What's New in v1.2.2

### Browser Extensions (Redesigned & AMO-Compliant)
- **Desktop-Aligned UI Design**: Completely overhauled the browser extension popup to match the Cosmo Downloader desktop software design:
  - **Steel Slate Palette**: Styled with official `#8fa6c4` (dark mode) and `#44546a` (light mode) accents.
  - **Adaptive Light/Dark Theme**: Full automatic theme switching via `prefers-color-scheme`.
  - **Expanded Dimensions**: Refined card layout at 340px width with 10px rounded borders.
  - **Connection Banner**: Clear horizontal status indicator showing live connection state to the Cosmo Downloader desktop engine.
  - **Hero Counter**: Centered 32px bold counter displaying intercepted download totals.
  - **Solid Primary Download Button**: Quick URL entry field with a solid primary accent send button.
- **Mozilla Firefox (AMO) Manifest V3 Compliance**:
  - Enforced `browser_specific_settings` with `gecko.id: rustget@downloader.addon`, `strict_min_version: "140.0"`, and Android-specific `strict_min_version: "142.0"`.
  - Declared `data_collection_permissions: { "required": ["none"] }` ensuring 0 errors and 0 warnings during addons.mozilla.org (AMO) automated validation.
  - Standardized POSIX forward slash paths (`/`) in all archives to eliminate linter extraction warnings.

### Build & Version Hygiene
- Bumped workspace version to `1.2.2` across `Cargo.toml`, Windows PE `.rc` resource files, Inno Setup installer, and extension manifests.
- Verified all 40 automated unit, audit, and integration tests pass with zero failures.

---

## Release Assets

| File | Description |
| :--- | :--- |
| `CosmoDownloader-Setup-v1.2.2.exe` | **Windows Installer (Recommended)** — Installs Cosmo Downloader (`v1.2.2`), shortcuts, `yt-dlp` / `ffmpeg`, and registers browser native messaging hosts. Seamlessly upgrades running versions. |
| `CosmoDownloader.exe` | Standalone Windows desktop application binary (`v1.2.2`). |
| `CosmoDownloader-Chrome-Extension-v1.2.2.zip` | Browser extension package for Google Chrome, Microsoft Edge, Brave, and Opera (`v1.2.2`). |

---

## Quick Install (Windows)

1. Download and run **`CosmoDownloader-Setup-v1.2.2.exe`**.
2. **Chrome / Edge / Brave / Opera:**
   - Extract `CosmoDownloader-Chrome-Extension-v1.2.2.zip`.
   - Go to `chrome://extensions` (or `edge://extensions` / `brave://extensions`), enable **Developer mode**, and select **Load unpacked**.
3. **Mozilla Firefox:**
   - Available via Mozilla Add-ons (AMO).
