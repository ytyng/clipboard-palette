#!/usr/bin/env zsh

cd $(dirname $0)/..

echo '[{"label": "Copy text", "text": "Hello, World!"}, {"label": "日本語", "text": "こんにちは、世界！"}]' | ./src-tauri/target/release/clipboard-palette --json
