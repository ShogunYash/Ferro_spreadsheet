#!/bin/bash

# Check if Rust is installed in WSL
if ! command -v cargo &> /dev/null; then
    echo "Cargo not found in WSL. Attempting to use Windows cargo..."
    
    # Try to find cargo in Windows
    WIN_CARGO=$(cmd.exe /c "where cargo" 2>/dev/null | head -1)
    
    if [ -z "$WIN_CARGO" ]; then
        echo "Error: Cargo not found in Windows either."
        echo "Please install Rust/Cargo from https://rustup.rs/"
        exit 1
    fi
    
    # Convert Windows path to WSL path
    WIN_CARGO=$(echo "$WIN_CARGO" | tr -d '\r')
    WIN_CARGO_DIR=$(dirname "$WIN_CARGO")
    
    echo "Using Windows Cargo from: $WIN_CARGO"
    
    # Build the project using Windows cargo
    echo "Building project..."
    cmd.exe /c "cd $(wslpath -w $(pwd)) && \"$WIN_CARGO\" build --release"
    
    if [ $? -ne 0 ]; then
        echo "Build failed"
        exit 1
    fi
    
    # Run the spreadsheet with the provided arguments
    echo "Running project..."
    cmd.exe /c "cd $(wslpath -w $(pwd)) && target\\release\\sheet.exe $@"
else
    # Use WSL's native cargo
    echo "Using WSL's Cargo"
    cargo build --release
    
    if [ $? -ne 0 ]; then
        echo "Build failed"
        exit 1
    fi
    
    # Run the compiled binary with arguments
    ./target/release/sheet "$@"
fi
