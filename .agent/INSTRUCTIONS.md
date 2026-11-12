# 🤖 AI Agent Operational Instructions & Guidelines

These instructions govern any AI coding agent operating in the `cosmo_downloader` workspace.

---

## 🛑 STRICT RULE #1: DO NOT SEARCH EVERY FILE

> [!CAUTION]
> **DO NOT search through each and every file in this repository.**
> **DO NOT run unbounded recursive searches (`find .`, `grep -r`, broad directory scans).**

### Why?
- The project contains large build directories (`target/`), installer builds (`installer/dist/`), compiled Windows executables (`*.exe`, `*.dll`), pre-packaged extensions (`*.zip`, `*.xpi`), and Git history (`.git/`).
- Recursive searches waste hundreds of thousands of tokens, slow down tool calls, hit output limits, and degrade context accuracy.

### What To Do Instead:
1. **Look at [`.agent/MINDMAP.md`](file:///home/tokit/Projects/cosmo_downloader/.agent/MINDMAP.md) first!**
   - The file lookup matrix tells you the exact crate, file name, and function name for almost every task.
2. **Targeted Reading:** Use `read_file` with explicit start and end line numbers on the specific file you identified from the mind map.
3. **Focused Symbol Lookup:** If you need to find an unmapped symbol, limit `grep` to a single directory (e.g., `grep -rn "pattern" crates/core/src/`).

---

## 🚫 Restricted & Ignored Paths

Never scan, search, or read the following paths:
- `target/**` — Rust build artifacts, intermediate object files, cached dependencies.
- `installer/dist/**` — Output installer `.exe` files.
- `.git/**` — Git metadata and history objects.
- `*.exe`, `*.dll`, `*.ico`, `*.rgba`, `*.zip`, `*.xpi` — Binary files.

---

## 🛠️ Architecture & Technology Stack Reference

When writing or modifying code in this repository, keep in mind:

1. **Rust Workspace (`2021` Edition):**
   - `rustget_core` (`crates/core`): Pure download engine, SQLite persistence, rate limiting, and HTTP bridge server.
   - `cosmodownloader-app` (`crates/app`): Desktop GUI using `egui` and `eframe` (v0.29), tray icon, autostart via Windows Registry, and topmost popup dialog.
   - `cosmodownloader-host` (`crates/native-host`): Native messaging bridge for Chrome and Firefox extensions.
   - `cosmodownloader-cli` (`crates/cli`): Command line utility using `clap`.

2. **Windows Platform Considerations:**
   - The primary deployment target is Windows 10 / 11.
   - The app uses Windows-specific APIs (`#[cfg(windows)]`, `CREATE_NO_WINDOW`, `std::os::windows::process::CommandExt`, `ShellExecuteExW`, Windows Registry for autostart and native messaging manifests).
   - Be careful not to break Windows-specific features when refactoring.

3. **Native Messaging Protocol Rules:**
   - In `crates/native-host/src/main.rs`, standard output (`stdout`) is strictly reserved for Native Messaging frames (4-byte length prefix + JSON payload).
   - Any logging or debug prints in the host MUST go to `stderr` (e.g. `tracing_subscriber::fmt().with_writer(io::stderr)`). Printing to `stdout` will break browser extension communication.

4. **Local Bridge Port:**
   - The internal HTTP bridge operates on `127.0.0.1:42424`. If modifying this port, it must be changed consistently across `crates/core/src/server.rs` and `crates/native-host/src/main.rs`.

5. GitHub Releases and Uploads
Development happens in the private working repo. NEVER create releases, tags, or upload assets to the private working repo.
ALL releases and uploads go ONLY to the public repo: Itsmahedi/Cosmo-Downlaoder with appropriate comprehensive but simple also good looking release notes
Always pass --repo Itsmahedi/Cosmo-Downlaoder explicitly to every gh release command. gh defaults to the repo of the current directory, which is the private one.
Before running any upload, verify the target repo is the public one.
Upload the updated Windows .exe and the updated browser extension (.zip) with each release.
Keep the public repo focused on distributing the executable and release files only. Do not push source code or private project files to it.
Example:

gh release create vX.Y.Z \
  --repo Itsmahedi/Cosmo-Downlaoder \
  --title "Cosmo Downloader vX.Y.Z" \
  --notes-file RELEASE_NOTES.md \
  path/to/CosmoDownloader.exe path/to/extension.zip

6. Version Management
Update the version with every release.
Keep the version consistent across the application (Cargo.toml), the browser extension manifest, and the GitHub release tag.
Include release notes and the version number in each release.
Release Targets
Windows: .exe
Browser: updated extension (with no mention of Linux anywhere in its text)
