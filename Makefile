.PHONY: all fix lint test test-all test-no-default fmt fmt-check clippy publish-dry run debug build release release-snapshot check clean check-all-features check-no-default check-wasm

all: check

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

clippy:
	cargo clippy --all-features -- -D warnings

lint: fmt-check clippy

fix:
	cargo fix --all-features --allow-dirty

test:
	cargo test --all --all-features

test-all:
	cargo test --all

test-no-default:
	cargo test --all --no-default-features

check-all-features:
	cargo check --all-targets --all-features

check-no-default:
	cargo check --all-targets --no-default-features

check-wasm:
	cargo check -p acta --lib --no-default-features --features unicode,wasm-console --target wasm32-unknown-unknown

check: fix lint test test-all test-no-default check-all-features check-no-default publish-dry

publish-dry:
	cargo publish --manifest-path crates/acta-build/Cargo.toml --dry-run --allow-dirty
	cargo publish --dry-run --allow-dirty

run:
	cargo run -p acta-debug --all-features

check-debug:
	cargo check -p acta-debug --all-features

build:
	goreleaser build --snapshot --clean --skip=before --config .goreleaser.yml

release:
	goreleaser release --clean --skip=before --skip=validate --config .goreleaser.yml

release-snapshot:
	goreleaser release --snapshot --clean --skip=before --skip=validate --config .goreleaser.yml

clean:
	cargo clean
	rm -rf dist/
