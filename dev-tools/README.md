# Development Tools

This directory provides a reference for testing the smart contract against a local Provenance chain.

## Prerequisites

- [jq](https://jqlang.github.io/jq/)

## Instructions

1. Follow the [root README instructions](../README.md) to obtain an optimized WASM file of the smart contract.
2. Stand up a local Provenance chain.
3. Inspect `example.sh` and make changes as you see fit to accommodate your local development environment.
4. Run the example script `example.sh <path to your local Provenance node> <path to contract WASM>`.
