.PHONY: check
check:
	cargo check --workspace

.PHONY: clippy
clippy:
	cargo clippy --workspace

.PHONY: test
test:
	cargo test --workspace
