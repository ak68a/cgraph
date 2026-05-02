# Technology Stack

**Project:** code-graph — multi-language static analysis tool with interactive graph visualization
**Researched:** 2026-05-02

---

## Recommended Stack

### Parsing: Tree-sitter Node.js Bindings

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| `tree-sitter` | `^0.25.1` | Native Node.js bindings for tree-sitter runtime | Stable Node-API based bindings (since v0.21.0); prebuilt binaries via prebuildify means no compile-on-install for users on common platforms |
| `tree-sitter-typescript` | `^0.23.2` | TypeScript + TSX grammar | Mature (739 commits, Nov 2024 release); two explicit dialects: `typescript` and `tsx`; maintained by tree-sitter org |
| `tree-sitter-go` | `^0.25.0` | Go grammar | 207 dependents on npm; maintained by tree-sitter org; stable at 0.25.0 |
| `tree-sitter-python` | `^0.25.0` | Python grammar | 353 dependents on npm; maintained by tree-sitter org; stable at 0.25.0 |
| `tree-sitter-swift` | `^0.7.1` | Swift grammar | Community-maintained (alex-pinkus); 477 commits, actively updated; v0.7.1 released June 2025; only credible option for Swift on npm |

**What NOT to use:**

- `web-tree-sitter` (WASM version): Designed for browser environments; adds WASM load overhead and async init boilerplate in a CLI context. Use native Node bindings instead. WASM format changed in 0.26.x and is incompatible with grammars compiled against 0.20.x.
- `tree-sitter@^0.26.x`: Not published to npm yet (as of May 2026); the 0.26 series requires Node 24 for native bindings — a breaking constraint for users on Node 18/20/22. Pin to 0.25.x until 0.26 stabilizes on npm.
- `ts-morph`: TypeScript-only; defeats the entire multi-language premise of this tool.

**Critical install note for macOS:** tree-sitter 0.25.0 has a known `npm install` failure on newer macOS + newer Node (C++20 compiler flag not passed by node-gyp). Workaround: `CXXFLAGS="-std=c++20" npm install`. This is a known open issue (#5335). Include this in your README prominently.

### Visualization: D3.js

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| `d3` | `^7.9.0` | Force-directed graph simulation and SVG rendering | v7.9.0 is the current stable release (March 2024); no v8 exists; full control over layout, interaction, and visual encoding that Cytoscape.js does not offer |

**Rendering strategy — SVG vs Canvas:**

For code graphs up to ~500 nodes: use SVG. D3's data-join model gives you direct CSS styling, hover states, and click handlers without extra abstraction. This is the correct choice for MVP where the graph fits a typical medium codebase.

For graphs with 500–5000+ nodes: switch the rendering layer to Canvas (`<canvas>` + CanvasRenderingContext2D) while keeping D3's force simulation intact. The simulation drives `x`/`y` coordinates; Canvas redraws on each tick. This hybrid keeps D3 simulation logic identical and only swaps the renderer.

**Recommended approach for this project:** Start with SVG. Add a Canvas fallback path only if performance profiling shows it is needed. Do not over-engineer the renderer on day one — the OversizeConnect codebase is unlikely to exceed 500 symbols initially.

**What NOT to use:**

- `Cytoscape.js`: Less control over layout physics; harder to customize visual encoding; the user explicitly chose D3 for full rendering control.
- `sigma.js` / `@antv/g6`: WebGL-based alternatives — overkill for this use case and adds complexity.
- `d3-force-3d`: 3D layout adds no value for a 2D dependency graph.

### CLI Framework

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| `commander` | `^14.0.0` | CLI argument parsing, subcommands, help generation | Most downloaded Node.js CLI library (~500M weekly downloads); clean fluent API; v14 is the current stable line (released May 2025); zero external dependencies |

**What NOT to use:**

- `yargs`: More verbose configuration, heavier; best when you need complex argument coercion middleware. Overkill for a single-command CLI.
- `meow`: Minimal and opinionated ESM-only; less discoverable help generation.
- `oclif` (Salesforce): Massive framework with scaffolding, plugin system, and CI hooks. Wrong abstraction level for a focused single-binary tool.

### HTTP Server (serves the graph UI)

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| Node.js built-in `http` | (built-in) | Serve the bundled HTML/JS/CSS graph viewer | Zero dependencies; a single static HTML file + inlined data payload means you do not need a framework. Serving one file at `GET /` and a data endpoint at `GET /data` is 30 lines of code with `http.createServer`. |

**What NOT to use:**

- `express` / `fastify` / `hono`: Framework overhead is unjustified for two routes. The HTTP server here is just a transport for the static UI — not a general-purpose API layer.
- `http-server` / `serve` (npm packages): These are standalone binaries, not embeddable libraries. You need programmatic control (inject graph data, assign dynamic port, signal WebSocket readiness).

### Open Browser Tab

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| `open` | `^10.x` (ESM) or `^9.x` (CJS) | Cross-platform `open localhost:PORT` after server starts | sindresorhus's `open` package is the de facto standard; uses `open` on macOS, `xdg-open` on Linux, `start` on Windows; spawns safely without shell injection |

**Note on ESM:** `open` v10+ is pure ESM (`import open from 'open'`). If the project's `package.json` uses `"type": "module"` this is fine. If CommonJS, pin to `open@9` which still provides a CJS export. Decide module system early — it propagates to all dependencies.

### WebSocket: Live Updates (Watch Mode)

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| `ws` | `^8.20.0` | Push incremental graph updates from CLI to browser on file save | Battle-hardened; 8.20.0 current (published ~April 2026); attaches to the same `http.Server` instance — no extra port; optional native binary for performance (not required for this use case) |

**What NOT to use:**

- `socket.io`: Adds polling fallback, rooms, namespaces, and reconnection logic you do not need. 10x heavier than `ws`. This is a local-only tool; WebSocket will always be available.
- `SSE (Server-Sent Events)` via built-in: A viable lightweight alternative if you only need server → browser push with no ACK. Worth considering if you want to eliminate `ws` entirely. However `ws` gives you bidirectional messaging for free if you later need client → server commands (e.g., "re-analyze now").

### File Watcher (Watch Mode)

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| `chokidar` | `^4.x` | Detect file saves and trigger re-parse | v4.x supports both ESM and CommonJS, Node 14+ minimum, 1 dependency (down from 13 in v3); reliable cross-platform FSEvents/inotify/polling fallback |

**Version guidance:** Chokidar v5 (released Nov 2025) is ESM-only and requires Node 20+. If the project targets Node 18 LTS users, pin to `chokidar@4`. If targeting Node 20+ only, v5 is fine and reduces install size further. Recommend `^4.x` for maximum user compatibility.

**What NOT to use:**

- Node.js `fs.watch` directly: Unreliable on macOS for directory trees; does not detect renamed/deleted files reliably across platforms.
- `nodemon`: A process runner, not a library. Not embeddable as a file-change event emitter.

---

## Full Dependency Table

### Runtime Dependencies

| Package | Pinned Version | Role |
|---------|---------------|------|
| `tree-sitter` | `^0.25.1` | Core parsing runtime |
| `tree-sitter-typescript` | `^0.23.2` | TypeScript/TSX grammar |
| `tree-sitter-go` | `^0.25.0` | Go grammar |
| `tree-sitter-python` | `^0.25.0` | Python grammar |
| `tree-sitter-swift` | `^0.7.1` | Swift grammar |
| `commander` | `^14.0.0` | CLI argument handling |
| `open` | `^9.1.0` | Open browser tab (CJS-compatible) |
| `ws` | `^8.20.0` | WebSocket server for live updates |
| `chokidar` | `^4.0.0` | File watcher for watch mode |
| `d3` | `^7.9.0` | Force simulation (served to browser) |

`d3` ships to the browser, not the Node process. Bundle it into the static HTML file (CDN link or inline the minified bundle) to avoid shipping it as a Node runtime dep.

### Dev Dependencies

| Package | Purpose |
|---------|---------|
| `typescript` | Type safety during development |
| `@types/node` | Node.js type definitions |
| `@types/d3` | D3 type definitions |
| `@types/ws` | ws type definitions |

---

## Module System Decision

**Recommend: CommonJS (`"type": "module"` absent)**

Rationale: `tree-sitter` (native Node addon) and `ws` are CommonJS-compatible but `open@10+` is ESM-only. Using CommonJS lets you `require()` all native addons without dynamic import overhead. Use `open@9` (last CJS release) or wrap `open@10` in a dynamic `import()`.

If you prefer full ESM, the project works but you must use `import()` for tree-sitter grammars (which are CommonJS modules) and keep Node version at 18+. ESM is not a meaningful benefit for a CLI tool with no code-splitting needs.

---

## Installation Commands

```bash
# Runtime dependencies
npm install tree-sitter@^0.25.1 \
  tree-sitter-typescript@^0.23.2 \
  tree-sitter-go@^0.25.0 \
  tree-sitter-python@^0.25.0 \
  tree-sitter-swift@^0.7.1 \
  commander@^14.0.0 \
  open@^9.1.0 \
  ws@^8.20.0 \
  chokidar@^4.0.0

# d3 goes to the browser bundle — install as dev dep or omit from package.json entirely
npm install -D d3@^7.9.0 @types/d3

# Dev tooling
npm install -D typescript @types/node @types/ws
```

---

## Confidence Assessment

| Decision | Confidence | Source |
|----------|------------|--------|
| `tree-sitter@^0.25.1` | HIGH | Official node-tree-sitter docs (v0.25.1 current); GitHub issue tracker confirms 0.26 not on npm |
| `tree-sitter-typescript@^0.23.2` | HIGH | GitHub tree-sitter/tree-sitter-typescript — v0.23.2 released Nov 2024 |
| `tree-sitter-go@^0.25.0` | HIGH | npm registry — 207 dependents, 0.25.0 current |
| `tree-sitter-python@^0.25.0` | HIGH | npm registry — 353 dependents, 0.25.0 current |
| `tree-sitter-swift@^0.7.1` | MEDIUM | GitHub alex-pinkus/tree-sitter-swift — actively maintained, v0.7.1 June 2025; only available option; 46 weekly npm downloads is low but grammar quality appears solid |
| `d3@^7.9.0` | HIGH | GitHub d3/d3 releases — v7.9.0 is latest; no v8 exists |
| `commander@^14.0.0` | HIGH | GitHub tj/commander.js releases — v14.0.x current (released May 2025) |
| Built-in `http` server | HIGH | Established pattern; no library needed |
| `open@^9.x` (CJS) | HIGH | GitHub sindresorhus/open — de facto standard; v9 is last CJS release |
| `ws@^8.20.0` | HIGH | npm registry — v8.20.0 current (April 2026) |
| `chokidar@^4.x` | HIGH | GitHub paulmillr/chokidar — v4 stable, v5 ESM-only (Node 20+) |
| SVG rendering (not Canvas) | MEDIUM | Community consensus: SVG fine under ~500 nodes; code graph scale for MVP fits this range |

---

## Sources

- [node-tree-sitter official docs v0.25.1](https://tree-sitter.github.io/node-tree-sitter/)
- [node-tree-sitter GitHub releases](https://github.com/tree-sitter/node-tree-sitter/releases)
- [tree-sitter npm not updated for v0.26 (issue #5334)](https://github.com/tree-sitter/tree-sitter/issues/5334)
- [npm install tree-sitter fails on macOS 26.2 (issue #5335)](https://github.com/tree-sitter/tree-sitter/issues/5335)
- [tree-sitter-typescript GitHub](https://github.com/tree-sitter/tree-sitter-typescript)
- [tree-sitter-swift GitHub (alex-pinkus)](https://github.com/alex-pinkus/tree-sitter-swift)
- [d3 GitHub releases](https://github.com/d3/d3/releases)
- [commander.js GitHub releases](https://github.com/tj/commander.js/releases)
- [ws npm package](https://www.npmjs.com/package/ws)
- [chokidar GitHub — migrating 3.x to 4.x](https://dev.to/43081j/migrating-from-chokidar-3x-to-4x-5ab5)
- [chokidar v5 on Libraries.io](https://libraries.io/npm/chokidar)
- [open GitHub (sindresorhus)](https://github.com/sindresorhus/open)
- [D3 force layout performance — SVG vs Canvas](https://reintech.io/blog/optimizing-d3-chart-performance-large-data)
