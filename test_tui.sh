#!/bin/bash

# Test script for GitUp TUI
# Run this in a proper terminal environment

# Build the TUI
echo "Building GitUp TUI..."
cargo build --release --package gitup-tui

# Run the TUI
echo "Starting GitUp TUI..."
echo "Use vim keybindings to navigate:"
echo "  h/j/k/l - move around"
echo "  gg/G - go to top/bottom"
echo "  q - quit"
echo ""
echo "Press Enter to continue..."
read

# Run the TUI with the current directory
./target/release/gitup-tui .