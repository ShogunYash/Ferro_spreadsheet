.PHONY: all clean test

all:
	cargo build

release:
	cargo build --release
	cp target/release/sheet ./sheet

test:
	cargo test

clean:
	cargo clean
	rm -f sheet
