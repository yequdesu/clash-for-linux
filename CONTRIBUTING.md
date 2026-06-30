# Contributing to Clash-Terminal

## Development Setup

1. Clone the repository
2. Install Go 1.21+
3. Install Rust 1.75+
4. Run `make build build-tui`

## Running Tests

```bash
# Go tests
cd tests/unit/go && go test ./...

# Rust tests
cd tui && cargo test

# Integration tests
bash tests/integration/cli_test.sh
```

## Code Style

- Go: Use `gofmt` and `goimports`
- Rust: Use `cargo fmt` and `cargo clippy`

## Pull Request Process

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Run all tests
6. Submit a pull request
