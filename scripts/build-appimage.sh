#!/usr/bin/env bash
set -euo pipefail

# Cosmo Downloader - Linux AppImage Build Script
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${ROOT_DIR}"

APP_NAME="CosmoDownloader"
VERSION="1.12.0"
ARCH="${ARCH:-x86_64}"
APPDIR="${ROOT_DIR}/build/AppDir"
DIST_DIR="${ROOT_DIR}/dist"
APPIMAGE_NAME="${APP_NAME}-${VERSION}-${ARCH}.AppImage"

echo "=== Building Cosmo Downloader AppImage v${VERSION} (${ARCH}) ==="

# 1. Ensure required library libxdo is available
LIBXDO_PATH=""
for search in "${HOME}/.local/lib" "/usr/lib" "/usr/lib64" "/usr/lib/x86_64-linux-gnu"; do
    if [ -f "${search}/libxdo.so.4" ] || [ -f "${search}/libxdo.so" ]; then
        LIBXDO_PATH="${search}"
        break
    fi
done

if [ -z "${LIBXDO_PATH}" ]; then
    echo "Notice: libxdo not found in system paths. Attempting to fetch xdotool library package..."
    TEMP_PKG="/tmp/xdotool_fetch.pkg.tar.zst"
    curl -sSL "https://geo.mirror.pkgbuild.com/extra/os/x86_64/xdotool-4.20260303.1-1-x86_64.pkg.tar.zst" -o "${TEMP_PKG}"
    mkdir -p "${HOME}/.local/lib" "${HOME}/.local/include"
    tar -xf "${TEMP_PKG}" -C "${HOME}/.local/" --strip-components=1 usr/lib usr/include
    rm -f "${TEMP_PKG}"
    LIBXDO_PATH="${HOME}/.local/lib"
fi

export LIBRARY_PATH="${LIBXDO_PATH}:${LIBRARY_PATH:-}"
export LD_LIBRARY_PATH="${LIBXDO_PATH}:${LD_LIBRARY_PATH:-}"
export PKG_CONFIG_PATH="${LIBXDO_PATH}/pkgconfig:${PKG_CONFIG_PATH:-}"

# 2. Build release binaries
echo "Compiling release binaries..."
cargo build --release --workspace

# 3. Prepare AppDir layout
echo "Preparing AppDir layout at ${APPDIR}..."
rm -rf "${APPDIR}"
mkdir -p "${APPDIR}/usr/bin"
mkdir -p "${APPDIR}/usr/lib"
mkdir -p "${APPDIR}/usr/share/applications"
mkdir -p "${APPDIR}/usr/share/icons/hicolor/128x128/apps"
mkdir -p "${DIST_DIR}"

# Copy binaries
cp -p "${ROOT_DIR}/target/release/CosmoDownloader" "${APPDIR}/usr/bin/"
cp -p "${ROOT_DIR}/target/release/cosmodownloader-cli" "${APPDIR}/usr/bin/"
cp -p "${ROOT_DIR}/target/release/cosmodownloader-host" "${APPDIR}/usr/bin/"

# Copy bundled shared libraries
if [ -d "${LIBXDO_PATH}" ]; then
    cp -P "${LIBXDO_PATH}"/libxdo.so* "${APPDIR}/usr/lib/" 2>/dev/null || true
fi

# Copy icons
ICON_SRC="${ROOT_DIR}/browser-extensions/icons/icon-128.png"
if [ -f "${ICON_SRC}" ]; then
    cp "${ICON_SRC}" "${APPDIR}/cosmodownloader.png"
    cp "${ICON_SRC}" "${APPDIR}/.DirIcon"
    cp "${ICON_SRC}" "${APPDIR}/usr/share/icons/hicolor/128x128/apps/cosmodownloader.png"
fi

# Create desktop entry
cat << 'EOF' > "${APPDIR}/cosmodownloader.desktop"
[Desktop Entry]
Name=Cosmo Downloader
Comment=High-speed multi-threaded download manager with cosmic acceleration engine
Exec=CosmoDownloader %U
Icon=cosmodownloader
Terminal=false
Type=Application
Categories=Network;FileTransfer;
StartupWMClass=CosmoDownloader
MimeType=x-scheme-handler/cosmodownloader;
EOF
cp "${APPDIR}/cosmodownloader.desktop" "${APPDIR}/usr/share/applications/"

# Create AppRun launcher
cat << 'EOF' > "${APPDIR}/AppRun"
#!/bin/sh
SELF=$(readlink -f "$0")
HERE=${SELF%/*}
export PATH="${HERE}/usr/bin:${PATH}"
export LD_LIBRARY_PATH="${HERE}/usr/lib:${LD_LIBRARY_PATH}"
exec "${HERE}/usr/bin/CosmoDownloader" "$@"
EOF
chmod +x "${APPDIR}/AppRun"

# 4. Locate or download appimagetool
APPIMAGETOOL=""
if command -v appimagetool >/dev/null 2>&1; then
    APPIMAGETOOL="appimagetool"
elif [ -x "/tmp/appimagetool-extracted/AppRun" ]; then
    APPIMAGETOOL="/tmp/appimagetool-extracted/AppRun"
else
    echo "Downloading appimagetool..."
    curl -sSL "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage" -o /tmp/appimagetool
    chmod +x /tmp/appimagetool
    (cd /tmp && ./appimagetool --appimage-extract && rm -rf appimagetool-extracted && mv squashfs-root appimagetool-extracted)
    APPIMAGETOOL="/tmp/appimagetool-extracted/AppRun"
fi

# 5. Build AppImage
echo "Packaging AppImage..."
ARCH="${ARCH}" "${APPIMAGETOOL}" "${APPDIR}" "${DIST_DIR}/${APPIMAGE_NAME}"
chmod +x "${DIST_DIR}/${APPIMAGE_NAME}"

echo ""
echo "=== Build Complete ==="
echo "AppImage created successfully:"
ls -lh "${DIST_DIR}/${APPIMAGE_NAME}"
