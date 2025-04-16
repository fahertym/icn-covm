.PHONY: build clean test clippy fmt doc benchmark federation-test

# Default target
all: build test clippy fmt

# Build the project
build:
	@echo "Building icn-covm..."
	cargo build

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	cargo clean

# Run tests
test:
	@echo "Running tests..."
	cargo test

# Run clippy lints
clippy:
	@echo "Running clippy..."
	cargo clippy -- -D warnings

# Check code formatting
fmt:
	@echo "Checking code formatting..."
	cargo fmt -- --check

# Build and open documentation
doc:
	@echo "Building documentation..."
	cargo doc --no-deps --open

# Run benchmarks comparing AST vs bytecode execution
benchmark:
	@echo "Running benchmarks..."
	@echo "Benchmark: Fibonacci"
	@cargo run --release -- --benchmark --program demo/benchmark/fibonacci.dsl
	@echo ""
	@echo "Benchmark: Factorial"
	@cargo run --release -- --benchmark --program demo/benchmark/factorial.dsl
	@echo ""
	@echo "Benchmark: Loop"
	@cargo run --release -- --benchmark --program demo/benchmark/loop.dsl

# Run federation tests using Docker
federation-test:
	@echo "Running federation tests..."
	@echo "Building Docker images..."
	docker-compose build
	@echo "Starting bootstrap node..."
	docker-compose up -d node1
	@echo "Waiting for bootstrap node to initialize (10s)..."
	@sleep 10
	@echo "Getting bootstrap node ID..."
	@PEER_ID=$$(docker-compose logs node1 | grep "Local peer ID" | tail -n 1 | sed -E 's/.*Local peer ID: ([^ ]+).*/\1/'); \
	if [ -n "$$PEER_ID" ]; then \
		echo "Found bootstrap node ID: $$PEER_ID"; \
		./update_bootstrap.sh $$PEER_ID; \
		echo "Updated bootstrap configuration"; \
		echo "Starting other nodes..."; \
		docker-compose up -d node2 node3; \
		echo "Waiting for nodes to connect (15s)..."; \
		sleep 15; \
		echo "Federation test logs:"; \
		docker-compose logs | grep -E "Connected to|mDNS discovered|Kademlia query"; \
		echo "Checking for successful node connections..."; \
		CONNECTION_COUNT=$$(docker-compose logs | grep -c "Connected to"); \
		if [ $$CONNECTION_COUNT -gt 0 ]; then \
			echo "✅ Federation test passed: $$CONNECTION_COUNT connections established"; \
			docker-compose down; \
			exit 0; \
		else \
			echo "❌ Federation test failed: No connections established"; \
			docker-compose down; \
			exit 1; \
		fi; \
	else \
		echo "❌ Could not find bootstrap node ID"; \
		docker-compose down; \
		exit 1; \
	fi

# Run all
run_all: all
	@echo "Running icn-covm..."
	cargo run -- --program program.dsl
