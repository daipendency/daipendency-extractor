# laibrary

**laibrary** extracts public APIs and documentation from external libraries and outputs them in a structured pseudo-XML format suitable for use with language models.

## Usage

To run the CLI tool:

```sh
cargo run -- /path/to/rust-crate
```

This will output the public API and documentation of the specified Rust crate.

## Library Integration

You can also use **laibrary** as a library in your Rust projects:

```rust
use laibrary::generate_library_api;
use std::path::Path;

let path = Path::new("/path/to/rust-crate");
match generate_library_api(&path) {
    Ok(output) => println!("{}", output),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Input Sources

### Rust

- Doc comments (`///` and `//!`) with Markdown support.
- The `README.md` file of the crate.
- Public API signatures and types.
- Additional Markdown files via `#[doc = include_str!("path/to/doc.md")]`.

### TypeScript

- JSDoc and TSDoc comments (`/** ... */`).
- The `README.md` file of the project.
- Public API declarations (`.d.ts` files).
- Additional documentation files (`.md` and `.txt` files).

## Output format

```xml
<library name="the-library" version="1.0.0">
    <documentation>
        # The Library

        This is a library that does something.
    </documentation>
    <api>
        <![CDATA[
        /// Entry point for the laibrary CLI tool.
        fn main() -> Result<(), Box<dyn Error>>
        ]]>
    </api>
    <examples>
        <example>
            <![CDATA[
            // Example usage of the library.
            ]]>
        </example>
    </examples>
</library>
```

## Future Plans

Whilst currently supporting Rust only, the codebase is designed to be modular and extensible. Support for additional languages like TypeScript is planned.
