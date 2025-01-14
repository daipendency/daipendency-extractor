#![cfg(test)]

use crate::analysers::Analyser;
use crate::languages::rust::analyser::RustAnalyser;
use crate::languages::rust::types::RustSymbol;
use crate::types::Symbol;
use tree_sitter::Parser;

pub fn setup_parser() -> Parser {
    let mut parser = Parser::new();
    let analyser = RustAnalyser::new();
    parser
        .set_language(&analyser.get_parser_language())
        .unwrap();
    parser
}

pub fn get_inner_module<'a>(path: &str, symbols: &'a [RustSymbol]) -> Option<&'a [RustSymbol]> {
    let parts: Vec<&str> = path.split("::").collect();
    let mut current_symbols = symbols;

    for part in parts {
        match current_symbols.iter().find_map(|symbol| {
            if let RustSymbol::Module { name, content } = symbol {
                if name == part {
                    Some(content)
                } else {
                    None
                }
            } else {
                None
            }
        }) {
            Some(next_symbols) => current_symbols = next_symbols,
            None => return None,
        }
    }

    Some(current_symbols)
}

pub fn get_rust_symbol<'a>(symbols: &'a [RustSymbol], name: &str) -> Option<&'a Symbol> {
    symbols.iter().find_map(|s| {
        if let RustSymbol::Symbol { symbol, .. } = s {
            if symbol.name == name {
                Some(symbol)
            } else {
                None
            }
        } else {
            None
        }
    })
}
