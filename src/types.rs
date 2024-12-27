/// Structure representing library information
#[derive(Debug)]
pub struct LibraryInfo {
    /// Name of the library
    pub name: String,
    /// Version of the library
    pub version: String,
    /// Documentation content
    pub documentation: String,
    /// API signatures and types
    pub api: String,
}
