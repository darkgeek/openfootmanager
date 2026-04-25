# Web Version Guide

This document explains how to run OpenFoot Manager as a web application in your browser.

## Prerequisites

1. **Rust toolchain** - You need Rust installed to build the backend server
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Node.js** - Required for the frontend build
   ```bash
   # Using nvm (recommended)
   curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
   nvm install 18
   nvm use 18
   ```

## Building and Running

### Step 1: Build the Backend Server

```bash
cd src-tauri
cargo build --release --features web
```

This will create a `openfootmanager` binary that runs as an HTTP server instead of a desktop app.

### Step 2: Start the Backend Server

```bash
# From src-tauri directory
./target/release/openfootmanager --web --port 3001
```

Or use the npm script (after building once):
```bash
npm run server
```

### Step 3: Build and Run the Frontend

```bash
# In the project root
npm install
npm run build:web
npm run preview
```

Or for development with hot reload:
```bash
npm run dev:web
```

Then open http://localhost:5173 in your browser.

## Architecture

The web version uses a client-server architecture:

```
┌─────────────────────────────────────────────────────────┐
│                    Your Browser                          │
│  ┌─────────────────────────────────────────────────┐   │
│  │         React Frontend (Vite dev server)        │   │
│  │                                                   │   │
│  │   All API calls go through /api/* endpoints     │   │
│  └─────────────────────────────────────────────────┘   │
│                        │ HTTP                          │
└────────────────────────┼────────────────────────────────┘
                         │
┌────────────────────────┼────────────────────────────────┐
│               Rust HTTP Server (Axum)                    │
│                                                         │
│  ┌─────────────────────────────────────────────────┐   │
│  │              HTTP API Handlers                  │   │
│  │  (POST /api/start_new_game, etc.)              │   │
│  └─────────────────────────────────────────────────┘   │
│                        │                               │
│  ┌─────────────────────────────────────────────────┐   │
│  │          Game Logic (ofm_core, domain)          │   │
│  └─────────────────────────────────────────────────┘   │
│                        │                               │
│  ┌─────────────────────────────────────────────────┐   │
│  │         SQLite Database (saves)                 │   │
│  └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

## Troubleshooting

### Backend won't compile

Make sure you have all Rust dependencies:
```bash
rustup update
cargo fetch
```

### Frontend shows "Failed to connect to API"

1. Make sure the backend server is running
2. Check that it's running on port 3001
3. The Vite dev server should proxy API requests automatically

### Database issues

The web version stores saves in:
- Linux/macOS: `~/.local/share/openfootmanager/saves/`
- Windows: `%APPDATA%\openfootmanager\saves\`

## Production Deployment

For a production web deployment, you would:

1. Build the frontend: `npm run build:web`
2. Configure the HTTP server to serve the `dist-web` folder
3. Use a reverse proxy (nginx, Caddy) for HTTPS

Note: This is a prototype implementation. For production use, you may need additional work on authentication, session management, and security hardening.
