pub mod queries;
pub mod symbols;
pub mod edges;
pub mod classify;

use std::path::Path;
use tree_sitter::{Node, Parser, Query, Language as TsLanguage};
use tree_sitter_typescript::LANGUAGE_TSX;
use cgraph_core::{
    Extractor, ExtractionResult, ParseError,
    Language,
};

pub struct TsExtractor {
    tsx_lang: TsLanguage,
    // Queries compiled against TSX grammar (superset of TS - Pitfall 2 from research)
    symbol_query: Query,
    import_query: Query,
    call_query: Query,
    type_ref_query: Query,
    type_ann_query: Query,
    reexport_query: Query,
    member_ref_query: Query,
}

impl TsExtractor {
    pub fn new() -> Self {
        let tsx_lang: TsLanguage = LANGUAGE_TSX.into();

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
        let type_ann_query = Query::new(&tsx_lang, queries::TYPE_ANN_QUERY_SRC)
            .expect("type_ann query compilation failed");
        let reexport_query = Query::new(&tsx_lang, queries::REEXPORT_QUERY_SRC)
            .expect("reexport query compilation failed");
        let member_ref_query = Query::new(&tsx_lang, queries::MEMBER_REF_QUERY_SRC)
            .expect("member_ref query compilation failed");

        Self {
            tsx_lang,
            symbol_query,
            import_query,
            call_query,
            type_ref_query,
            type_ann_query,
            reexport_query,
            member_ref_query,
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

        // Always use TSX grammar for both .ts and .tsx files.
        // Queries are compiled against tsx_lang; tree-sitter's query engine requires that
        // the node's language matches the query's language. TSX is a strict superset of
        // TypeScript so all valid .ts syntax parses correctly under the TSX grammar.
        // Using ts_lang for .ts files causes query matches to return zero results. (Bug fix)
        let is_tsx = path.extension().map_or(false, |e| e == "tsx");
        let lang = &self.tsx_lang;

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
            let error_line = find_first_error_line(root);
            errors.push(ParseError::PartialParse {
                path: path.display().to_string(),
                line: error_line,
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
            &self.type_ann_query,
            &self.reexport_query,
            &self.member_ref_query,
        );

        ExtractionResult { nodes, edges, errors }
    }
}

/// Walk the tree to find the first ERROR or MISSING node and return its 1-based line number.
/// Returns 0 if no error node is found (should not happen when `root.has_error()` is true).
fn find_first_error_line(node: Node) -> u32 {
    if node.is_error() || node.is_missing() {
        return node.start_position().row as u32 + 1;
    }
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let line = find_first_error_line(cursor.node());
            if line > 0 {
                return line;
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    0
}
