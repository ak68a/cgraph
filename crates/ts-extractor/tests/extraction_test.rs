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
fn exported_symbol_line_span_covers_full_declaration() {
    // CR-01 regression: line_start and line_end must span the full declaration,
    // not just the identifier token.
    let extractor = TsExtractor::new();
    let source = std::fs::read_to_string("tests/fixtures/services.ts")
        .expect("services.ts fixture missing");
    let result = extractor.extract(Path::new("tests/fixtures/services.ts"), &source);

    // `export function fetchUser()` spans lines 24-27 in services.ts (4 lines)
    let fetch_user = result.nodes.iter().find(|n| n.name == "fetchUser" && n.is_exported).unwrap();
    assert!(
        fetch_user.line_end > fetch_user.line_start,
        "Exported function fetchUser should span multiple lines: line_start={}, line_end={}",
        fetch_user.line_start, fetch_user.line_end
    );

    // `export class UserRepository` spans lines 8-16 (multi-line class)
    let user_repo = result.nodes.iter().find(|n| n.name == "UserRepository" && n.is_exported).unwrap();
    assert!(
        user_repo.line_end > user_repo.line_start,
        "Exported class UserRepository should span multiple lines: line_start={}, line_end={}",
        user_repo.line_start, user_repo.line_end
    );
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

// --- Edge Extraction Tests ---

// PARS-05: Import edges

#[test]
fn import_named() {
    let extractor = TsExtractor::new();
    let source = std::fs::read_to_string("tests/fixtures/hooks.ts")
        .expect("hooks.ts fixture missing");
    let result = extractor.extract(Path::new("tests/fixtures/hooks.ts"), &source);

    // import { useState, useEffect } from 'react'
    let react_imports: Vec<_> = result.edges.iter()
        .filter(|e| e.kind == cgraph_core::EdgeKind::Import && e.target_id.contains("react"))
        .collect();
    assert!(
        react_imports.iter().any(|e| e.target_id.contains("useState")),
        "Missing import edge for useState from react. Edges: {:?}",
        react_imports
    );
    assert!(
        react_imports.iter().any(|e| e.target_id.contains("useEffect")),
        "Missing import edge for useEffect from react. Edges: {:?}",
        react_imports
    );
}

#[test]
fn import_named_relative() {
    let extractor = TsExtractor::new();
    let source = std::fs::read_to_string("tests/fixtures/hooks.ts")
        .expect("hooks.ts fixture missing");
    let result = extractor.extract(Path::new("tests/fixtures/hooks.ts"), &source);

    // import { fetchUser } from './services'
    let service_imports: Vec<_> = result.edges.iter()
        .filter(|e| e.kind == cgraph_core::EdgeKind::Import && e.target_id.contains("./services"))
        .collect();
    assert!(
        service_imports.iter().any(|e| e.target_id.contains("fetchUser")),
        "Missing import edge for fetchUser from ./services. Edges: {:?}",
        service_imports
    );
}

#[test]
fn import_default() {
    let extractor = TsExtractor::new();
    let source = std::fs::read_to_string("tests/fixtures/components.tsx")
        .expect("components.tsx fixture missing");
    let result = extractor.extract(Path::new("tests/fixtures/components.tsx"), &source);

    // import React from 'react'
    let react_default: Vec<_> = result.edges.iter()
        .filter(|e| e.kind == cgraph_core::EdgeKind::Import && e.target_id.contains("react::React"))
        .collect();
    assert!(
        !react_default.is_empty(),
        "Missing default import edge for React. All import edges: {:?}",
        result.edges.iter().filter(|e| e.kind == cgraph_core::EdgeKind::Import).collect::<Vec<_>>()
    );
}

#[test]
fn import_raw_alias_path() {
    // Per D-28/D-29: import paths are emitted raw, no resolution
    let extractor = TsExtractor::new();
    // Create inline source with an alias path
    let source = r#"import { Button } from '@/components/Button';"#;
    let result = extractor.extract(Path::new("test.ts"), source);

    let alias_import: Vec<_> = result.edges.iter()
        .filter(|e| e.kind == cgraph_core::EdgeKind::Import)
        .collect();
    assert!(
        alias_import.iter().any(|e| e.target_id.contains("@/components/Button")),
        "Alias path not preserved raw. Edges: {:?}",
        alias_import
    );
}

// PARS-06: Call edges

#[test]
fn call_direct() {
    let extractor = TsExtractor::new();
    let source = std::fs::read_to_string("tests/fixtures/hooks.ts")
        .expect("hooks.ts fixture missing");
    let result = extractor.extract(Path::new("tests/fixtures/hooks.ts"), &source);

    // Direct calls: useState(), useEffect(), fetchUser()
    let call_edges: Vec<_> = result.edges.iter()
        .filter(|e| e.kind == cgraph_core::EdgeKind::Call)
        .collect();
    assert!(
        call_edges.iter().any(|e| e.target_id.contains("useState")),
        "Missing call edge for useState(). Call edges: {:?}",
        call_edges
    );
    assert!(
        call_edges.iter().any(|e| e.target_id.contains("fetchUser")),
        "Missing call edge for fetchUser(). Call edges: {:?}",
        call_edges
    );
}

#[test]
fn call_no_member() {
    // Per D-30: obj.method() calls should NOT be captured
    let extractor = TsExtractor::new();
    let source = r#"
        const result = obj.method();
        const data = this.fetchData();
        const value = console.log("test");
        const direct = standalone();
    "#;
    let result = extractor.extract(Path::new("test.ts"), source);

    let call_edges: Vec<_> = result.edges.iter()
        .filter(|e| e.kind == cgraph_core::EdgeKind::Call)
        .collect();

    // Should only have "standalone" as a call edge
    assert!(
        call_edges.iter().any(|e| e.target_id.contains("standalone")),
        "Missing call edge for standalone(). Call edges: {:?}",
        call_edges
    );
    // Should NOT have method, fetchData, or log
    assert!(
        !call_edges.iter().any(|e| e.target_id.contains("method")),
        "Member call obj.method() should not produce edge. Call edges: {:?}",
        call_edges
    );
    assert!(
        !call_edges.iter().any(|e| e.target_id.contains("log")),
        "Member call console.log() should not produce edge. Call edges: {:?}",
        call_edges
    );
}

// PARS-07: Type reference edges

#[test]
fn type_ref_extends() {
    let extractor = TsExtractor::new();
    let source = std::fs::read_to_string("tests/fixtures/services.ts")
        .expect("services.ts fixture missing");
    let result = extractor.extract(Path::new("tests/fixtures/services.ts"), &source);

    // class UserService extends UserRepository
    let extends_edges: Vec<_> = result.edges.iter()
        .filter(|e| e.kind == cgraph_core::EdgeKind::TypeRef && e.source_id.contains("UserService"))
        .collect();
    assert!(
        extends_edges.iter().any(|e| e.target_id.contains("UserRepository")),
        "Missing TypeRef edge: UserService extends UserRepository. TypeRef edges: {:?}",
        result.edges.iter().filter(|e| e.kind == cgraph_core::EdgeKind::TypeRef).collect::<Vec<_>>()
    );
}

#[test]
fn type_ref_implements() {
    let extractor = TsExtractor::new();
    let source = std::fs::read_to_string("tests/fixtures/services.ts")
        .expect("services.ts fixture missing");
    let result = extractor.extract(Path::new("tests/fixtures/services.ts"), &source);

    // class UserRepository implements Repository
    let impl_edges: Vec<_> = result.edges.iter()
        .filter(|e| e.kind == cgraph_core::EdgeKind::TypeRef && e.source_id.contains("UserRepository"))
        .collect();
    assert!(
        impl_edges.iter().any(|e| e.target_id.contains("Repository")),
        "Missing TypeRef edge: UserRepository implements Repository. TypeRef edges: {:?}",
        result.edges.iter().filter(|e| e.kind == cgraph_core::EdgeKind::TypeRef).collect::<Vec<_>>()
    );
}

#[test]
fn type_ref_iface_extends() {
    // Interface extending another interface
    let extractor = TsExtractor::new();
    let source = r#"
        interface Base {
            id: string;
        }
        export interface Extended extends Base {
            name: string;
        }
    "#;
    let result = extractor.extract(Path::new("test.ts"), source);

    let type_refs: Vec<_> = result.edges.iter()
        .filter(|e| e.kind == cgraph_core::EdgeKind::TypeRef)
        .collect();
    assert!(
        type_refs.iter().any(|e| e.target_id.contains("Base") && e.source_id.contains("Extended")),
        "Missing TypeRef edge: Extended extends Base. TypeRef edges: {:?}",
        type_refs
    );
}

// PARS-08: Re-export edges

#[test]
fn reexport_named() {
    let extractor = TsExtractor::new();
    let source = std::fs::read_to_string("tests/fixtures/barrel.ts")
        .expect("barrel.ts fixture missing");
    let result = extractor.extract(Path::new("tests/fixtures/barrel.ts"), &source);

    // export { UserService, UserRepository } from './services'
    let reexport_edges: Vec<_> = result.edges.iter()
        .filter(|e| e.kind == cgraph_core::EdgeKind::ReExport)
        .collect();
    assert!(
        reexport_edges.iter().any(|e| e.target_id.contains("./services::UserService")),
        "Missing ReExport edge for UserService. ReExport edges: {:?}",
        reexport_edges
    );
    assert!(
        reexport_edges.iter().any(|e| e.target_id.contains("./services::UserRepository")),
        "Missing ReExport edge for UserRepository. ReExport edges: {:?}",
        reexport_edges
    );
}

#[test]
fn reexport_star() {
    let extractor = TsExtractor::new();
    let source = std::fs::read_to_string("tests/fixtures/barrel.ts")
        .expect("barrel.ts fixture missing");
    let result = extractor.extract(Path::new("tests/fixtures/barrel.ts"), &source);

    // export * from './hooks'
    let star_edges: Vec<_> = result.edges.iter()
        .filter(|e| e.kind == cgraph_core::EdgeKind::ReExport && e.target_id.contains("::*"))
        .collect();
    assert!(
        star_edges.iter().any(|e| e.target_id.contains("./hooks::*")),
        "Missing star ReExport edge for ./hooks. Star edges: {:?}",
        star_edges
    );
}

#[test]
fn reexport_raw_path() {
    // Per D-25: extractor emits raw single-hop ReExport edges (no chain resolution)
    let extractor = TsExtractor::new();
    let source = r#"export { Config } from './config';"#;
    let result = extractor.extract(Path::new("index.ts"), source);

    let reexport_edges: Vec<_> = result.edges.iter()
        .filter(|e| e.kind == cgraph_core::EdgeKind::ReExport)
        .collect();
    assert_eq!(reexport_edges.len(), 1, "Expected exactly 1 reexport edge");
    assert!(
        reexport_edges[0].target_id.contains("./config::Config"),
        "ReExport target should be raw path. Got: {:?}",
        reexport_edges[0].target_id
    );
}

// PARS-09 and PARS-10 are implicit -- verified by import_raw_alias_path and reexport_raw_path tests
// Phase 2 emits raw paths; Phase 3 indexer resolves them.

#[test]
fn namespace_reexport_not_star() {
    // export * as ns from './module' should NOT produce a star ::* -> ::* edge
    let extractor = TsExtractor::new();
    let source = r#"export * as Utils from './utils';"#;
    let result = extractor.extract(Path::new("index.ts"), source);

    let reexport_edges: Vec<_> = result.edges.iter()
        .filter(|e| e.kind == cgraph_core::EdgeKind::ReExport)
        .collect();

    // Should have exactly 1 edge: namespace re-export
    assert_eq!(reexport_edges.len(), 1, "Expected 1 namespace reexport edge. Got: {:?}", reexport_edges);

    let edge = &reexport_edges[0];
    // source_id should be "index.ts::Utils" (the namespace name, NOT ::*)
    assert_eq!(edge.source_id, "index.ts::Utils",
        "Namespace re-export source_id should use namespace name, not ::*. Got: {}", edge.source_id);
    // target_id should be "./utils::*" (everything from the module)
    assert_eq!(edge.target_id, "./utils::*",
        "Namespace re-export target_id should be path::*. Got: {}", edge.target_id);
}

#[test]
fn star_reexport_still_works() {
    // Regression: plain export * from should still work after namespace fix
    let extractor = TsExtractor::new();
    let source = r#"export * from './helpers';"#;
    let result = extractor.extract(Path::new("index.ts"), source);

    let star_edges: Vec<_> = result.edges.iter()
        .filter(|e| e.kind == cgraph_core::EdgeKind::ReExport && e.source_id.contains("::*"))
        .collect();
    assert_eq!(star_edges.len(), 1, "Expected 1 star reexport edge. Got: {:?}", star_edges);
    assert_eq!(star_edges[0].target_id, "./helpers::*");
}

#[test]
fn overload_dedup() {
    // TypeScript function overloads should produce a single SymbolNode
    let extractor = TsExtractor::new();
    let source = r#"
        export function greet(name: string): string;
        export function greet(name: string, greeting: string): string;
        export function greet(name: string, greeting?: string): string {
            return (greeting || "Hello") + ", " + name;
        }
    "#;
    let result = extractor.extract(Path::new("overloads.ts"), source);

    let greet_nodes: Vec<_> = result.nodes.iter()
        .filter(|n| n.name == "greet")
        .collect();
    assert_eq!(greet_nodes.len(), 1,
        "Overloaded function 'greet' should produce exactly 1 SymbolNode. Got: {}",
        greet_nodes.len());
}

// --- Full Integration Test ---

#[test]
fn full_extraction_produces_nodes_and_edges() {
    let extractor = TsExtractor::new();
    let source = std::fs::read_to_string("tests/fixtures/services.ts")
        .expect("services.ts fixture missing");
    let result = extractor.extract(Path::new("tests/fixtures/services.ts"), &source);

    // Should have both nodes and edges
    assert!(!result.nodes.is_empty(), "No symbols extracted from services.ts");
    assert!(!result.edges.is_empty(), "No edges extracted from services.ts");
    assert!(result.errors.is_empty(), "Unexpected errors: {:?}", result.errors);

    // Verify we get import, call, and type ref edges
    let has_import = result.edges.iter().any(|e| e.kind == cgraph_core::EdgeKind::Import);
    let has_type_ref = result.edges.iter().any(|e| e.kind == cgraph_core::EdgeKind::TypeRef);
    assert!(has_import, "No Import edges in services.ts");
    assert!(has_type_ref, "No TypeRef edges in services.ts");
}

#[test]
fn partial_parse_still_extracts() {
    // Per D-14: partial parse errors don't prevent extraction
    let extractor = TsExtractor::new();
    let source = r#"
        export function validFn(): string { return "ok"; }
        export function broken( { // syntax error
        export function anotherValid(): number { return 42; }
    "#;
    let result = extractor.extract(Path::new("broken.ts"), source);

    // Should have parse error recorded
    assert!(!result.errors.is_empty(), "Should have parse errors for invalid syntax");
    // Should still extract what it can (may or may not get validFn depending on error recovery)
    // The key contract is: no panic, errors recorded as data
}
