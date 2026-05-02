pub mod queries;
pub mod symbols;
pub mod edges;
pub mod classify;

use std::path::Path;
use tree_sitter::{Parser, Query, Language as TsLanguage};
use tree_sitter_typescript::{LANGUAGE_TYPESCRIPT, LANGUAGE_TSX};
use cgraph_core::{
    Extractor, ExtractionResult, ParseError,
    Language, SymbolNode, SymbolEdge,
};

pub struct TsExtractor {
    ts_lang: TsLanguage,
    tsx_lang: TsLanguage,
    // Queries compiled against TSX grammar (superset of TS - Pitfall 2 from research)
    symbol_query: Query,
    import_query: Query,
    call_query: Query,
    type_ref_query: Query,
    reexport_query: Query,
}

impl TsExtractor {
    pub fn new() -> Self {
        let tsx_lang: TsLanguage = LANGUAGE_TSX.into();
        let ts_lang: TsLanguage = LANGUAGE_TYPESCRIPT.into();

        // Compile all queries against TSX grammar (superset of TypeScript, Pitfall 2).
        // This means the same queries work for both .ts and .tsx parse trees.
        let symbol_query = Query::new(&tsx_lang, queries::SYMBOL_QUERY_SRC)
            .expect("symbol query compilation failed");
        let import_query = Query::new(&tsx_lang, queries::IMPORT_QUERY_SRC)
            .expect("import query compilation failed");
        let call_query = Query::new(&tsx_lang, queries::CALL_QUERY_SRC)
            .expect("call query compilation failed");
        let type_ref_query = Query::new(&tsx_lang, queries::TYPE_REF_QUERY_SRC)
            .expect("type_ref query compilation failed");
        let reexport_query = Query::new(&tsx_lang, queries::REEXPORT_QUERY_SRC)
            .expect("reexport query compilation failed");

        Self {
            ts_lang,
            tsx_lang,
            symbol_query,
            import_query,
            call_query,
            type_ref_query,
            reexport_query,
        }
    }
}

impl Default for TsExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Extractor for TsExtractor {
    fn language(&self) -> Language {
        Language::TypeScript
    }

    fn can_handle(&self, path: &Path) -> bool {
        matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("ts") | Some("tsx")
        )
    }

    fn extract(&self, path: &Path, source: &str) -> ExtractionResult {
        let mut errors = Vec::new();

        // Select grammar based on extension (D-36, Pitfall 1)
        let is_tsx = path.extension().map_or(false, |e| e == "tsx");
        let lang = if is_tsx { &self.tsx_lang } else { &self.ts_lang };

        // Parse source text
        let mut parser = Parser::new();
        parser.set_language(lang).expect("grammar load");
        let tree = match parser.parse(source, None) {
            Some(t) => t,
            None => {
                errors.push(ParseError::PartialParse {
                    path: path.display().to_string(),
                    line: 0,
                });
                return ExtractionResult { nodes: Vec::new(), edges: Vec::new(), errors };
            }
        };

        let root = tree.root_node();

        // Record partial parse errors but continue extraction (D-14)
        if root.has_error() {
            errors.push(ParseError::PartialParse {
                path: path.display().to_string(),
                line: root.start_position().row as u32,
            });
        }

        let file_path_str = path.display().to_string();
        let cg_language = if is_tsx { Language::TypeScriptReact } else { Language::TypeScript };

        // Pass 1: Extract symbols
        let nodes = symbols::extract_symbols(
            root,
            source,
            &file_path_str,
            cg_language.clone(),
            &self.symbol_query,
        );

        // Pass 2: Extract edges
        let edges = edges::extract_edges(
            root,
            source,
            &file_path_str,
            &self.import_query,
            &self.call_query,
            &self.type_ref_query,
            &self.reexport_query,
        );

        ExtractionResult { nodes, edges, errors }
    }
}
