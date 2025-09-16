#!/bin/bash

# Test script for enhanced graph in TUI

echo "Testing Enhanced Graph Integration in GitUp TUI"
echo "================================================"
echo ""
echo "The TUI will launch with the following controls:"
echo "  - Press 'E' in the Commits tab to toggle Enhanced Graph"
echo "  - Use 'j'/'k' or arrow keys to navigate"
echo "  - Press 'g' to go to top, 'G' to go to bottom"
echo "  - Use 'Ctrl-d'/'Ctrl-u' for page down/up"
echo "  - Press 'q' to quit"
echo ""
echo "Press Enter to launch the TUI..."
read

./target/release/gitup tui .