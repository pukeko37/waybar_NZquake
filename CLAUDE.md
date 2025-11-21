# Claude Code Style Guide for waybar_nzquake

This document defines the coding standards and principles for this Rust project.

## Core Principles

### YAGNI (You Aren't Gonna Need It)
- Only implement what is needed NOW
- Do not add features, fields, or code for potential future use
- Remove any code that is not actively used

### Dead Code is Forbidden
- **NEVER use `#[allow(dead_code)]`** - it masks the symptom instead of fixing the problem
- If the compiler warns about dead code, DELETE IT
- Unused fields, functions, imports, or modules must be removed immediately
- The only exception is test-only code properly marked with `#[cfg(test)]`

### Type Safety and Domain Modeling
- Use Rust's type system to encode business rules and constraints
- Prefer newtype patterns for domain concepts (e.g., `Temperature`, `Magnitude`)
- Make illegal states unrepresentable through types
- Use `Result` and `Option` appropriately; avoid panics in production code

### Error Handling
- Use `anyhow` for application errors with context
- Use `Result` return types for fallible operations
- Provide meaningful error messages with context
- Never use `.unwrap()` except in tests or when immediately followed by error handling

### Documentation
- Every public module should have `//!` module-level documentation
- Public functions should have `///` doc comments
- Document the "why" not the "what" when the code is non-obvious
- Keep comments up-to-date with code changes

### Code Organization
- Keep modules focused and cohesive
- Use clear, descriptive names for functions and variables
- Avoid deeply nested code; prefer early returns
- Functions should do one thing well

### Dependencies
- Minimize external dependencies
- Prefer standard library when possible
- Avoid heavyweight crates for simple tasks
- Document why each dependency is needed

### Testing
- Write tests for business logic and domain models
- Use descriptive test names that explain what is being tested
- Integration tests should be skippable in CI with `std::env::var("CI")`
- Mock external services appropriately

### Performance
- Prefer simplicity over premature optimization
- Use `cargo build --release` optimizations for production
- Profile before optimizing
- Document any non-obvious performance considerations

## Rust-Specific Guidelines

### Ownership and Borrowing
- Prefer borrowing over cloning when possible
- Use references (`&T`) for read-only access
- Use mutable references (`&mut T`) sparingly
- Clone only when necessary for ownership

### Formatting
- Use `rustfmt` with default settings
- Run `cargo fmt` before committing
- Maximum line length: 100 characters (rustfmt default)

### Linting
- Code must compile without warnings
- Run `cargo clippy` and address all warnings
- Do not suppress clippy lints without good reason and documentation

### Imports
- Group imports: std, external crates, internal modules
- Remove unused imports immediately
- Use explicit imports rather than glob imports except for preludes

### Error Messages
- User-facing errors should be helpful and actionable
- Include context about what operation failed
- Suggest solutions when possible

## Project-Specific Rules

### GeoNet API Integration
- Respect the public API's rate limits
- Handle network errors gracefully
- Deserialize only the fields we actually use from the JSON response

### Waybar Output
- Always output valid JSON
- Provide meaningful error messages in the tooltip
- Keep the status bar text concise

### Location Handling
- Accept latitude,longitude in GeoNet format (e.g., "-41.2865,174.7762")
- Validate coordinates are reasonable for New Zealand
- Warn but don't fail on out-of-range coordinates

## Anti-Patterns to Avoid

❌ Adding fields "just in case" - violates YAGNI
❌ Using `#[allow(dead_code)]` - masks real problems
❌ Leaving TODO comments - implement or file an issue
❌ Commented-out code - use git history instead
❌ Generic variable names like `data`, `result`, `temp` - be specific
❌ Large functions (>50 lines) - break them down
❌ Deeply nested if/match statements - refactor for clarity

## Code Review Checklist

Before committing, verify:
- [ ] Code compiles without warnings
- [ ] All tests pass
- [ ] No dead code or unused imports
- [ ] Documentation is up-to-date
- [ ] Error handling is appropriate
- [ ] Code follows YAGNI principle
- [ ] Type safety is maximized
