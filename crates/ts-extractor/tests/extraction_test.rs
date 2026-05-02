use std::path::Path;
use cgraph_core::Extractor;
use cgraph_ts_extractor::TsExtractor;

#[test]
fn extractor_compiles_queries_without_panic() {
    let _extractor = TsExtractor::new();
}

#[test]
fn can_handle_ts_files() {
    let extractor = TsExtractor::new();
    assert!(extractor.can_handle(Path::new("foo.ts")));
    assert!(extractor.can_handle(Path::new("bar.tsx")));
    assert!(!extractor.can_handle(Path::new("baz.js")));
    assert!(!extractor.can_handle(Path::new("qux.rs")));
}

#[test]
fn extract_returns_no_errors_on_valid_ts() {
    let extractor = TsExtractor::new();
    let source = std::fs::read_to_string("tests/fixtures/schemas.ts")
        .expect("schemas.ts fixture missing");
    let result = extractor.extract(Path::new("tests/fixtures/schemas.ts"), &source);
    assert!(result.errors.is_empty(), "Unexpected parse errors: {:?}", result.errors);
}

#[test]
fn extract_returns_no_errors_on_valid_tsx() {
    let extractor = TsExtractor::new();
    let source = std::fs::read_to_string("tests/fixtures/components.tsx")
        .expect("components.tsx fixture missing");
    let result = extractor.extract(Path::new("tests/fixtures/components.tsx"), &source);
    assert!(result.errors.is_empty(), "Unexpected parse errors: {:?}", result.errors);
}

#[test]
fn grammar_selection_ts_vs_tsx() {
    let extractor = TsExtractor::new();
    // .ts file should parse without errors
    let ts_source = std::fs::read_to_string("tests/fixtures/services.ts")
        .expect("services.ts fixture missing");
    let ts_result = extractor.extract(Path::new("tests/fixtures/services.ts"), &ts_source);
    assert!(ts_result.errors.is_empty());

    // .tsx file should parse without errors (JSX syntax)
    let tsx_source = std::fs::read_to_string("tests/fixtures/components.tsx")
        .expect("components.tsx fixture missing");
    let tsx_result = extractor.extract(Path::new("tests/fixtures/components.tsx"), &tsx_source);
    assert!(tsx_result.errors.is_empty());
}
