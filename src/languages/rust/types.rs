use crate::types::Symbol;

#[derive(Debug, Clone)]
pub enum RustSymbol {
    Symbol {
        symbol: Symbol,
    },
    Module {
        name: String,
        content: Vec<RustSymbol>,
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
