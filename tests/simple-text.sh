#!/usr/bin/env zsh

cd $(dirname $0)/..

echo "Hello, World!" | ./src-tauri/target/release/clipboard-palette
