#!/bin/bash
# OpenFoot Manager - Start Script
# Usage: ./start.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  OpenFoot Manager - Web Version${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""

# Kill existing processes
echo -e "${YELLOW}Stopping existing processes...${NC}"
pkill -f "openfootmanager --web" 2>/dev/null || true
pkill -f "vite" 2>/dev/null || true
sleep 2

# Start Backend
echo -e "${YELLOW}Starting Backend (port 3001)...${NC}"
cd "$SCRIPT_DIR/src-tauri"
setsid ./target/release/openfootmanager --web > /tmp/ofm_backend.log 2>&1 &
BACKEND_PID=$!
sleep 3

# Verify backend is running
if ss -tlnp 2>/dev/null | grep -q ":3001 "; then
    echo -e "${GREEN}✅ Backend started on http://localhost:3001${NC}"
else
    echo -e "${RED}❌ Backend failed to start${NC}"
    echo "Check: /tmp/ofm_backend.log"
    exit 1
fi

# Start Frontend
echo -e "${YELLOW}Starting Frontend...${NC}"
cd "$SCRIPT_DIR"
npm run dev > /tmp/ofm_frontend.log 2>&1 &
FRONTEND_PID=$!
sleep 3

# Verify frontend is running
if ps -p $FRONTEND_PID > /dev/null 2>&1; then
    echo -e "${GREEN}✅ Frontend started on http://localhost:1420${NC}"
else
    echo -e "${RED}❌ Frontend failed to start${NC}"
    echo "Check: /tmp/ofm_frontend.log"
fi

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  Ready! Open http://localhost:1420${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "To stop: pkill -f 'openfootmanager --web' && pkill -f vite"
echo "To view logs:"
echo "  Backend:  tail -f /tmp/ofm_backend.log"
echo "  Frontend: tail -f /tmp/ofm_frontend.log"
