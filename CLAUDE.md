# cgraph

Multi-language code graph visualization tool written in Rust. Parses codebases (TypeScript, Swift, Go, Python) via tree-sitter and serves an interactive D3 force graph in the browser.

## Project Structure

- `src/` — Rust source (CLI, parser, indexer, server)
- `client/` — Browser client (HTML, JS, D3 force graph)
- `.planning/` — Project planning docs (roadmap, requirements, research)

## Tech Stack

- **Language**: Rust
- **Parser**: tree-sitter (native C/Rust linkage)
- **CLI**: clap
- **HTTP/WebSocket**: axum or actix-web
- **Browser**: D3.js force graph (embedded static assets)
- **File watching**: notify crate

## Commands

- `cargo build` — Build the binary
- `cargo run -- <path>` — Run against a directory
- `cargo test` — Run tests

## GSD Workflow

This project uses the GSD planning workflow. Planning docs are in `.planning/`.

- Current phase: check `.planning/STATE.md`
- Roadmap: `.planning/ROADMAP.md`
- Requirements: `.planning/REQUIREMENTS.md`
