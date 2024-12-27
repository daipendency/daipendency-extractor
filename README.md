# laibrary

Generate context for LLMs to work reliably with library APIs.

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
