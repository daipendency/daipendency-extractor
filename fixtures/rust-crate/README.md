# rust-crate

Demonstrates Rust library features through data processing and text formatting.

```rust
use rust_crate::text::{Format, TextFormatter};

let fmt = TextFormatter::new(Format::Capitalise);
assert_eq!(fmt.format("hello world")?, "Hello World");
```
