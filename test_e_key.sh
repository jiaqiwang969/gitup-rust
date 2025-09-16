#!/bin/bash

echo "Testing GitUp TUI E key functionality"
echo "======================================"
echo ""
echo "Please test the following in the TUI:"
echo "1. Launch: ./target/release/gitup tui ."
echo "2. Make sure you're on the Commits tab (first tab)"
echo "3. Press 'E' (capital E) - you should see a message 'Enhanced Graph: ON'"
echo "4. Press 'v' to enable graph view if not already visible"
echo "5. If still no change, try:"
echo "   - Press 'E' again to toggle off/on"
echo "   - Check the status bar for any error messages"
echo ""
echo "Debug steps:"
echo "- The enhanced graph only works on the Commits tab"
echo "- Look for the status message at the bottom when pressing 'E'"
echo ""
echo "Press Enter to continue..."
read

# Try to check if the enhanced graph module is actually being compiled in
echo "Checking if enhanced_graph module is included in the build..."
nm target/release/gitup | grep -i enhanced | head -5

echo ""
echo "Checking EnhancedGraphIntegration..."
strings target/release/gitup | grep -i "Enhanced Graph" | head -5