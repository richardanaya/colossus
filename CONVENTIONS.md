# Rust Development Conventions

use "gpt-4o-2024-08-06" as openai model

## Code Simplicity

- **Keep It Simple**: Strive for simplicity in code design. Avoid unnecessary complexity and over-engineering.
- **Readability**: Write code that is easy to read and understand. Use clear and descriptive names for variables, functions, and modules.
- **Minimalism**: Use the least amount of code necessary to achieve functionality. Avoid redundant code and excessive comments.

## Code Structure

- **Modular Design**: Break down code into small, reusable modules. Each module should have a single responsibility.
- **Consistent Formatting**: Follow Rust's standard formatting conventions. Use `rustfmt` to automatically format code.
- **Error Handling**: Use Rust's `Result` and `Option` types for error handling. Handle errors gracefully and provide meaningful error messages.

## Best Practices

- **Use Idiomatic Rust**: Follow Rust's idioms and best practices. Leverage Rust's powerful type system and ownership model.
- **Testing**: Write unit tests for all functions and modules. Use `cargo test` to run tests and ensure code quality.
- **Documentation**: Document code using Rust's documentation comments. Provide clear explanations and examples.

## Performance

- **Efficient Algorithms**: Use efficient algorithms and data structures. Optimize code for performance where necessary.
- **Benchmarking**: Use `cargo bench` to benchmark code and identify performance bottlenecks.
- **Memory Management**: Be mindful of memory usage. Use Rust's ownership and borrowing system to manage memory efficiently.
