# PureRemove Web

Version web de [PureRemove](https://github.com/heiphaistos44-crypto) — suppression de fond d'image par IA (RMBG-1.4, ONNX), hébergée sur VPS.

**Prod : https://pureremove.heiphaistos.org**

## Architecture

- `server/` — Rust **axum** : API + frontend statique, inférence ONNX serveur (CPU)
- `web/` — **React + Vite + Tailwind** : UI identique à la version desktop
- `deploy/` — script déploiement VPS + vhost nginx

### API

| Route | Méthode | Détail |
|-------|---------|--------|
| `/api/process` | POST | multipart `file` + `options` (JSON `{"background":{"type":"Transparent"}}`) → PNG |
| `/api/health` | GET | `{status, model, version}` |

### Protections (accès public)

- Upload max **25 MB**
- Rate-limit **10 req/min/IP** (clé `CF-Connecting-IP`)
- Max **2 inférences simultanées** (sémaphore)
- Timeout requête 60 s
- Écoute `127.0.0.1:3002` uniquement (exposé via nginx)

## Dev local

```bash
# Backend (model.onnx requis : huggingface.co/briaai/RMBG-1.4 → onnx/model.onnx)
cd server
MODEL_PATH=./model.onnx cargo run

# Frontend (proxy /api → :3002)
cd web
npm install && npm run dev
```

## Déploiement VPS (212.227.140.45)

```bash
# 1. Sync du code vers /opt/pureremove (rsync/scp/git)
# 2. Build frontend en local : cd web && npm run build  (dist/ inclus dans le sync)
# 3. Sur le VPS :
cd /opt/pureremove && bash deploy/deploy.sh

# 4. nginx (une fois) :
cp deploy/nginx-pureremove.conf /etc/nginx/sites-available/pureremove
ln -s /etc/nginx/sites-available/pureremove /etc/nginx/sites-enabled/
nginx -t && systemctl reload nginx

# 5. DNS Cloudflare : A pureremove → 212.227.140.45 (proxied)
```

Variables d'env serveur : `PORT` (3002), `MODEL_PATH` (./model.onnx), `STATIC_DIR` (../web/dist).

## Versions

- **1.0.0** (2026-06-11) — portage web initial depuis PureRemove desktop v1.2.1
