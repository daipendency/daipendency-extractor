use thiserror::Error;

/// Metadata about a library.
///
/// The metadata is typically extracted from a library's manifest file (e.g., `package.json`, `Cargo.toml`).
#[derive(Debug)]
pub struct LibraryMetadata<EntryPoint> {
    /// The name of the library as specified in its manifest
    pub name: String,

    /// The version of the library, if specified in its manifest
    pub version: Option<String>,

    /// Documentation string for the library, typically extracted from its manifest or documentation files
    pub documentation: String,

    /// The entry point(s) for the library.
    ///
    /// Whilst this is typically a single path (e.g. Rust's `src/lib.rs`), some languages/frameworks
    /// may have multiple entry points, such as TypeScript's `exports` directive in `package.json`.
    pub entry_point: EntryPoint,
}

#[derive(Error, Debug)]
pub enum LibraryMetadataError {
    #[error(transparent)]
    MissingManifest(#[from] std::io::Error),
    #[error("{0}")]
    MalformedManifest(String),
}
