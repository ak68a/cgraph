# cgraph

## What This Is

A multi-language static analysis tool that parses codebases (TypeScript/React Native, Swift, Go, Python) and produces an interactive force-directed graph of symbol relationships. Written in Rust, distributed as a single binary (`cg`) — run it against any project directory and explore the graph in your browser.

## Core Value

Instantly see what's connected to what — dead code, blast radius, dependency depth — without manual grep work.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] Multi-language parsing via tree-sitter (TS/RN, Swift, Go, Python)
- [ ] Interactive D3 force graph of symbols and relationships
- [ ] Dead code detection (unused exports highlighted)
- [ ] Blast radius view (click a symbol, see all transitive dependents)
- [ ] Search and focus on specific symbols
- [ ] Watch mode with live graph updates on file save
- [ ] CLI delivery as single binary (`cg ./path`)
- [ ] Filtering by file, symbol type, usage count, edge type

### Out of Scope

- Electron/desktop app — CLI + browser is sufficient
- CI integration — on-demand and always-on usage only
- Code modification/refactoring — read-only analysis tool
- Cross-repo analysis — single project directory at a time

## Context

- Built to scratch a real itch: finding dead code (`fetchPilotDetails`, `parseUserDocument`) and understanding change blast radius (`_seconds/_nanoseconds` timestamp format across Zod schemas, interfaces, parsers) in the OversizeConnect React Native codebase
- General-purpose tool, but OversizeConnect is the first test case and proving ground
- Usage pattern: mix of on-demand exploration and always-on dashboard (watch mode)
- Tree-sitter chosen over language-specific parsers (ts-morph) for multi-language support from a single parsing infrastructure
- Written in Rust — tree-sitter is native C/Rust (no binding overhead), single binary distribution eliminates all install friction, performance handles any codebase size
- Each language is a pluggable "extractor" — adding a new language is writing one extractor, not rebuilding the tool
- Browser client is still HTML/JS/D3 — Rust serves it as embedded static files

## Constraints

- **Tech stack**: Rust for CLI/parser/server; HTML/JS/D3 for browser client
- **Distribution**: Single binary via cargo install, Homebrew, and npm (prebuilt binaries)
- **Parser**: tree-sitter (native C/Rust) for all languages — no binding layer
- **Visualization**: D3 force graph in browser — full control over rendering and interaction
- **Delivery**: CLI that starts localhost server, opens browser tab

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rust over Node.js/TypeScript | Tree-sitter is native C/Rust (no binding layer); single binary distribution eliminates ABI/install issues; performance is free | — Pending |
| Tree-sitter over ts-morph | Multi-language support from day one; single parsing infrastructure | — Pending |
| D3 over Cytoscape.js | More control over graph rendering; user preference | — Pending |
| CLI + localhost over Electron | Zero packaging complexity; browser does the heavy lifting | — Pending |
| All 4 languages in v1 | Extractors are independent work; tree-sitter makes it modular | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-05-02 after Phase 02 completion — TypeScript extractor fully implemented with 30 passing tests*
