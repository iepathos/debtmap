# Debtmap Coding Conventions

## Functional Programming Guidelines
1. **Pure Functions First**: Keep side effects at IO boundaries
2. **Immutability**: Use persistent data structures from `im`
3. **Composition**: Build complex behavior from simple functions
4. **No Mutable State**: Avoid `mut` except in visitor patterns
5. **Explicit Error Handling**: Use Result/Option, never panic

## Code Style
- Use `rustfmt` default configuration
- Maximum line length: 100 characters
- Function length: Prefer under 50 lines
- Module organization: One concept per module

## Naming Conventions
- Functions: `snake_case` verb phrases (e.g., `calculate_complexity`)
- Types: `PascalCase` nouns (e.g., `FileMetrics`)
- Constants: `SCREAMING_SNAKE_CASE`
- Modules: `snake_case` nouns

## Error Handling
- Use `anyhow::Result` for application errors
- Use `thiserror` for library errors
- Always provide context with `.context()`
- Never use `unwrap()` in production code

## Testing
- Unit tests in same file as implementation
- Integration tests in `tests/` directory
- Property-based tests with `proptest`
- Minimum 80% code coverage

## Documentation
- All public APIs must have rustdoc
- Include examples in documentation
- Document invariants and assumptions
- Use `//` for inline comments

## Git Commit Messages
Format: `type: description`

Types:
- `feat`: New feature
- `fix`: Bug fix
- `refactor`: Code refactoring
- `test`: Test additions/changes
- `docs`: Documentation changes
- `perf`: Performance improvements

## Dependencies
- Minimize external dependencies
- Prefer well-maintained, popular crates
- Document why each dependency is needed
- Regular dependency audits with `cargo audit`