#!/usr/bin/env zsh

source "${0:A:h}/config.sh"

echo "This is section1 Line1
This is section1 Line2

This is section1 Line3


This is section2 Line1

This is section2 Line2
This is section2 Line3
" | run_clipboard_palette --split-empty-line=2
