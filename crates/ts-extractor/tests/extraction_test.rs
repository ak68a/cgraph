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

// --- Symbol Extraction Tests (PARS-01) ---

#[test]
fn exported_functions_extracted() {
    let extractor = TsExtractor::new();
    let source = std::fs::read_to_string("tests/fixtures/services.ts")
        .expect("services.ts fixture missing");
    let result = extractor.extract(Path::new("tests/fixtures/services.ts"), &source);

    // fetchUser is an exported function
    let fetch_user = result.nodes.iter().find(|n| n.name == "fetchUser");
    assert!(fetch_user.is_some(), "fetchUser not found in nodes");
    let fu = fetch_user.unwrap();
    assert_eq!(fu.kind, cgraph_core::SymbolKind::Function);
    assert!(fu.is_exported);
    assert_eq!(fu.id, "tests/fixtures/services.ts::fetchUser");
}

#[test]
fn exported_types_extracted() {
    let extractor = TsExtractor::new();
    let source = std::fs::read_to_string("tests/fixtures/schemas.ts")
        .expect("schemas.ts fixture missing");
    let result = extractor.extract(Path::new("tests/fixtures/schemas.ts"), &source);

    // UserType is an exported interface
    let user_type = result.nodes.iter().find(|n| n.name == "UserType");
    assert!(user_type.is_some(), "UserType interface not found");
    assert_eq!(user_type.unwrap().kind, cgraph_core::SymbolKind::Interface);

    // UserRole is an exported type alias
    let user_role = result.nodes.iter().find(|n| n.name == "UserRole");
    assert!(user_role.is_some(), "UserRole type not found");
    assert_eq!(user_role.unwrap().kind, cgraph_core::SymbolKind::Type);

    // Permission is an exported enum
    let permission = result.nodes.iter().find(|n| n.name == "Permission");
    assert!(permission.is_some(), "Permission enum not found");
    assert_eq!(permission.unwrap().kind, cgraph_core::SymbolKind::Enum);

    // ValidationError is an exported class
    let val_err = result.nodes.iter().find(|n| n.name == "ValidationError");
    assert!(val_err.is_some(), "ValidationError class not found");
    assert_eq!(val_err.unwrap().kind, cgraph_core::SymbolKind::Class);
}

#[test]
fn hook_detection() {
    let extractor = TsExtractor::new();
    let source = std::fs::read_to_string("tests/fixtures/hooks.ts")
        .expect("hooks.ts fixture missing");
    let result = extractor.extract(Path::new("tests/fixtures/hooks.ts"), &source);

    let use_current = result.nodes.iter().find(|n| n.name == "useCurrentUser");
    assert!(use_current.is_some(), "useCurrentUser not found");
    assert_eq!(use_current.unwrap().kind, cgraph_core::SymbolKind::Hook);

    let use_toggle = result.nodes.iter().find(|n| n.name == "useToggle");
    assert!(use_toggle.is_some(), "useToggle not found");
    assert_eq!(use_toggle.unwrap().kind, cgraph_core::SymbolKind::Hook);
}

#[test]
fn tsx_components_extracted() {
    let extractor = TsExtractor::new();
    let source = std::fs::read_to_string("tests/fixtures/components.tsx")
        .expect("components.tsx fixture missing");
    let result = extractor.extract(Path::new("tests/fixtures/components.tsx"), &source);

    let profile = result.nodes.iter().find(|n| n.name == "ProfileCard");
    assert!(profile.is_some(), "ProfileCard not found");
    let p = profile.unwrap();
    assert_eq!(p.kind, cgraph_core::SymbolKind::Function);
    assert!(p.is_exported);
}

#[test]
fn exported_enums_extracted() {
    let extractor = TsExtractor::new();
    let source = std::fs::read_to_string("tests/fixtures/enums.ts")
        .expect("enums.ts fixture missing");
    let result = extractor.extract(Path::new("tests/fixtures/enums.ts"), &source);

    let direction = result.nodes.iter().find(|n| n.name == "Direction");
    assert!(direction.is_some(), "Direction enum not found");
    assert_eq!(direction.unwrap().kind, cgraph_core::SymbolKind::Enum);

    let status = result.nodes.iter().find(|n| n.name == "Status");
    assert!(status.is_some(), "Status enum not found");
    assert_eq!(status.unwrap().kind, cgraph_core::SymbolKind::Enum);
}

#[test]
fn non_exported_functions_captured() {
    let extractor = TsExtractor::new();
    let source = std::fs::read_to_string("tests/fixtures/services.ts")
        .expect("services.ts fixture missing");
    let result = extractor.extract(Path::new("tests/fixtures/services.ts"), &source);

    // fetchFromDb is a non-exported function
    let fetch_db = result.nodes.iter().find(|n| n.name == "fetchFromDb");
    assert!(fetch_db.is_some(), "fetchFromDb not found (non-exported function)");
    assert!(!fetch_db.unwrap().is_exported);
}

#[test]
fn symbol_id_format() {
    let extractor = TsExtractor::new();
    let source = std::fs::read_to_string("tests/fixtures/schemas.ts")
        .expect("schemas.ts fixture missing");
    let result = extractor.extract(Path::new("tests/fixtures/schemas.ts"), &source);

    // All IDs should be file_path::symbol_name format (D-01)
    for node in &result.nodes {
        assert!(
            node.id.contains("::"),
            "Symbol ID '{}' does not contain '::'", node.id
        );
        assert!(
            node.id.starts_with("tests/fixtures/schemas.ts::"),
            "Symbol ID '{}' does not start with file path", node.id
        );
    }
}

#[test]
fn exported_classes_extracted() {
    let extractor = TsExtractor::new();
    let source = std::fs::read_to_string("tests/fixtures/services.ts")
        .expect("services.ts fixture missing");
    let result = extractor.extract(Path::new("tests/fixtures/services.ts"), &source);

    let user_repo = result.nodes.iter().find(|n| n.name == "UserRepository");
    assert!(user_repo.is_some(), "UserRepository class not found");
    assert_eq!(user_repo.unwrap().kind, cgraph_core::SymbolKind::Class);
    assert!(user_repo.unwrap().is_exported);

    let user_service = result.nodes.iter().find(|n| n.name == "UserService");
    assert!(user_service.is_some(), "UserService class not found");
    assert_eq!(user_service.unwrap().kind, cgraph_core::SymbolKind::Class);
    assert!(user_service.unwrap().is_exported);
}
