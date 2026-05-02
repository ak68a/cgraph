/// Symbol extraction query - captures exported function/arrow/interface/type/class/enum declarations.
/// Each pattern index maps to a SymbolKind (see lib.rs pattern_to_kind).
pub const SYMBOL_QUERY_SRC: &str = r#"
; Pattern 0: exported function declaration
(export_statement
  declaration: (function_declaration
    name: (identifier) @symbol_name)) @export_stmt

; Pattern 1: exported arrow function (const)
(export_statement
  declaration: (lexical_declaration
    (variable_declarator
      name: (identifier) @symbol_name
      value: (arrow_function)))) @export_stmt

; Pattern 2: exported interface
(export_statement
  declaration: (interface_declaration
    name: (type_identifier) @symbol_name)) @export_stmt

; Pattern 3: exported type alias
(export_statement
  declaration: (type_alias_declaration
    name: (type_identifier) @symbol_name)) @export_stmt

; Pattern 4: exported class
(export_statement
  declaration: (class_declaration
    name: (type_identifier) @symbol_name)) @export_stmt

; Pattern 5: exported enum
(export_statement
  declaration: (enum_declaration
    name: (identifier) @symbol_name)) @export_stmt

; Pattern 6: export default identifier (export default Foo)
(export_statement
  value: (identifier) @default_name) @default_export
"#;

/// Import edge query - captures named, default, and namespace imports.
pub const IMPORT_QUERY_SRC: &str = r#"
; Named imports: import { foo, bar } from './module'
(import_statement
  (import_clause
    (named_imports
      (import_specifier
        name: (identifier) @import_name)))
  source: (string
    (string_fragment) @import_path))

; Default import: import Foo from './module'
(import_statement
  (import_clause
    (identifier) @default_import_name)
  source: (string
    (string_fragment) @import_path))

; Namespace import: import * as ns from './module'
(import_statement
  (import_clause
    (namespace_import
      (identifier) @namespace_name))
  source: (string
    (string_fragment) @import_path))
"#;

/// Call edge query - direct named function calls only (per D-30).
/// Matches foo(), useState(), Component() but NOT obj.method() or this.foo().
pub const CALL_QUERY_SRC: &str = r#"
(call_expression
  function: (identifier) @call_target)
"#;

/// Type reference query - extends, implements, type annotations.
pub const TYPE_REF_QUERY_SRC: &str = r#"
; Class extends: class Foo extends Bar
(class_declaration
  name: (type_identifier) @class_name
  (class_heritage
    (extends_clause
      (identifier) @extends_target)))

; Class implements: class Foo implements Bar, Baz
(class_declaration
  name: (type_identifier) @class_name
  (class_heritage
    (implements_clause
      (type_identifier) @implements_target)))

; Interface extends: interface Foo extends Bar
(interface_declaration
  name: (type_identifier) @iface_name
  (extends_type_clause
    (type_identifier) @extends_target))
"#;

/// Re-export query - named and star re-exports (per D-26).
/// Named re-exports: export { foo, bar } from './module'
/// Star re-exports: export * from './module'
pub const REEXPORT_QUERY_SRC: &str = r#"
; Named re-export: export { foo, bar } from './module'
(export_statement
  (export_clause
    (export_specifier
      name: (identifier) @specifier_name))
  source: (string
    (string_fragment) @source_path))

; Star re-export: export * from './module'
(export_statement
  source: (string
    (string_fragment) @star_source))
"#;
