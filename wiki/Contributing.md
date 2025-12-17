# Contributing

Thank you for considering contributing to the Herakles Process Memory Exporter! This guide will help you get started.

## Development Setup

### Prerequisites

- **Rust**: 1.70 or later
- **Linux**: For /proc filesystem access
- **Git**: For version control

### Clone and Build

```bash
# Clone the repository
git clone https://github.com/herakles-io/herakles-proc-mem-exporter.git
cd herakles-proc-mem-exporter

# Build debug version
cargo build

# Build release version
cargo build --release

# Run tests
cargo test
```

### Development Environment

```bash
# Install development dependencies
rustup component add clippy rustfmt

# Run with debug logging
RUST_LOG=debug cargo run

# Run with test data
cargo run -- -t testdata.json

# Generate test data
cargo run -- generate-testdata -o testdata.json
```

## Code Style Guidelines

### Rust Formatting

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check
```

### Linting

```bash
# Run clippy
cargo clippy

# Fix clippy warnings
cargo clippy --fix
```

### Code Style Rules

1. **Follow Rust conventions**: Use snake_case for functions/variables, CamelCase for types
2. **Document public items**: Add doc comments for public functions and structs
3. **Error handling**: Use `Result` and `Option` appropriately
4. **Avoid unwrap**: Use `?` operator or handle errors explicitly
5. **Keep functions focused**: Single responsibility principle

### Example Code Style

```rust
/// Parses memory metrics from /proc/<pid>/smaps_rollup file.
///
/// # Arguments
/// * `path` - Path to the smaps_rollup file
/// * `buf_kb` - Buffer size in kilobytes
///
/// # Returns
/// Tuple of (RSS, PSS, USS) in bytes, or error
fn parse_smaps_rollup(path: &Path, buf_kb: usize) -> Result<(u64, u64, u64), std::io::Error> {
    let file = fs::File::open(path)?;
    let reader = BufReader::with_capacity(buf_kb * 1024, file);
    
    // Implementation...
    
    Ok((rss_bytes, pss_bytes, uss_bytes))
}
```

## Adding New Subgroups

### Location

Built-in subgroups are defined in `data/subgroups.toml`.

### Format

```toml
subgroups = [
  # Pattern matching by process name
  { group = "category", subgroup = "specific", matches = [
    "process-name",
    "another-process",
  ] },
  
  # Pattern matching by command line
  { group = "category", subgroup = "specific", cmdline_matches = [
    "java.*com.example.App",
  ] },
  
  # Combined matching
  { group = "category", subgroup = "specific", 
    matches = ["myapp"],
    cmdline_matches = ["python.*myapp.py"]
  },
]
```

### Guidelines for New Subgroups

1. **Use appropriate group**: Match existing group categories (db, web, monitoring, etc.)
2. **Be specific**: Use precise process names, avoid overly broad patterns
3. **Test matches**: Verify patterns match intended processes
4. **Add comments**: Document what software the subgroup covers
5. **Alphabetize**: Keep entries sorted within each group section

### Example: Adding a New Database

```toml
# In data/subgroups.toml

  # === Database Systems ===
  # ... existing entries ...
  
  # CockroachDB - Distributed SQL database
  { group = "db", subgroup = "cockroachdb", matches = [
    "cockroach",
  ] },
```

## Adding New Metrics

### Location

Metrics are defined in `src/main.rs` in the `MemoryMetrics` struct.

### Steps

1. **Add field to struct**:
```rust
struct MemoryMetrics {
    // Existing metrics...
    
    // New metric
    new_metric: GaugeVec,
}
```

2. **Create and register in `new()`**:
```rust
fn new(registry: &Registry) -> Result<Self, Box<dyn std::error::Error>> {
    // Existing metrics...
    
    let new_metric = GaugeVec::new(
        Opts::new(
            "herakles_proc_mem_new_metric",
            "Description of the new metric",
        ),
        &["label1", "label2"],
    )?;
    
    registry.register(Box::new(new_metric.clone()))?;
    
    Ok(Self {
        // Existing fields...
        new_metric,
    })
}
```

3. **Add to `reset()`**:
```rust
fn reset(&self) {
    // Existing resets...
    self.new_metric.reset();
}
```

4. **Update in handler**:
```rust
// In metrics_handler
state.metrics.new_metric
    .with_label_values(&[label1, label2])
    .set(value);
```

### Naming Convention

- Prefix: `herakles_proc_mem_`
- Use snake_case
- Be descriptive but concise
- Include unit in name (e.g., `_bytes`, `_seconds`, `_percent`)

## Testing Requirements

### Before Submitting

```bash
# Run all tests
cargo test

# Check formatting
cargo fmt -- --check

# Run clippy
cargo clippy -- -D warnings

# Build release
cargo build --release

# Test with real data
./target/release/herakles-proc-mem-exporter test

# Test with synthetic data
./target/release/herakles-proc-mem-exporter generate-testdata -o /tmp/test.json
./target/release/herakles-proc-mem-exporter -t /tmp/test.json &
curl http://localhost:9215/metrics | head -50
```

### Writing Tests

Add tests in `src/main.rs` or create a `tests/` directory:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_kb_value() {
        assert_eq!(parse_kb_value("1234 kB"), Some(1234));
        assert_eq!(parse_kb_value("  5678  kB  "), Some(5678));
        assert_eq!(parse_kb_value("invalid"), None);
    }

    #[test]
    fn test_classify_process() {
        let (group, subgroup) = classify_process_raw("postgres");
        assert_eq!(group.as_ref(), "db");
        assert_eq!(subgroup.as_ref(), "postgres");
    }
}
```

## Pull Request Process

### Before Opening PR

1. **Fork the repository**
2. **Create a feature branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```
3. **Make your changes**
4. **Run tests**:
   ```bash
   cargo test
   cargo fmt -- --check
   cargo clippy -- -D warnings
   ```
5. **Commit with clear message**:
   ```bash
   git commit -m "feat: add support for XYZ database"
   ```

### Commit Message Format

Follow conventional commits:

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Formatting, no code change
- `refactor`: Code restructuring
- `test`: Adding tests
- `chore`: Maintenance tasks

Examples:
```
feat(subgroups): add support for ScyllaDB
fix(parser): handle empty smaps_rollup file
docs: update installation instructions
refactor(metrics): simplify aggregation logic
```

### PR Description Template

```markdown
## Description
Brief description of the changes.

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Documentation update
- [ ] Refactoring

## Testing
- [ ] Unit tests pass
- [ ] Manual testing completed
- [ ] Documentation updated

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Comments added for complex code
- [ ] Documentation updated
```

## Release Process

### Version Bump

1. Update version in `Cargo.toml`:
   ```toml
   [package]
   version = "0.2.0"
   ```

2. Update version in `main.rs`:
   ```rust
   #[command(version = "0.2.0")]
   ```

3. Create git tag:
   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```

### Release Checklist

- [ ] All tests pass
- [ ] Documentation updated
- [ ] CHANGELOG updated
- [ ] Version bumped
- [ ] Tag created
- [ ] Release notes written

## Getting Help

- **Questions**: Open a GitHub issue with `[Question]` prefix
- **Bugs**: Open a GitHub issue with reproduction steps
- **Feature requests**: Open a GitHub issue with `[Feature]` prefix
- **Contact**: proc-mem@herakles.io

## Code of Conduct

- Be respectful and inclusive
- Welcome newcomers
- Focus on constructive feedback
- Assume good intentions

## License

By contributing, you agree that your contributions will be licensed under the same dual license as the project (MIT OR Apache-2.0).

## Next Steps

- [Architecture overview](Architecture.md)
- [Testing documentation](Testing.md)
- [Configuration reference](Configuration.md)

## 🔗 Project & Support

Project: https://github.com/herakles-io/herakles-proc-mem-exporter — More info: https://www.herakles.io — Support: proc-mem@herakles.io
