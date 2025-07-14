#!/usr/bin/env zsh

cd $(dirname $0)/..

echo "This is section1 Line1
This is section1 Line2

This is section1 Line3


This is section2 Line1

This is section2 Line2
This is section2 Line3
" | ./src-tauri/target/debug/clipboard-palette --split-empty-line=2
