.PHONY: build check test fmt lint coverage coverage-report mutants mutants-list mutants-module ci doc clean

build:
	cargo build

check:
	cargo check

test:
	cargo test

fmt:
	cargo fmt --check

lint:
	cargo clippy -- -D warnings

coverage:
	cargo llvm-cov --html --fail-under-lines 90

coverage-report:
	cargo llvm-cov --open

mutants:
	cargo mutants --timeout 60

mutants-list:
	cargo mutants --list

mutants-module:
	@test -n "$(MOD)" || (echo "Usage: make mutants-module MOD=src/model/" && exit 1)
	cargo mutants -f "$(MOD)" --timeout 60

ci: fmt lint test coverage
	@echo "All CI checks passed"

doc:
	cargo doc --no-deps --open

clean:
	cargo clean
	rm -rf mutants.out/
