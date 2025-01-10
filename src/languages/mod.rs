mod rust;
mod test_helpers;

use crate::analysers::Analyser;
use crate::error::LaibraryError;
use crate::languages::rust::RustAnalyser;

type AnalyserMapping = (&'static str, fn() -> Box<dyn Analyser>);

const LANGUAGES: [AnalyserMapping; 1] = [("rust", || Box::from(RustAnalyser))];

/// Get an analyser for the specified language
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
        let result = get_analyser("rust").unwrap();

        assert_eq!(
            result.get_file_extensions(),
            RustAnalyser.get_file_extensions()
        );
    }

    #[test]
    fn test_get_analyser_unsupported() {
        let unsupported_lang = "brainfuck";

        let result = get_analyser(unsupported_lang);

        assert!(matches!(
            result,
            Err(LaibraryError::UnsupportedLanguage(lang)) if lang == unsupported_lang
        ));
    }
}
