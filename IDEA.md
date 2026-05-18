# Code Graph

Static analysis tool that parses a TypeScript/React Native codebase and produces an interactive graph view of relationships between functions, classes, components, hooks, and exports.

## Problem

No easy way to visually see what's connected to what across a codebase. Dead code (like `fetchPilotDetails` in pilot-api.ts) sits around because nothing flags it. Understanding call chains, dependency depth, and unused exports requires manual grep work.

## Core Features

- **Obsidian-style force graph** of symbols (functions, components, hooks, types, classes)
- **Edges** represent imports, function calls, type references
- **Filters**: by file, by symbol type (hook/component/function/type), by usage count
- **Highlight unused exports** (zero incoming edges)
- **Click a node** to see its source, callers, and callees
- **Search** to find and focus on a specific symbol

## Tech

- **Parser**: `ts-morph` (wraps TypeScript compiler API) for AST + symbol resolution
- **Graph viz**: D3 force graph or Cytoscape.js
- **Delivery**: local web app — point at a directory, it parses and serves the graph
- **Optional**: watch mode to re-parse on file changes

## Inspired By

- Finding `parseUserDocument` and `fetchPilotDetails` were dead code in mobile-app by manually grepping for consumers
- The `_seconds`/`_nanoseconds` timestamp format being baked into Zod schemas, interfaces, and parsers across many files with no way to see the full blast radius of changing it
