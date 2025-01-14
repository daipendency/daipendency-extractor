use crate::types::Symbol;

#[derive(Debug, Clone)]
pub struct RustFile {
    pub doc_comment: Option<String>,
    pub symbols: Vec<RustSymbol>,
}

#[derive(Debug, Clone)]
pub enum RustSymbol {
    Symbol {
        symbol: Symbol,
    },
    Module {
        name: String,
        content: Vec<RustSymbol>,
        doc_comment: Option<String>,
    },
    ModuleDeclaration {
        name: String,
        is_public: bool,
    },
    SymbolReexport {
        name: String,
        source_path: String,
    },
}
