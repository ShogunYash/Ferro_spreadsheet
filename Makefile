# Makefile for Ferro_spreadsheet

# Detect the environment
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Linux)
  # Running in WSL or Linux
  CARGO = $(shell command -v cargo 2>/dev/null || echo "cargo")
  # If cargo is not found, try using Windows cargo via cmd.exe
  ifeq ($(CARGO),cargo)
    CARGO_PATH = $(shell cmd.exe /c "where cargo" 2>/dev/null | head -1 | tr -d '\r')
    ifneq ($(CARGO_PATH),)
      CARGO_DIR = $(dir $(CARGO_PATH))
      CARGO = cmd.exe /c $(CARGO_PATH)
    endif
  endif
else
  # Windows environment
  CARGO = cargo
endif

# Build targets
.PHONY: all clean build run check-cargo

# Default target
all: check-cargo build

# Check if cargo is available
check-cargo:
	@if [ "$(CARGO)" = "cargo" ] && ! command -v cargo >/dev/null; then \
		echo "Error: Cargo is not installed or not in PATH"; \
		echo "Please install Rust/Cargo from https://rustup.rs/"; \
		exit 1; \
	fi

# Build the project
build: check-cargo
	@echo "Building Ferro spreadsheet..."
	$(CARGO) build --release

# Run the project with provided arguments
run: check-cargo
	@echo "Running Ferro spreadsheet..."
	$(CARGO) run --release -- $(filter-out $@,$(MAKECMDGOALS))

# Clean the project
clean: check-cargo
	@echo "Cleaning up..."
	$(CARGO) clean

# Install the project (UNIX-like systems)
install: build
	@echo "Installing Ferro spreadsheet..."
	cp target/release/sheet /usr/local/bin/

# Install the project (Windows)
install-win: build
	@echo "Installing Ferro spreadsheet..."
	copy /Y target\release\sheet.exe %USERPROFILE%\bin\

# Allow passing arguments to the run target
%:
	@:
