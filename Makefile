#!/usr/bin/make -f
CONTAINER_RUNTIME := $(shell which docker 2>/dev/null || which podman 2>/dev/null)

.PHONY: all
all: fmt build test lint schema

.PHONY: fmt
fmt:
	@cargo fmt --all -- --check

.PHONY: build
build:
	@cargo wasm

.PHONY: unit-test
unit-test:
	@RUST_BACKTRACE=1 cargo unit-test

.PHONY: doc-test
doc-test:
	@RUST_BACKTRACE=1 cargo doc-test

.PHONY: test
test: unit-test doc-test

.PHONY: lint
lint:
	@cargo clippy -- -D warnings

.PHONY: schema
schema:
	@cargo schema

.PHONY: optimize
optimize:
	$(CONTAINER_RUNTIME) run --rm -v $(CURDIR):/code:Z \
		--mount type=volume,source=asset-classification-smart-contract_cache,target=/code/target \
		--mount type=volume,source=asset-classification-smart-contract_registry_cache,target=/usr/local/cargo/registry \
		cosmwasm/rust-optimizer:0.12.6

