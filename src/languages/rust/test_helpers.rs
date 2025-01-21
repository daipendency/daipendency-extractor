#![cfg(test)]

use crate::analysers::Analyser;
use crate::languages::rust::analyser::RustAnalyser;
use tree_sitter::Parser;

pub fn setup_parser() -> Parser {
    let mut parser = Parser::new();
    let analyser = RustAnalyser::new();
    parser
        .set_language(&analyser.get_parser_language())
        .unwrap();
    parser
}
