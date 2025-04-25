.PHONY: all clean run test coverage docs ext1

all: target/release/spreadsheet

target/release/spreadsheet:
	cargo build --release
	
clean:
	cargo clean
	rm -f report.aux report.log report.out report.pdf report.toc *.dot *.png *.html *.sheet *.txt

run: target/release/spreadsheet
	./target/release/spreadsheet 999 18278

ext1: target/release/spreadsheet
	./target/release/spreadsheet --vim 999 18278 rust_spreadsheet.sheet

# Run all tests
test:
	cargo test

# Run test coverage with tarpaulin
coverage:
	cargo tarpaulin --out Html --output-dir coverage

# Generate documentation
docs:
	cargo doc --no-deps
	pdflatex report.tex


# # Makefile for Ferro_spreadsheet

# # Detect the environment
# UNAME_S := $(shell uname -s)
# ifeq ($(UNAME_S),Linux)
#   # Running in WSL or Linux
#   CARGO = $(shell command -v cargo 2>/dev/null || echo "cargo")
#   # If cargo is not found, try using Windows cargo via cmd.exe
#   ifeq ($(CARGO),cargo)
#     CARGO_PATH = $(shell cmd.exe /c "where cargo" 2>/dev/null | head -1 | tr -d '\r')
#     ifneq ($(CARGO_PATH),)
#       CARGO_DIR = $(dir $(CARGO_PATH))
#       CARGO = cmd.exe /c $(CARGO_PATH)
#     endif
#   endif
# else
#   # Windows environment
#   CARGO = cargo
# endif

# # Build configuration
# RELEASE_FLAGS = --release
# # Fix: Set RUSTFLAGS as an environment variable when calling cargo
# # RUSTFLAGS_OPTIMIZED = -C target-cpu=native -C opt-level=3 -C codegen-units=1 -C lto=fat
# # Modify the RUSTFLAGS_OPTIMIZED line
# # RUSTFLAGS_OPTIMIZED = -C target-cpu=native -C opt-level=3 -C codegen-units=1 -C lto=thin
# RUSTFLAGS_OPTIMIZED = -C target-cpu=native -C opt-level=3

# # Build targets with different optimization levels
# .PHONY: all clean build run check-cargo optimized bench test package

# # Default target
# all: check-cargo build

# # Check if cargo is available
# check-cargo:
# 	@if [ "$(CARGO)" = "cargo" ] && ! command -v cargo >/dev/null; then \
# 		echo "Error: Cargo is not installed or not in PATH"; \
# 		echo "Please install Rust/Cargo from https://rustup.rs/"; \
# 		exit 1; \
# 	fi

# # Build the project
# build: check-cargo
# 	@echo "Building Ferro spreadsheet..."
# 	$(CARGO) build --release

# # Build with high optimization settings
# optimized: check-cargo
# 	@echo "Building highly optimized Ferro spreadsheet..."
# 	RUSTFLAGS="$(RUSTFLAGS_OPTIMIZED)" $(CARGO) build $(RELEASE_FLAGS)

# # Run the project with provided arguments
# run: check-cargo
# 	@echo "Running Ferro spreadsheet..."
# 	$(CARGO) run --release -- $(filter-out $@,$(MAKECMDGOALS))

# # Run with high optimization
# run-optimized: optimized
# 	@echo "Running optimized Ferro spreadsheet..."
# 	RUSTFLAGS="$(RUSTFLAGS_OPTIMIZED)" $(CARGO) run $(RELEASE_FLAGS) -- $(filter-out $@,$(MAKECMDGOALS))

# # Benchmark the application
# bench: check-cargo
# 	@echo "Running benchmarks..."
# 	$(CARGO) bench

# # Run tests
# test: check-cargo
# 	@echo "Running tests..."
# 	$(CARGO) test

# # Clean the project
# clean: check-cargo
# 	@echo "Cleaning up..."
# 	$(CARGO) clean

# # Install the project (UNIX-like systems)
# install: build
# 	@echo "Installing Ferro spreadsheet..."
# 	cp target/release/sheet /usr/local/bin/

# # Install the project (Windows)
# install-win: build
# 	@echo "Installing Ferro spreadsheet..."
# 	copy /Y target\release\sheet.exe %USERPROFILE%\bin\

# # Create release package
# package: optimized
# 	@echo "Creating release package..."
# 	@mkdir -p release-package
# 	cp target/release/sheet release-package/
# 	cp README.md release-package/ 2>/dev/null || echo "No README found"
# 	@echo "Package created in release-package directory"

# # Allow passing arguments to the run target
# %:
# 	@:
