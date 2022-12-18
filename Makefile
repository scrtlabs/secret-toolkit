.PHONY: check
check:
	cargo check --workspace

.PHONY: clippy
clippy:
	cargo clippy --workspace

.PHONY: test
test:
	cargo test --workspace

.PHONY: publish
publish:
	cargo publish -p secret-toolkit-crypto
	cargo publish -p secret-toolkit-serialization
	cargo publish -p secret-toolkit-incubator
	cargo publish -p secret-toolkit-permit
	cargo publish -p secret-toolkit-utils
	cargo publish -p secret-toolkit-snip20
	cargo publish -p secret-toolkit-snip721
	cargo publish -p secret-toolkit-storage
	cargo publish -p secret-toolkit-viewing-key