pub mod graph;
pub mod crawl;
pub mod resolve;
pub mod analysis;

pub use graph::CodeGraph;
pub use crawl::{Indexer, IndexerError};
pub use analysis::{DeadCodeResult, DeadCodeEntry, Confidence, CycleResult, blast_radius, transitive_deps, detect_cycles, dead_code};
