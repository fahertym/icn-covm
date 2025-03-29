.PHONY: build clean test clippy fmt doc benchmark

# Default target
all: build test clippy fmt

# Build the project
build:
	@echo "Building nano-cvm..."
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

# Run all
run_all: all
	@echo "Running nano-cvm..."
	cargo run -- --program program.dsl
