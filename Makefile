.PHONY: all ci publish-dry release release-snapshot clean

all: ci

ci:
	cargo fmt-check
	cargo lint
	cargo check-ws
	cargo check-no-default
	cargo test-ws
	cargo test-all-features
	cargo test-no-default
	cargo doc-all
	cargo security
	cargo audit

publish-dry:
	cargo publish-dry-build
	cargo publish-dry-acta

release:
	goreleaser release --clean --skip=publish,validate

release-snapshot:
	goreleaser release --clean --snapshot

clean:
	cargo clean
	rm -rf dist
