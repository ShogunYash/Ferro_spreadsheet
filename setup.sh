#!/bin/bash

# Make the WSL script executable
chmod +x wsl_run.sh

# Display instructions
echo "==================================================="
echo "Ferro Spreadsheet Setup Complete"
echo "==================================================="
echo ""
echo "To run the spreadsheet in WSL:"
echo "  ./wsl_run.sh 10 10"
echo ""
echo "To run the spreadsheet in Windows CMD/PowerShell:"
echo "  run.bat 10 10"
echo ""
echo "To run with make (if Cargo is in PATH):"
echo "  make run 10 10"
echo ""
echo "==================================================="
echo "If you're getting 'cargo not found' errors:"
echo "1. Install Rust at https://rustup.rs/"
echo "2. Make sure Cargo is in your PATH"
echo "==================================================="
