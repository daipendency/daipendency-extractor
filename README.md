# laibrary

**laibrary** extracts public APIs and documentation from external libraries, so that AI coding assistants can pass them on as context to the language model.

## Usage

To run the CLI tool:

```sh
cargo run -- /path/to/library
```

You can also use **laibrary** as a library:

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
- TypeScript (planned):
  - JSDoc and TSDoc comments (`/** ... */`).
  - Public API declarations (`.d.ts` files).
  - Project documentation (`.md` and `.txt` files).
