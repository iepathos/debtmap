# debtmap

> 🚧 **Early Prototype** - This project is under active development and APIs may change

A fast, language-agnostic code complexity and technical debt analyzer written in Rust. Debtmap helps identify areas of technical debt in your codebase by analyzing complexity, duplication, and code quality patterns.

## Features

- **Multi-language support** - Currently supports Rust and Python with extensible architecture for more languages
- **Comprehensive metrics** - Analyzes cyclomatic and cognitive complexity, code duplication, and various code smells
- **Parallel processing** - Built with Rust and Rayon for blazing-fast analysis of large codebases
- **Multiple output formats** - JSON, TOML, and human-readable table formats
- **Configurable thresholds** - Customize complexity and duplication thresholds to match your standards
- **Incremental analysis** - Smart caching system for analyzing only changed files

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/iepathos/debtmap.git
cd debtmap

# Build and install
cargo install --path .
```

### Using Cargo

```bash
cargo install debtmap
```

## Quick Start

```bash
# Analyze current directory
debtmap analyze .

# Analyze with custom thresholds
debtmap analyze ./src --threshold-complexity 15 --threshold-duplication 50

# Output as JSON
debtmap analyze ./src --format json --output report.json

# Analyze specific languages only
debtmap analyze . --languages rust,python

# Initialize configuration file
debtmap init

# Validate project against thresholds
debtmap validate ./src
```

## Commands

### `analyze`
Comprehensive analysis including complexity metrics, code duplication, technical debt patterns, and dependency analysis.

```bash
debtmap analyze <PATH> [OPTIONS]

Options:
  -f, --format <FORMAT>              Output format [default: terminal] [possible values: json, markdown, terminal]
  -o, --output <FILE>                Output file (stdout if not specified)
  --threshold-complexity <N>         Complexity threshold [default: 10]
  --threshold-duplication <N>        Duplication threshold in lines [default: 50]
  --languages <LANGS>                Comma-separated list of languages to analyze
```

### `init`
Initialize a configuration file for the project.

```bash
debtmap init [OPTIONS]

Options:
  -f, --force    Force overwrite existing configuration file
```

### `validate`
Validate code against configured thresholds and fail if metrics exceed limits.

```bash
debtmap validate <PATH> [OPTIONS]

Options:
  -c, --config <FILE>    Configuration file to use [default: .debtmap.toml]
```

## Metrics Explained

### Cyclomatic Complexity
Measures the number of linearly independent paths through code. Higher values indicate more complex, harder-to-test code.

- **1-5**: Simple, easy to test
- **6-10**: Moderate complexity
- **11-20**: Complex, consider refactoring
- **20+**: Very complex, high risk

### Cognitive Complexity
Measures how difficult code is to understand. Unlike cyclomatic complexity, it considers nesting depth and control flow interruptions.

### Code Duplication
Identifies similar code blocks that could be refactored into shared functions.

### Technical Debt Patterns
- **Long methods/functions**: Functions exceeding recommended line counts
- **Deep nesting**: Code with excessive indentation levels
- **Large files**: Files that have grown too large to maintain easily
- **Circular dependencies**: Modules that depend on each other
- **High coupling**: Excessive dependencies between modules

## Configuration

Create a `.debtmap.toml` file in your project root:

```toml
[thresholds]
complexity = 15
duplication = 25
max_file_lines = 500
max_function_lines = 50
max_nesting_depth = 4

[ignore]
paths = ["target/", "node_modules/", "vendor/"]
patterns = ["*.generated.rs", "*.pb.go"]

[languages]
enabled = ["rust", "python"]
```

## Output Examples

### Terminal Format (Default)
```
╭─────────────────────────────────────────────────────────────╮
│                    Debtmap Analysis Report                  │
├─────────────────────────────────────────────────────────────┤
│ File                     │ Complexity │ Debt Items │ Issues │
├──────────────────────────┼────────────┼────────────┼────────┤
│ src/analyzers/rust.rs    │ 15         │ 3          │ 2      │
│ src/core/metrics.rs      │ 8          │ 1          │ 0      │
│ src/debt/patterns.rs     │ 22         │ 5          │ 3      │
╰─────────────────────────────────────────────────────────────╯
```

### JSON Format
```json
{
  "timestamp": "2024-01-09T12:00:00Z",
  "summary": {
    "total_files": 25,
    "high_complexity_files": 3,
    "high_duplication_files": 2,
    "total_issues": 8
  },
  "files": [
    {
      "path": "src/analyzers/rust.rs",
      "complexity": {
        "cyclomatic": 15,
        "cognitive": 18
      },
      "duplication_percentage": 12,
      "issues": [...]
    }
  ]
}
```

## Architecture

Debtmap is built with a functional, modular architecture:

- **`analyzers/`** - Language-specific AST parsers and analyzers
- **`complexity/`** - Complexity calculation algorithms
- **`debt/`** - Technical debt pattern detection
- **`core/`** - Core data structures and traits
- **`io/`** - File walking and output formatting
- **`transformers/`** - Data transformation pipelines

## Contributing

We welcome contributions! This is an early-stage project, so there's plenty of room for improvement.

### Areas for Contribution

- **Language support**: Add analyzers for JavaScript, Go, Java, etc.
- **New metrics**: Implement additional complexity or quality metrics
- **Performance**: Optimize analysis algorithms
- **Documentation**: Improve docs and add examples
- **Testing**: Expand test coverage

### Development

```bash
# Run tests
cargo test

# Run with verbose output
RUST_LOG=debug cargo run -- analyze ./src

# Benchmark
cargo bench

# Format code
cargo fmt

# Run lints
cargo clippy
```

## License

MIT License - see [LICENSE](LICENSE) file for details

## Roadmap

- [ ] JavaScript/TypeScript support
- [ ] Go support
- [ ] Integration with CI/CD pipelines
- [ ] Web UI for visualization
- [ ] Historical trend tracking
- [ ] IDE plugins
- [ ] Automated refactoring suggestions
- [ ] Machine learning-based debt prediction

## Acknowledgments

Built with excellent Rust crates including:
- [syn](https://github.com/dtolnay/syn) for Rust AST parsing
- [rustpython-parser](https://github.com/RustPython/RustPython) for Python parsing
- [rayon](https://github.com/rayon-rs/rayon) for parallel processing
- [clap](https://github.com/clap-rs/clap) for CLI parsing

---

**Note**: This is a prototype tool under active development. Please report issues and feedback on [GitHub](https://github.com/iepathos/debtmap/issues).