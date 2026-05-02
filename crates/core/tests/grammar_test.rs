use tree_sitter::Parser;
use tree_sitter_typescript::{LANGUAGE_TYPESCRIPT, LANGUAGE_TSX};

#[test]
fn typescript_grammar_links_and_parses() {
    let mut parser = Parser::new();
    parser
        .set_language(&LANGUAGE_TYPESCRIPT.into())
        .expect("Error loading TypeScript grammar");

    let source = std::fs::read_to_string("tests/fixtures/sample.ts")
        .expect("sample.ts fixture missing");
    let tree = parser.parse(&source, None).expect("parse returned None");
    let root = tree.root_node();

    assert_eq!(root.kind(), "program");
    assert!(!root.has_error(), "ERROR nodes in sample.ts");
}

#[test]
fn tsx_grammar_links_and_parses() {
    let mut parser = Parser::new();
    parser
        .set_language(&LANGUAGE_TSX.into())
        .expect("Error loading TSX grammar");

    let source = std::fs::read_to_string("tests/fixtures/sample.tsx")
        .expect("sample.tsx fixture missing");
    let tree = parser.parse(&source, None).expect("parse returned None");
    assert!(!tree.root_node().has_error(), "ERROR nodes in sample.tsx");
}

#[test]
fn swift_grammar_links() {
    use tree_sitter_swift::LANGUAGE;
    let mut parser = Parser::new();
    parser
        .set_language(&LANGUAGE.into())
        .expect("Error loading Swift grammar");

    let source = std::fs::read_to_string("tests/fixtures/sample.swift")
        .expect("sample.swift fixture missing");
    let tree = parser.parse(&source, None).expect("parse returned None");
    assert!(!tree.root_node().has_error(), "ERROR nodes in sample.swift");
}

#[test]
fn go_grammar_links() {
    use tree_sitter_go::LANGUAGE;
    let mut parser = Parser::new();
    parser
        .set_language(&LANGUAGE.into())
        .expect("Error loading Go grammar");

    let source = std::fs::read_to_string("tests/fixtures/sample.go")
        .expect("sample.go fixture missing");
    let tree = parser.parse(&source, None).expect("parse returned None");
    assert!(!tree.root_node().has_error(), "ERROR nodes in sample.go");
}

#[test]
fn python_grammar_links() {
    use tree_sitter_python::LANGUAGE;
    let mut parser = Parser::new();
    parser
        .set_language(&LANGUAGE.into())
        .expect("Error loading Python grammar");

    let source = std::fs::read_to_string("tests/fixtures/sample.py")
        .expect("sample.py fixture missing");
    let tree = parser.parse(&source, None).expect("parse returned None");
    assert!(!tree.root_node().has_error(), "ERROR nodes in sample.py");
}
