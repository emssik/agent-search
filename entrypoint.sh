#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

if [ -f "$SCRIPT_DIR/.env" ]; then
    set -a
    source "$SCRIPT_DIR/.env"
    set +a
fi

# Activate venv if present
if [ -f "$SCRIPT_DIR/.venv/bin/activate" ]; then
    source "$SCRIPT_DIR/.venv/bin/activate"
fi

CORPUS="${AGENT_CORPUS:-/corpus}"
LANGUAGE="${STEMMER_LANGUAGE:-pl}"

if [ ! -f "$CORPUS/.agent-search-index/manifest.json" ]; then
    echo "==> Building index for $CORPUS (language: $LANGUAGE)..."
    agent-search index -c "$CORPUS" --language "$LANGUAGE"
else
    echo "==> Updating index for $CORPUS..."
    agent-search index -c "$CORPUS"
fi

echo "==> Starting server on :8080"
exec uvicorn web.server:app --host 0.0.0.0 --port 8080
