# Code Graph

## What This Is

A multi-language static analysis tool that parses codebases (TypeScript/React Native, Swift, Go, Python) and produces an interactive force-directed graph of symbol relationships. Distributed as an npm CLI — run it against any project directory and explore the graph in your browser.

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
- [ ] CLI delivery via npm (`npx code-graph ./path`)
- [ ] Filtering by file, symbol type, usage count, edge type

### Out of Scope

- Electron/desktop app — CLI + browser is sufficient
- Homebrew/standalone binary — npm distribution only for v1
- CI integration — on-demand and always-on usage only
- Code modification/refactoring — read-only analysis tool
- Cross-repo analysis — single project directory at a time

## Context

- Built to scratch a real itch: finding dead code (`fetchPilotDetails`, `parseUserDocument`) and understanding change blast radius (`_seconds/_nanoseconds` timestamp format across Zod schemas, interfaces, parsers) in the OversizeConnect React Native codebase
- General-purpose tool, but OversizeConnect is the first test case and proving ground
- Usage pattern: mix of on-demand exploration and always-on dashboard (watch mode)
- Tree-sitter chosen over language-specific parsers (ts-morph) for multi-language support from a single parsing infrastructure
- Each language is a pluggable "extractor" — adding a new language is writing one extractor, not rebuilding the tool

## Constraints

- **Tech stack**: Node.js runtime (tree-sitter Node bindings, D3, HTTP server)
- **Distribution**: npm package only — no native binary cross-compilation
- **Parser**: tree-sitter for all languages — consistent AST approach
- **Visualization**: D3 force graph — full control over rendering and interaction
- **Delivery**: CLI that starts localhost server, opens browser tab

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Tree-sitter over ts-morph | Multi-language support from day one; single parsing infrastructure | — Pending |
| D3 over Cytoscape.js | More control over graph rendering; user preference | — Pending |
| CLI + localhost over Electron | Zero packaging complexity; browser does the heavy lifting | — Pending |
| npm over Homebrew | Tool is already Node-native; free distribution; users have Node | — Pending |
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
*Last updated: 2026-05-02 after initialization*
