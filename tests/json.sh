#!/usr/bin/env zsh

source "${0:A:h}/config.sh"

echo '[{"label": "Copy text", "text": "Hello, World!"}, {"label": "日本語", "text": "こんにちは、世界！"}]' | run_clipboard_palette --json
