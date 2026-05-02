pub mod model;
pub mod detect;
pub mod extractor;

// Re-export top-level types for ergonomic imports
pub use model::{Language, SymbolKind, EdgeKind, SymbolNode, SymbolEdge};
pub use extractor::{Extractor, ExtractionResult, ParseError};
pub use detect::{detect_language, scan_directory, DetectionResult};
