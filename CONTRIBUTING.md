# Contributing to Sentinel-MCP

Thank you for your interest in contributing to Sentinel-MCP! This document provides guidelines and instructions for contributing.

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment for all contributors.

## How to Contribute

### Reporting Bugs

If you find a bug, please create an issue with:
- Clear description of the problem
- Steps to reproduce
- Expected vs actual behavior
- Environment details (OS, Rust version, etc.)
- Relevant logs or error messages

### Suggesting Features

Feature requests are welcome! Please create an issue with:
- Clear description of the feature
- Use case and benefits
- Potential implementation approach
- Any relevant examples

### Pull Requests

1. **Fork the repository**
2. **Create a feature branch**: `git checkout -b feature/your-feature-name`
3. **Make your changes**
4. **Write tests** for new functionality
5. **Run tests**: `cargo test`
6. **Run clippy**: `cargo clippy -- -D warnings`
7. **Format code**: `cargo fmt`
8. **Commit with clear messages**
9. **Push to your fork**
10. **Create a Pull Request**

### Development Setup

```bash
# Clone the repository
git clone https://github.com/paulmmoore3416/Sentinel-MCP.git
cd Sentinel-MCP

# Install dependencies
cargo build

# Run tests
cargo test

# Run the server
cargo run
```

### Code Style

- Follow Rust standard style guidelines
- Use `cargo fmt` before committing
- Run `cargo clippy` and address warnings
- Write clear, descriptive comments
- Keep functions focused and small

### Testing

- Write unit tests for new functions
- Add integration tests for new features
- Ensure all tests pass before submitting PR
- Aim for >80% code coverage

### Documentation

- Update README.md if adding features
- Add inline documentation for public APIs
- Update ARCHITECTURE.md for design changes
- Include examples in documentation

## Project Structure

```
sentinel-mcp/
├── src/           # Source code
├── tests/         # Test files
├── docs/          # Documentation
├── prompts/       # IBM Bob prompts
├── k8s/           # Kubernetes manifests
└── examples/      # Example files
```

## Getting Help

- Check existing issues and documentation
- Ask questions in GitHub Discussions
- Review the implementation guide in docs/

## License

By contributing, you agree that your contributions will be licensed under the MIT License.