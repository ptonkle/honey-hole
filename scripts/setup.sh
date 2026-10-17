#!/usr/bin/env bash
# One-time setup for the browser check: a Python virtual environment with
# Playwright, and the three browser engines it drives.
set -euo pipefail
cd "$(dirname "$0")/.."
python3 -m venv .venv
.venv/bin/pip install --quiet --upgrade pip
.venv/bin/pip install --quiet "playwright==1.63.0"
.venv/bin/python -m playwright install ${PLAYWRIGHT_WITH_DEPS:+--with-deps} chromium firefox webkit
echo "Browser check ready: .venv/bin/python tests/browser/check.py"
