#!/bin/bash

# Build the project
cargo build --release

# Check if the build succeeded
if [ $? -ne 0 ]; then
    echo "Build failed"
    exit 1
fi

# Run the spreadsheet with the provided arguments
./target/release/sheet "$@"
