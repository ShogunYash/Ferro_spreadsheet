@echo off
rem Build the project
cargo build --release

rem Check if the build succeeded
if %errorlevel% neq 0 (
    echo Build failed
    exit /b %errorlevel%
)

rem Run the spreadsheet with the provided arguments
target\release\sheet %*
