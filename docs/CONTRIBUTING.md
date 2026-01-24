# Contributing to proxy-spider

Thank you for your interest in contributing to proxy-spider! This document provides guidelines and instructions for contributing.

## 🎯 Ways to Contribute

- **Bug Reports**: Report bugs via GitHub Issues
- **Feature Requests**: Suggest new features or improvements
- **Code Contributions**: Submit pull requests with bug fixes or new features
- **Documentation**: Improve documentation, examples, or tutorials
- **Testing**: Add test cases or improve test coverage

## 🚀 Getting Started

### Prerequisites

- Rust 1.75 or later (beta channel recommended)
- Git
- (Optional) Docker for testing containerized builds

### Setting Up Development Environment

1. **Fork and Clone**
   ```bash
   git clone https://github.com/YOUR_USERNAME/proxy-spider.git
   cd proxy-spider
   ```

2. **Install Dependencies**
   ```bash
   # Install Rust if you haven't already
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   
   # Install beta toolchain
   rustup install beta
   rustup default beta
   ```

3. **Build the Project**
   ```bash
   cargo build --all-features
   ```

4. **Run Tests**
   ```bash
   cargo test --all-features
   ```

5. **Install Pre-commit Hooks**
   ```bash
   pip install pre-commit
   pre-commit install
   ```

## 📝 Development Workflow

### 1. Create a Branch

Create a descriptive branch name:
```bash
git checkout -b feature/add-new-scraper
git checkout -b fix/proxy-parsing-bug
git checkout -b docs/improve-readme
```

### 2. Make Changes

- Write clean, idiomatic Rust code
- Follow the existing code style
- Add tests for new functionality
- Update documentation as needed

### 3. Test Your Changes

```bash
# Run all tests
cargo test --all-features

# Run clippy
cargo clippy --all-features --all-targets

# Run formatting check
cargo fmt --check

# Run benchmarks (if applicable)
cargo bench
```

### 4. Commit Your Changes

Write clear, descriptive commit messages:
```bash
git add .
git commit -m "feat: add support for custom proxy validators"
```

**Commit Message Format:**
- `feat:` New feature
- `fix:` Bug fix
- `docs:` Documentation changes
- `test:` Adding or updating tests
- `refactor:` Code refactoring
- `perf:` Performance improvements
- `chore:` Maintenance tasks

### 5. Push and Create Pull Request

```bash
git push origin your-branch-name
```

Then create a pull request on GitHub with:
- Clear title and description
- Reference to related issues
- Screenshots/examples if applicable

## 🧪 Testing Guidelines

### Unit Tests

Add unit tests in the same file as the code:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_parsing() {
        // Test implementation
    }
}
```

### Integration Tests

Add integration tests in the `tests/` directory:
```rust
// tests/integration_test.rs
use proxy_spider::*;

#[test]
fn test_end_to_end_flow() {
    // Test implementation
}
```

### Test Coverage

Aim for:
- 70%+ code coverage for new features
- 100% coverage for critical paths
- Edge cases and error conditions

## 📚 Documentation Guidelines

### Code Documentation

Document all public APIs:
```rust
/// Checks if a proxy is working
///
/// # Arguments
///
/// * `proxy` - The proxy to check
/// * `config` - Configuration settings
///
/// # Returns
///
/// Returns `Ok(())` if the proxy is working, `Err` otherwise
///
/// # Examples
///
/// ```
/// let proxy = Proxy::new("http://192.168.1.1:8080");
/// check_proxy(&proxy, &config).await?;
/// ```
pub async fn check_proxy(proxy: &Proxy, config: &Config) -> Result<()> {
    // Implementation
}
```

### README Updates

Update README.md when adding:
- New features
- New configuration options
- Breaking changes

## 🎨 Code Style

### Rust Style

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `rustfmt` for formatting
- Use `clippy` for linting
- Prefer explicit types over `auto`
- Use meaningful variable names

### Naming Conventions

- `snake_case` for functions and variables
- `PascalCase` for types and traits
- `SCREAMING_SNAKE_CASE` for constants
- Descriptive names over abbreviations

### Error Handling

- Use `Result` for fallible operations
- Provide context with error messages
- Use custom error types from `src/errors.rs`
- Add suggestions for common errors

### Async Code

- Use `async/await` syntax
- Prefer `tokio::spawn` for background tasks
- Use `tokio::select!` for cancellation
- Add timeouts for network operations

## 🔍 Code Review Process

### What We Look For

- **Correctness**: Does the code work as intended?
- **Tests**: Are there adequate tests?
- **Documentation**: Is the code well-documented?
- **Style**: Does it follow our style guidelines?
- **Performance**: Are there any performance concerns?
- **Security**: Are there any security implications?

### Review Timeline

- Initial review within 48 hours
- Follow-up reviews within 24 hours
- Merge when approved by maintainer

## 🐛 Bug Reports

### Before Reporting

1. Check if the bug is already reported
2. Try to reproduce with the latest version
3. Gather relevant information

### Bug Report Template

```markdown
**Describe the bug**
A clear description of the bug

**To Reproduce**
Steps to reproduce:
1. Run command '...'
2. See error

**Expected behavior**
What you expected to happen

**Actual behavior**
What actually happened

**Environment**
- OS: [e.g., Ubuntu 22.04]
- Rust version: [e.g., 1.75]
- proxy-spider version: [e.g., 0.1.0]

**Additional context**
Any other relevant information
```

## 💡 Feature Requests

### Feature Request Template

```markdown
**Is your feature request related to a problem?**
A clear description of the problem

**Describe the solution you'd like**
A clear description of what you want to happen

**Describe alternatives you've considered**
Alternative solutions or features

**Additional context**
Any other context or screenshots
```

## 📋 Pull Request Checklist

Before submitting a PR, ensure:

- [ ] Code compiles without warnings
- [ ] All tests pass
- [ ] New tests added for new functionality
- [ ] Documentation updated
- [ ] Commit messages are clear
- [ ] Code follows style guidelines
- [ ] No unnecessary dependencies added
- [ ] Performance impact considered
- [ ] Security implications reviewed

## 🏆 Recognition

Contributors will be:
- Listed in the project README
- Mentioned in release notes
- Credited in commit history

## 📞 Getting Help

- **Questions**: Open a GitHub Discussion
- **Chat**: Join our community chat (if available)
- **Email**: Contact maintainers directly

## 📜 License

By contributing, you agree that your contributions will be licensed under the MIT License.

## 🙏 Thank You!

Thank you for contributing to proxy-spider! Your efforts help make this project better for everyone.
