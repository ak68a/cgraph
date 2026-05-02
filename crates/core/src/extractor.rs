use std::path::Path;
use thiserror::Error;
use crate::model::{Language, SymbolNode, SymbolEdge};

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("I/O error reading {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Parse produced ERROR nodes in {path} at line {line}")]
    PartialParse { path: String, line: u32 },
}

#[derive(Debug)]
pub struct ExtractionResult {
    pub nodes: Vec<SymbolNode>,
    pub edges: Vec<SymbolEdge>,
    pub errors: Vec<ParseError>,
}

pub trait Extractor {
    /// The language this extractor handles.
    fn language(&self) -> Language;

    /// Returns true if this extractor can handle the given file path.
    fn can_handle(&self, path: &Path) -> bool;

    /// Extract graph fragments from the given file.
    /// `source` is the full text content — file I/O is the caller's responsibility (D-18).
    fn extract(&self, path: &Path, source: &str) -> ExtractionResult;
}
