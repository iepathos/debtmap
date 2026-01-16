#!/usr/bin/env bash
# Simple dev server for iterating on the dashboard

cd "$(dirname "$0")/.."

PORT=${1:-8080}

echo "Starting dev server at http://localhost:$PORT"
echo "Dashboard at: http://localhost:$PORT/viz-dev/dashboard.html"
echo "Press Ctrl+C to stop"
echo ""

# Try python3 first, fall back to python
if command -v python3 &> /dev/null; then
    python3 -m http.server "$PORT"
elif command -v python &> /dev/null; then
    python -m http.server "$PORT"
else
    echo "Error: Python not found. Install Python or use another HTTP server."
    echo "Alternatives:"
    echo "  - npx serve ."
    echo "  - php -S localhost:$PORT"
    exit 1
fi
