#!/bin/bash
# deploy.sh — Déploiement PureRemove Web sur VPS Debian 13
set -euo pipefail

APP_DIR="/opt/pureremove"
LOG() { echo "[$(date -Iseconds)] [INFO] $*"; }

cd "$APP_DIR"

# ── 1. Rust toolchain ──────────────────────────────────────────────
if ! command -v cargo >/dev/null 2>&1; then
    LOG "Installation rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi
source "$HOME/.cargo/env" 2>/dev/null || true

# ── 2. Modèle RMBG-1.4 ─────────────────────────────────────────────
if [ ! -f "$APP_DIR/model.onnx" ]; then
    LOG "Téléchargement model.onnx (~44 MB)..."
    curl -L "https://huggingface.co/briaai/RMBG-1.4/resolve/main/onnx/model.onnx" \
         -o "$APP_DIR/model.onnx"
fi

# ── 3. Build frontend React ────────────────────────────────────────
LOG "Build frontend React..."
cd "$APP_DIR/web"
npm ci --prefer-offline
npm run build
LOG "Frontend buildé dans $APP_DIR/web/dist"

# ── 4. Build serveur Rust ──────────────────────────────────────────
LOG "cargo build --release..."
cd "$APP_DIR/server"
cargo build --release
LOG "Binaire : $APP_DIR/server/target/release/pureremove-server"

# ── 5. PM2 ─────────────────────────────────────────────────────────
cd "$APP_DIR"
if pm2 describe pureremove >/dev/null 2>&1; then
    LOG "Restart PM2 pureremove..."
    pm2 restart pureremove --update-env
else
    LOG "Création process PM2 pureremove (port 3002)..."
    MODEL_PATH="$APP_DIR/model.onnx" \
    STATIC_DIR="$APP_DIR/web/dist" \
    PORT=3002 \
    pm2 start "$APP_DIR/server/target/release/pureremove-server" --name pureremove
    pm2 save
fi

# ── 6. Vérification ────────────────────────────────────────────────
sleep 2
curl -fsS http://127.0.0.1:3002/api/health && echo "" && LOG "Déploiement OK ✅"