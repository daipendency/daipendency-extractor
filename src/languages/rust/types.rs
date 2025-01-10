use crate::types::Symbol;

#[derive(Debug)]
pub enum RustSymbol {
    Symbol(Symbol),
    Module {
        name: String,
        content: Vec<RustSymbol>,
    },
}
