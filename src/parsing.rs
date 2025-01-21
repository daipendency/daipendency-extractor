use crate::error::LaibraryError;
use tree_sitter::{Language, Parser};

pub fn get_parser(parser_language: &Language) -> Result<Parser, LaibraryError> {
    let mut parser = Parser::new();
    parser
        .set_language(parser_language)
        .map_err(|e| LaibraryError::Parse(format!("Error setting language for parser: {}", e)))?;
    Ok(parser)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter_rust::LANGUAGE;

    #[test]
    fn get_parser_valid() {
        let result = get_parser(&LANGUAGE.into());

        assert!(result.is_ok());
    }
}
