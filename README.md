# laibrary

**laibrary** provides AI coding assistants with public APIs and documentation from dependencies.

## Usage

To run the CLI tool:

```sh
cargo run -- /path/to/library
```

You can also use it as a library:

```rust
use laibrary::generate_library_api;
use std::path::Path;

let path = Path::new("/path/to/library");
match generate_library_api(&path) {
    Ok(output) => println!("{}", output),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Supported Languages

- [Rust](src/languages/rust/README.md).
