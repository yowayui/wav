#!/usr/bin/env bash
set -euo pipefail

echo "==> Building WAV (release)..."
cargo build --release

BIN_DIR="${HOME}/.local/bin"
APP_DIR="${HOME}/.local/share/applications"

mkdir -p "${BIN_DIR}" "${APP_DIR}"

echo "==> Installing binary to ${BIN_DIR}/wav..."
cp target/release/wav "${BIN_DIR}/wav"
chmod +x "${BIN_DIR}/wav"

echo "==> Installing desktop entry..."
cat > "${APP_DIR}/wav.desktop" << 'EOF'
[Desktop Entry]
Type=Application
Name=WAV
Comment=Minimalist music player
Exec=wav
Terminal=true
Categories=AudioVideo;Audio;Player;
Keywords=music;player;audio;tui;
EOF

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "${APP_DIR}" 2>/dev/null || true
fi

echo "==> Done! Run 'wav' from anywhere."
