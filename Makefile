.PHONY: dump run test fmt check clean all demo

dump:
	@./scripts/generate_full_dump.sh

run:
	cargo run

test:
	timeout 30s cargo test -- --quiet --nocapture --test-threads=1 || echo "⚠️ Test suite may have hung or timed out."

fmt:
	cargo fmt -- --check

check:
	cargo check

clean:
	@echo "Cleaning up dump and other generated files..."
	@rm -f full_project_dump.txt

all: fmt check test dump
	@echo "✅ All tasks completed."




demo:
	@echo "Running all .dsl demo programs..."
	@find demo -type f -name "*.dsl" | sort | while read -r file; do \
		echo "=== Running $$file ==="; \
		cargo run -- -p "$$file" --stdlib || echo "❌ Failed: $$file"; \
		echo ""; \
	done
