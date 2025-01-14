mod rust;

use crate::analysers::Analyser;
use crate::error::LaibraryError;
use crate::languages::rust::RustAnalyser;

type AnalyserMapping = (&'static str, fn() -> Box<dyn Analyser>);

const LANGUAGES: [AnalyserMapping; 1] = [("rust", || Box::new(RustAnalyser::new()))];

pub fn get_analyser(language: &str) -> Result<Box<dyn Analyser>, LaibraryError> {
    LANGUAGES
        .iter()
        .find(|(name, _)| *name == language)
        .map(|(_, create_analyser)| create_analyser())
        .ok_or_else(|| LaibraryError::UnsupportedLanguage(language.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_analyser_rust() {
        let analyser = get_analyser("rust").unwrap();
        assert_eq!(
            format!("{:?}", analyser.get_parser_language()),
            format!("{:?}", RustAnalyser.get_parser_language())
        );
    }

    #[test]
    fn test_get_analyser_unsupported() {
        let result = get_analyser("unsupported");
        assert!(matches!(result, Err(LaibraryError::UnsupportedLanguage(_))));
    }
}
