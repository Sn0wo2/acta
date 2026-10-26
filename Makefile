.PHONY: all fmt fmt-check check check-all-features check-no-default check-wasm lint lint-fix test test-all test-no-default doc security audit ci build publish-dry run check-debug release release-snapshot clean

ci: lint check check-no-default test test-all test-no-default doc security audit

all: ci

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

check:
	cargo check --workspace --all-targets $(CARGO_FLAGS)

check-all-features:
	cargo check --workspace --all-targets --all-features $(CARGO_FLAGS)

check-no-default:
	cargo check --workspace --all-targets --no-default-features $(CARGO_FLAGS)

check-wasm:
	cargo check -p acta --lib --no-default-features --features wasm-console --target wasm32-unknown-unknown $(CARGO_FLAGS)

lint-fix: fmt
	cargo clippy --workspace --all-targets --all-features --fix --allow-dirty --allow-staged

lint: fmt-check
	cargo clippy --workspace --all-targets --all-features $(CARGO_FLAGS) -- -D warnings

test test-all test-no-default:
	cargo test --workspace $(patsubst test-%,--%-features,$(filter-out test,$@)) $(CARGO_FLAGS)

doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features $(CARGO_FLAGS)

security:
	cargo deny check

audit:
	cargo audit

build:
	cargo build --workspace $(CARGO_FLAGS)

publish-dry:
	cargo publish --manifest-path crates/acta-build/Cargo.toml --dry-run --allow-dirty
	cargo publish --dry-run --allow-dirty

run:
	cargo run -p acta-debug --all-features $(CARGO_FLAGS)

check-debug:
	cargo check -p acta-debug --all-features $(CARGO_FLAGS)

release release-snapshot:
	goreleaser release --clean $(if $(filter release-snapshot,$@),--snapshot,--skip=publish,validate)

clean:
	cargo clean
	rm -rf dist
