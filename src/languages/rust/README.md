# Laibrary Rust Support

## Input Sources

- Doc comments (`///` and `//!`) with Markdown support
- The `README.md` file of the crate
- Public API signatures and types
- Additional Markdown files via `#[doc = include_str!("path/to/doc.md")]`

## Output Format

```xml
<library name="the-library" version="1.0.0">
    <documentation>
        # The Library
        This is a library that does something.
    </documentation>
    <api>
        <![CDATA[
        /// A public function in the library
        pub fn do_something() -> Result<(), Error>
        ]]>
    </api>
    <examples>
        <example>
            <![CDATA[
            let result = do_something()?;
            ]]>
        </example>
    </examples>
</library>
```
