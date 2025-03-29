.PHONY: dump run test fmt check

dump:
	@./scripts/generate_full_dump.sh

run:
	cargo run

test:
	cargo test

fmt:
	cargo fmt -- --check

check:
	cargo check
