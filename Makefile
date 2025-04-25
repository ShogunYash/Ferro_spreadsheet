.PHONY: all clean run extensions vim vim-load test test-extensions coverage coverage-extensions view-report clippy clippy-extensions fmt fmt-fix docs

# Default target builds without extensions
all: target/release/spreadsheet-core

# Build without extensions (core functionality only)
target/release/spreadsheet-core:
	cargo build --release

# Build with extensions (includes vim mode and extensions)
target/release/spreadsheet-extensions:
	cargo build --release --features extensions

clean:
	cargo clean
	rm -f report.aux report.log report.out report.pdf report.toc *.dot *.png *.html *.sheet *.txt

# Run without extensions
run: target/release/spreadsheet-core
	./target/release/spreadsheet 999 18278

# Run with extensions
extensions: target/release/spreadsheet-extensions
	./target/release/spreadsheet 999 18278

# Vim mode requires extensions
vim: target/release/spreadsheet-extensions
	./target/release/spreadsheet --vim 999 18278

# Vim mode with file loading requires extensions
vim-load: target/release/spreadsheet-extensions
	./target/release/spreadsheet --vim 999 18278 rust_spreadsheet.sheet

# Testing targets
test:
	cargo test

test-extensions:
	cargo test --features extensions

docs:
	cargo doc --no-deps
	pdflatex report.tex
# Code coverage targets
coverage:
	cargo tarpaulin --ignore-tests --out Html --include-files 'src/*'

coverage-extensions:
	cargo tarpaulin --ignore-tests --out Html --include-files 'src/*' --features extensions

view-report:
	explorer.exe tarpaulin-report.html
	
clippy:
	cargo clippy

clippy-extensions:
	cargo clippy --features extensions

fmt:
	cargo fmt --check

fmt-fix:
	cargo fmt