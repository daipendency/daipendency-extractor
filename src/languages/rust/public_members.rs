use std::fmt;

/// Represents different types of public API members in Rust
#[derive(Debug, Clone, PartialEq)]
pub enum RustPublicMember {
    Function(Function),
    Struct(Struct),
    Enum(Enum),
    Trait(Trait),
    Macro(Macro),
}

/// Represents a public function
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<String>,
    pub doc_comment: Option<String>,
    pub type_parameters: Vec<TypeParameter>,
    pub where_clause: Option<String>,
}

/// Represents a function parameter
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub type_name: String,
}

/// Represents a type parameter
#[derive(Debug, Clone, PartialEq)]
pub struct TypeParameter {
    pub name: String,
    pub bounds: Vec<String>,
}

/// Represents a public struct
#[derive(Debug, Clone, PartialEq)]
pub struct Struct {
    pub name: String,
    pub fields: Vec<StructField>,
    pub doc_comment: Option<String>,
    pub type_parameters: Vec<TypeParameter>,
}

/// Represents a struct field
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: String,
    pub type_name: String,
    pub doc_comment: Option<String>,
}

/// Represents a public enum
#[derive(Debug, Clone, PartialEq)]
pub struct Enum {
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub doc_comment: Option<String>,
    pub type_parameters: Vec<TypeParameter>,
}

/// Represents an enum variant
#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<VariantField>,
    pub doc_comment: Option<String>,
}

/// Represents an enum variant field
#[derive(Debug, Clone, PartialEq)]
pub struct VariantField {
    pub name: Option<String>,
    pub type_name: String,
}

/// Represents a public trait
#[derive(Debug, Clone, PartialEq)]
pub struct Trait {
    pub name: String,
    pub methods: Vec<Function>,
    pub doc_comment: Option<String>,
    pub type_parameters: Vec<TypeParameter>,
}

/// Represents a public macro
#[derive(Debug, Clone, PartialEq)]
pub struct Macro {
    pub name: String,
    pub definition: String,
    pub doc_comment: Option<String>,
}

impl fmt::Display for RustPublicMember {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RustPublicMember::Function(func) => write!(f, "{}", func),
            RustPublicMember::Struct(structure) => write!(f, "{}", structure),
            RustPublicMember::Enum(enumerate) => write!(f, "{}", enumerate),
            RustPublicMember::Trait(trait_) => write!(f, "{}", trait_),
            RustPublicMember::Macro(macro_) => write!(f, "{}", macro_),
        }
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format type parameters
        let type_params = if !self.type_parameters.is_empty() {
            format!("<{}>", self.type_parameters.iter()
                .map(|tp| tp.to_string())
                .collect::<Vec<_>>()
                .join(", "))
        } else {
            String::new()
        };

        // Format parameters
        let params = self.parameters.iter()
            .map(|p| format!("{}: {}", p.name, p.type_name))
            .collect::<Vec<_>>()
            .join(", ");

        // Format return type
        let return_type = self.return_type.as_ref()
            .map(|rt| format!(" -> {}", rt))
            .unwrap_or_default();

        // Format where clause
        let where_clause = self.where_clause.as_ref()
            .map(|wc| format!(" where {}", wc))
            .unwrap_or_default();

        write!(f, "pub fn {}{}({}){}{};", 
            self.name, type_params, params, return_type, where_clause)
    }
}

impl fmt::Display for TypeParameter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.bounds.is_empty() {
            write!(f, "{}", self.name)
        } else {
            write!(f, "{}: {}", self.name, self.bounds.join(" + "))
        }
    }
}

impl fmt::Display for Struct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format type parameters
        let type_params = if !self.type_parameters.is_empty() {
            format!("<{}>", self.type_parameters.iter()
                .map(|tp| tp.to_string())
                .collect::<Vec<_>>()
                .join(", "))
        } else {
            String::new()
        };

        // Format fields
        let fields = self.fields.iter()
            .map(|field| {
                if let Some(doc) = &field.doc_comment {
                    format!("{}\n    pub {}: {}", doc.trim(), field.name, field.type_name)
                } else {
                    format!("    pub {}: {}", field.name, field.type_name)
                }
            })
            .collect::<Vec<_>>()
            .join(",\n");

        write!(f, "pub struct {}{} {{\n{}\n}}", self.name, type_params, fields)
    }
}

impl fmt::Display for Enum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format type parameters
        let type_params = if !self.type_parameters.is_empty() {
            format!("<{}>", self.type_parameters.iter()
                .map(|tp| tp.to_string())
                .collect::<Vec<_>>()
                .join(", "))
        } else {
            String::new()
        };

        // Format variants
        let variants = self.variants.iter()
            .map(|variant| {
                let fields = match variant.fields.len() {
                    0 => String::new(),
                    _ => format!("({})", variant.fields.iter()
                        .map(|field| {
                            if let Some(name) = &field.name {
                                format!("{}: {}", name, field.type_name)
                            } else {
                                field.type_name.clone()
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(", "))
                };
                
                if let Some(doc) = &variant.doc_comment {
                    format!("{}\n    {}{}", doc.trim(), variant.name, fields)
                } else {
                    format!("    {}{}", variant.name, fields)
                }
            })
            .collect::<Vec<_>>()
            .join(",\n");

        write!(f, "pub enum {}{} {{\n{}\n}}", self.name, type_params, variants)
    }
}

impl fmt::Display for Trait {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format type parameters
        let type_params = if !self.type_parameters.is_empty() {
            format!("<{}>", self.type_parameters.iter()
                .map(|tp| tp.to_string())
                .collect::<Vec<_>>()
                .join(", "))
        } else {
            String::new()
        };

        // Format methods
        let methods = self.methods.iter()
            .map(|method| method.to_string())
            .collect::<Vec<_>>()
            .join("\n    ");

        write!(f, "pub trait {}{} {{\n    {}\n}}", self.name, type_params, methods)
    }
}

impl fmt::Display for Macro {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(doc) = &self.doc_comment {
            write!(f, "{}\n{}", doc.trim(), self.definition)
        } else {
            write!(f, "{}", self.definition)
        }
    }
}

impl From<String> for RustPublicMember {
    fn from(s: String) -> Self {
        // This is a fallback for non-function items
        RustPublicMember::Macro(Macro {
            name: String::new(),
            definition: s,
            doc_comment: None,
        })
    }
}
