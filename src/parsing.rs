use crate::error::ExtractionError;
use tree_sitter::{Language, Parser};

pub fn get_parser(parser_language: &Language) -> Result<Parser, ExtractionError> {
    let mut parser = Parser::new();
    parser
        .set_language(parser_language)
        .map_err(|e| ExtractionError::Parse(format!("Error setting language for parser: {}", e)))?;
    Ok(parser)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::c_void;

    // Minimal valid language struct matching tree-sitter's TSLanguage
    #[repr(C)]
    struct MinimalLanguage {
        version: u32,
        symbol_count: u32,
        symbol_metadata: &'static [u32],
        parse_actions: &'static [u16],
        lex_modes: &'static [u32],
        symbol_names: &'static [&'static str],
        field_count: u32,
        field_names: &'static [&'static str],
        field_map_slices: &'static [u8],
        field_map_entries: &'static [u16],
        parse_table: &'static [u16],
        lex_fn: Option<unsafe extern "C" fn(*mut c_void, u32, *mut c_void) -> bool>,
    }

    static MINIMAL_LANGUAGE: MinimalLanguage = MinimalLanguage {
        version: 14, // TREE_SITTER_LANGUAGE_VERSION
        symbol_count: 1,
        symbol_metadata: &[0],
        parse_actions: &[0],
        lex_modes: &[0],
        symbol_names: &["root"],
        field_count: 0,
        field_names: &[],
        field_map_slices: &[],
        field_map_entries: &[],
        parse_table: &[0],
        lex_fn: None,
    };

    #[test]
    fn get_parser_valid() {
        let language = unsafe { Language::from_raw(&MINIMAL_LANGUAGE as *const _ as *const _) };

        let result = get_parser(&language);

        assert!(result.is_ok());
    }
}
