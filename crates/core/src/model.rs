use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    TypeScript,
    TypeScriptReact, // .tsx
    Swift,
    Go,
    Python,
    Unknown(String), // extension seen but not parseable
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Class,
    Type,
    Interface,
    Hook,
    Enum,
    Module,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeKind {
    Import,
    Call,
    TypeRef,
    ReExport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolNode {
    pub id: String,          // "file_path::symbol_name" (D-01)
    pub name: String,
    pub kind: SymbolKind,
    pub file_path: String,
    pub language: Language,
    pub line_start: u32,
    pub line_end: u32,
    pub is_exported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolEdge {
    pub source_id: String,
    pub target_id: String,
    pub kind: EdgeKind,
    pub source_location: u32, // line number of the reference (D-04)
}
