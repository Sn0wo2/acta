.DEFAULT_GOAL := ci
CARGO_FLAGS ?=

.PHONY: all fmt fmt-check check check-all-features check-no-default check-wasm fix lint clippy test test-all test-no-default doc deny audit ci build publish-dry run check-debug release release-snapshot clean

all: ci

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

check:
	cargo check --all-targets $(CARGO_FLAGS)

check-all-features:
	cargo check --workspace --all-targets --all-features $(CARGO_FLAGS)

check-no-default:
	cargo check -p acta --all-targets --no-default-features $(CARGO_FLAGS)

check-wasm:
	cargo check -p acta --lib --no-default-features --features wasm-console --target wasm32-unknown-unknown $(CARGO_FLAGS)

fix: fmt
	cargo clippy --workspace --all-targets --all-features --fix --allow-dirty --allow-staged $(CARGO_FLAGS)

lint: fmt-check clippy

clippy:
	cargo clippy --workspace --all-targets --all-features $(CARGO_FLAGS) -- -D warnings

test:
	cargo test --workspace --all-features $(CARGO_FLAGS)

test-all:
	cargo test --workspace $(CARGO_FLAGS)

test-no-default:
	cargo test -p acta --no-default-features $(CARGO_FLAGS)

doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features $(CARGO_FLAGS)

deny:
	cargo deny check

audit:
	cargo audit

ci: fmt-check check check-all-features check-no-default clippy test test-all test-no-default doc deny audit

build:
	cargo build --workspace --all-features $(CARGO_FLAGS)

publish-dry:
	cargo publish --manifest-path crates/acta-build/Cargo.toml --dry-run --allow-dirty
	cargo publish --dry-run --allow-dirty

run:
	cargo run -p acta-debug --all-features $(CARGO_FLAGS)

check-debug:
	cargo check -p acta-debug --all-features $(CARGO_FLAGS)

release:
	goreleaser release --clean --skip=before,publish,validate --config .goreleaser.yml

release-snapshot:
	goreleaser release --snapshot --clean --skip=before,publish,validate --config .goreleaser.yml

clean:
	cargo clean
	rm -rf dist
