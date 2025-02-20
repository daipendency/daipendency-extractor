use crate::ExtractionError;
use std::ops::Range;
use tree_sitter::{Node, Parser, Query, QueryCursor, QueryMatches, Tree};

/// A parsed source file with its tree-sitter parse tree and original source code.
pub struct ParsedFile<'a> {
    root_tree: Tree,
    source_code: &'a str,
}

impl<'a> ParsedFile<'a> {
    /// Parse source code into a tree-sitter parse tree.
    ///
    /// # Parameters
    /// * `source_code` - The source code to parse
    /// * `parser` - A mutable reference to a configured tree-sitter parser
    ///
    /// # Returns
    /// A new `ParsedFile` instance or an `ExtractionError` if parsing fails
    pub fn parse(source_code: &'a str, parser: &mut Parser) -> Result<Self, ExtractionError> {
        let root_tree = parser
            .parse(source_code, None)
            .ok_or_else(|| ExtractionError::Malformed("Failed to parse source file".to_string()))?;

        if root_tree.root_node().has_error() {
            return Err(ExtractionError::Malformed(
                "Failed to parse source file".to_string(),
            ));
        }

        Ok(Self {
            root_tree,
            source_code,
        })
    }

    /// Return the root node of the parse tree.
    ///
    /// # Returns
    /// The root node of the parsed source file
    pub fn root_node(&'a self) -> Node<'a> {
        self.root_tree.root_node()
    }

    /// Return a tree-sitter node's text content.
    ///
    /// # Parameters
    /// * `node` - The tree-sitter node to render
    ///
    /// # Returns
    /// The text content of the node or an `ExtractionError` if rendering fails
    pub fn render_node(&self, node: Node) -> Result<String, ExtractionError> {
        node.utf8_text(self.source_code.as_bytes())
            .map(|s| s.to_string())
            .map_err(|_| ExtractionError::Malformed("Failed to render node".to_string()))
    }

    /// Return text content from a byte range in the source code.
    ///
    /// # Parameters
    /// * `range` - The byte range to extract from the source code
    ///
    /// # Returns
    /// The text content within the specified range
    pub fn render(&self, range: Range<usize>) -> String {
        self.source_code[range].to_string()
    }

    /// Create a new tree-sitter query from a query string.
    ///
    /// # Parameters
    /// * `query` - The query string in tree-sitter query language
    ///
    /// # Returns
    /// A compiled query or an `ExtractionError` if query creation fails
    pub fn make_query(&self, query: &str) -> Result<Query, ExtractionError> {
        Query::new(&self.root_tree.language(), query)
            .map_err(|_| ExtractionError::Malformed("Failed to create query".to_string()))
    }

    /// Execute a tree-sitter query on a specific node.
    ///
    /// # Parameters
    /// * `query` - The compiled tree-sitter query to execute
    /// * `node` - The node to search within
    /// * `cursor` - A mutable reference to a query cursor for tracking query state
    ///
    /// # Returns
    /// An iterator over the query matches
    pub fn exec_query<'b>(
        &'b self,
        query: &'b Query,
        node: Node<'b>,
        cursor: &'b mut QueryCursor,
    ) -> QueryMatches<'b, 'b, &'b [u8], &'b [u8]> {
        cursor.matches(query, node, self.source_code.as_bytes())
    }
}
