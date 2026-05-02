# Domain Pitfalls: Code Graph Visualization Tool

**Domain:** Multi-language static analysis + interactive force-directed graph visualization
**Researched:** 2026-05-02
**Applies to:** code-graph (tree-sitter + D3 + Node.js CLI)

---

## Critical Pitfalls

Mistakes that cause rewrites, broken installs, or a tool nobody uses.

---

### Pitfall 1: Tree-sitter Native Binding ABI Mismatch

**What goes wrong:** The tree-sitter Node.js native binding (`.node` file) is compiled against one Node.js ABI version and breaks silently or loudly when the user's Node.js version differs. The error message is: `"The module '.../tree_sitter_runtime_binding.node' was compiled against a different Node.js version using NODE_MODULE_VERSION 115. This version of Node.js requires NODE_MODULE_VERSION 118."` Users running `npx code-graph` on a system where Node.js has been updated since the binding was compiled get a hard crash on first use.

**Why it happens:** The npm `tree-sitter` package ships prebuilt `.node` binaries. These binaries are tied to a specific Node ABI. When the user's Node version changes (e.g., upgraded via `nvm`), the pre-compiled binary no longer matches. tree-sitter uses `node-gyp-build` which will fall back to compiling from source — but this requires Python, a C++ compiler (`gcc`/`clang`), and build tools to be present, which most end users of a CLI tool do not have.

**Consequences:** Hard install failure or runtime crash. Users open GitHub issues. Tool reputation damaged before they see any graph.

**Warning signs:**
- Any user reporting `npm install` or `npx` failing with MODULE_VERSION errors
- CI logs failing on Node version upgrade

**Prevention:**
- Pin a specific Node.js version range in `engines` field of `package.json` and enforce it with `check-node-version` or similar at startup
- Test installation on Node 18 LTS, Node 20 LTS, and Node 22 LTS in CI
- Treat the ABI mismatch scenario as a first-class error with a human-readable message: "Please run `npm rebuild` or reinstall the package"
- Consider evaluating `web-tree-sitter` (WASM) as the default transport with native as an opt-in for performance — WASM binaries are architecture-independent and never need recompilation

**Phase:** Parser foundation phase. Must be solved before any public release.

**Sources:** [Issue #2867](https://github.com/tree-sitter/tree-sitter/issues/2867), [Issue #188](https://github.com/tree-sitter/node-tree-sitter/issues/188)

---

### Pitfall 2: Tree-sitter Grammar / Runtime Version Mismatch

**What goes wrong:** `tree-sitter-typescript`, `tree-sitter-go`, etc. are independent packages with their own version of the tree-sitter ABI they were compiled against. Installing a grammar package version that was built against a different tree-sitter core ABI version produces: `"language was generated with an incompatible version of the Tree-sitter CLI"`. This happens silently if the language loads but produces garbage parse trees.

**Why it happens:** Each language grammar is compiled C code that embeds the tree-sitter ABI version at compile time. The npm `tree-sitter` core package version and each `tree-sitter-<language>` grammar version must target matching ABI ranges. The ecosystem has active version drift — in late 2025 the npm `tree-sitter` package was at v0.25 while the GitHub release was at v0.26.5, creating a permanent mismatch for anyone relying on npm.

**Consequences:** Parser returns ERROR nodes for valid code without throwing. Your extractor silently produces incomplete or wrong graphs. Dead code is not detected. Blast radius is wrong.

**Warning signs:**
- ERROR nodes in the parsed AST for syntactically valid files
- Unexpectedly low node counts on a known codebase
- Grammar package requiring a different tree-sitter version than the one installed

**Prevention:**
- Lock ALL tree-sitter packages (core + all language grammars) to exact versions (`"tree-sitter": "0.21.0"`, not `"^0.21.0"`) in `package.json`
- Add an integration test that parses a known fixture file and asserts zero ERROR nodes in the result
- Re-test the locked version matrix on every Node LTS upgrade
- Check `Language.version` at startup against `Parser.LANGUAGE_VERSION` and `Parser.MIN_COMPATIBLE_LANGUAGE_VERSION` — throw a clear error if they don't match rather than silently producing wrong output

**Phase:** Parser foundation phase. Locked versions must be committed before writing any extractor.

**Sources:** [Issue #151](https://github.com/tree-sitter/tree-sitter/issues/151), [Issue #4598](https://github.com/tree-sitter/tree-sitter/issues/4598), [Issue #5334](https://github.com/tree-sitter/tree-sitter/issues/5334)

---

### Pitfall 3: D3 Force Graph Freezes the Browser with Large Codebases

**What goes wrong:** D3's force simulation runs on the main browser thread. Each tick calls `requestAnimationFrame`, then updates every node and edge position in the DOM. At 500+ nodes with SVG rendering, the browser UI becomes sluggish. At 1,500+ nodes it freezes. A real TypeScript codebase can have 5,000-20,000 symbols.

**Why it happens:** D3 force layout is O(n log n) per tick via the Barnes-Hut approximation, but the rendering step — updating SVG element `cx/cy/x/y` attributes — is O(n+e) and involves DOM mutation on each animation frame. The force simulation also needs hundreds of ticks to converge, meaning hundreds of frames of DOM thrashing before the graph settles.

**Consequences:** Users open the browser, see it freeze for 30+ seconds, and close the tab. The tool becomes unusable for any project above toy scale.

**Warning signs:**
- Any project with >300 files will likely exceed 1,000 symbols
- First load times over 3 seconds in the browser
- `simulation.alpha()` still high after 5 seconds

**Prevention:**
- Use Canvas rendering instead of SVG. D3-Canvas handles 2,000-5,000 nodes at interactive frame rates; D3-SVG breaks at ~500. This is the single highest-impact decision for performance.
- Run the force simulation to completion off-screen before rendering: call `simulation.stop()`, then `simulation.tick(300)` in a loop (or in a WebWorker), then render the final settled positions. Users see a fully-laid-out graph immediately rather than watching it settle.
- Do NOT render all nodes on initial load. Default to showing only the top N nodes by edge count (e.g., 100-200), with controls to expand. Code graphs are not "show everything" tools.
- If simulation must run live (watch mode), use `simulation.alphaDecay(0.05)` to converge faster and reduce the animation window.

**Phase:** Visualization phase. Canvas vs SVG decision must be made before writing the first rendering loop — it is a complete rewrite to change later.

**Sources:** [D3 force group discussion](https://groups.google.com/g/d3-js/c/nwf_Jafk_E8), [Neo4j scale-up article](https://medium.com/neo4j/scale-up-your-d3-graph-visualisation-part-2-2726a57301ec), [D3-Force optimization DZone](https://dzone.com/articles/d3-force-directed-graph-layout-optimization-in-neb)

---

### Pitfall 4: "Hairball" Initial View — All Nodes Shown at Once

**What goes wrong:** The tool opens in the browser and immediately renders every symbol from the codebase as a node. A 500-file TypeScript project produces 3,000+ nodes in a dense cluster. Users cannot read labels, cannot click meaningfully, and perceive the tool as broken rather than as a visualization of a complex codebase.

**Why it happens:** The temptation to "show everything" is strong — the whole value prop is seeing connections. But the human visual system cannot process more than ~50-100 labeled entities simultaneously. Graph visualization research calls this the "hairball problem."

**Consequences:** Users close the browser tab within 30 seconds. The tool fails its core UX mission despite working correctly technically.

**Warning signs:**
- Initial render shows a solid blob of nodes with no readable labels
- Any project with more than ~100 files

**Prevention:**
- Default view should show only files as nodes (not individual symbols), or the top 50 nodes by in-degree centrality
- Layer progressive disclosure: files → exported symbols → internal symbols
- First click on a node expands only its direct neighbors, not the transitive closure
- "Blast radius" and "dead code" views should be activated explicitly — do not auto-render them on load
- Provide a slider or threshold control for minimum edge count to include a node

**Phase:** Visualization/UX phase. Must be in the design spec before implementation begins.

**Sources:** [Cambridge Intelligence graph UX guide](https://cambridge-intelligence.com/graph-visualization-ux-how-to-avoid-wrecking-your-graph-visualization/)

---

### Pitfall 5: Dead Code False Positives from Barrel Files and Dynamic Patterns

**What goes wrong:** The tool marks symbols as dead code when they are actually alive. This is the "70% false positive rate" problem documented in real dead code detection tools. Common causes:

1. **Barrel files / index re-exports**: `fileA.ts` exports `doThing()`. `index.ts` does `export { doThing } from './fileA'`. `fileC.ts` imports from `'./index'`. A naive graph traces the import in fileC to index.ts, but does not follow the re-export chain back to fileA — so `doThing` in fileA appears unused.

2. **Dynamic import patterns**: `import(`./${featureName}`)` — the imported symbol is computed at runtime and cannot be statically resolved. Any symbol only reachable through a dynamic path will appear dead.

3. **Framework-specific entry points**: React components used in JSX (not called as functions), symbols consumed via `React.lazy()`, symbols registered in a router config by string name — none of these look like regular function calls to a static analyzer.

**Why it happens:** Tree-sitter gives you syntax, not semantics. Import resolution — following `from './index'` to what `index.ts` actually re-exports — requires building a resolution layer on top of the raw AST. This is the hardest part of the extractor, not the parsing.

**Consequences:** The OversizeConnect use case is finding dead code. If the tool reports false positives, developers will delete live code. If it reports false negatives, the tool has no value. Either direction breaks trust.

**Warning signs:**
- Any codebase using barrel/index files (React Native projects almost always do)
- Symbols only used in JSX attributes (`<Component />` not `Component()`)
- Anything using `React.lazy`, `require()`, or string-keyed lookups

**Prevention:**
- Implement multi-hop re-export resolution: when a file re-exports, trace through to the origin and mark the origin symbol as transitively used
- Mark all symbols that appear in JSX element names as used (not just explicit function calls)
- Add a "suspicious" confidence level rather than a binary dead/alive — show low-confidence dead code candidates differently from high-confidence ones
- Never auto-delete; show; let the developer decide
- Include the resolution path in the UI: "This symbol is used via: fileC → index.ts → fileA.ts"

**Phase:** Extractor phase (import resolution). Dead code accuracy depends entirely on getting this right.

**Sources:** [Dead code false positives Medium](https://medium.com/@usepharaoh/my-dead-code-detector-has-a-70-false-positive-rate-on-the-framework-everyone-uses-8f70353292f1), [ts-prune barrel file issue #40](https://github.com/pzavolinsky/ts-unused-exports/issues/123), [LogRocket dead code detection](https://blog.logrocket.com/how-detect-dead-code-frontend-project/)

---

### Pitfall 6: Watch Mode Event Storms and Stale Graph State

**What goes wrong:** Watch mode fires multiple file system events per single save. Text editors like VS Code write files atomically (temp file → rename → delete original), producing 3-5 events per save: `unlink`, `add`, `change`. A file format-on-save (Prettier runs, writes again) triggers a second wave. If each event triggers a full re-parse and graph rebuild, the system thrashes constantly during active editing. If events are dropped or processed out of order, the graph becomes stale: showing a symbol that was deleted, or missing a symbol that was added.

**Why it happens:** Chokidar (the standard file watcher for Node.js) exposes raw OS file system events. It does not know that 5 rapid events in 200ms represent one logical "user saved this file." Race conditions exist at the OS level: when chokidar sets up a watcher on a directory, there is a window between the initial directory scan and the watch registration where changes can be missed entirely.

**Consequences:** CPU spikes to 100% during active editing. Graph flickers or shows stale data. Watch mode becomes unusable as an "always-on dashboard."

**Warning signs:**
- Any text editor with format-on-save enabled (the primary development environment for React Native)
- Rename/move operations on files (generates `unlink` + `add`, not a single `rename`)
- Saving multiple files simultaneously (e.g., auto-import organizer saving all files)

**Prevention:**
- Debounce file events with a 300-500ms window: collect all events, process them as a batch after activity stops
- Use `awaitWriteFinish: { stabilityThreshold: 200, pollInterval: 100 }` in chokidar config to wait for write completion before firing the change event
- Implement incremental re-parse: only re-extract the changed file, then patch the graph (remove old edges for that file, insert new ones) rather than rebuilding from scratch
- Use a version counter on graph nodes: each re-parse increments the version; nodes with old version numbers at sweep time are stale and should be removed
- Test explicitly: save a file that is imported by 10 others and verify the graph updates correctly in one cycle without flickering

**Phase:** Watch mode phase. The debounce + incremental patch architecture must be designed upfront — adding it to a full-rebuild watch loop later requires significant rework.

**Sources:** [Chokidar race condition issue #1112](https://github.com/paulmillr/chokidar/issues/1112), [watch-debounced](https://github.com/eklingen/watch-debounced)

---

## Moderate Pitfalls

Mistakes that degrade quality or require significant rework but don't cause complete failure.

---

### Pitfall 7: Wrong Granularity in the Graph Data Model

**What goes wrong:** The graph is either too coarse (file-level nodes only, hides the function-call detail that makes blast radius useful) or too fine (every variable declaration as a node, produces an unusable million-node hairball). The granularity decision is baked into the data model and very expensive to change after extractors are written.

**Prevention:**
- Settle on the granularity hierarchy before writing extractors: `File → ExportedSymbol → InternalSymbol` with edge types `IMPORTS`, `CALLS`, `DEFINES`, `EXTENDS`, `IMPLEMENTS`
- Build the rendering layer to show different granularity levels (file view vs. symbol view) by filtering on node type, not by having different graph structures
- Test granularity on the OversizeConnect codebase specifically: count nodes and edges at each level before committing

**Phase:** Graph data model phase (first architectural decision). Changing granularity after extractors are written requires rewriting all extractors.

---

### Pitfall 8: Scanning node_modules and Generated Files

**What goes wrong:** The tool parses `node_modules/` (hundreds of thousands of files), TypeScript `.d.ts` declaration files, bundler output in `dist/` or `build/`, and generated `.pb.ts` files. This produces a graph with millions of nodes, most of which are irrelevant, and the initial scan takes minutes.

**Prevention:**
- Default exclude list must include: `node_modules`, `dist`, `build`, `.next`, `.expo`, `coverage`, `*.d.ts`, `*.generated.ts`, `*.pb.ts`
- Accept a `.codegraphignore` file (same format as `.gitignore`) for project-specific exclusions
- Log scan statistics at startup: "Scanning N files across M directories, skipping X paths"
- First-run experience on OversizeConnect should complete in under 10 seconds

**Phase:** CLI/extraction phase.

---

### Pitfall 9: TypeScript Path Aliases Not Resolved

**What goes wrong:** React Native projects (including OversizeConnect) use `tsconfig.json` `paths` aliases: `import { useUser } from '@hooks/useUser'` rather than `'../../hooks/useUser'`. Tree-sitter returns the raw import string `@hooks/useUser`. Without resolving the alias, the extractor sees this as an external package import and breaks all edges from this file to the hooks directory. The graph is structurally wrong.

**Prevention:**
- Parse `tsconfig.json` (and `babel.config.js` for React Native) at startup to extract `paths` and `moduleNameMapper`
- Resolve all import strings through the alias table before looking up target files
- Fall back gracefully: unresolvable imports should be flagged as "external" rather than silently dropped

**Phase:** TypeScript extractor phase. Must be solved before the OversizeConnect test case is useful.

---

### Pitfall 10: Cycle Handling in Blast Radius Traversal

**What goes wrong:** Computing blast radius requires a transitive graph traversal: "all nodes that transitively depend on symbol X." Code graphs regularly contain cycles (two files that import each other, common in React hook patterns). A naive DFS or BFS traversal without cycle detection enters an infinite loop and crashes with a stack overflow.

**Prevention:**
- Use an iterative BFS with an explicit `visited` Set — never recursive DFS for graph traversal in this domain
- Test blast radius on OversizeConnect's Zod schema files, which are a known circular dependency candidate
- Cap traversal depth to a configurable maximum (default 10 hops) with a visible indicator: "Showing direct and transitive dependents up to depth 10"

**Phase:** Query/traversal phase.

**Sources:** [Gradle stack overflow issue #22850](https://github.com/gradle/gradle/issues/22850)

---

### Pitfall 11: Inconsistent Extraction Quality Across Languages

**What goes wrong:** TypeScript extraction works well because it is the primary test case. Swift, Go, and Python extractors are built quickly to meet the "all 4 languages in v1" goal, but with lower quality: missing edge types, missed import patterns, wrong scope attribution. Users with Go or Python codebases get a worse product than users with TypeScript codebases. This discrepancy is not obvious until someone reports wrong results.

**Why it happens:** Each language has idiomatic patterns that are not obvious from the grammar alone:
- **Go**: package-level declarations, implicit interfaces, `init()` functions — edges come from method sets, not explicit inheritance
- **Python**: `__init__.py` re-export patterns, `*` imports, dynamic attribute lookup via `getattr` — very similar to the barrel file problem but worse
- **Swift**: `@objc` dynamic dispatch, protocol conformance, `extension` blocks that add methods to types defined elsewhere

**Prevention:**
- Define a baseline test fixture for each language: a small file with every relevant construct (imports, function calls, struct definitions, inheritance)
- Assert exact expected edges for each fixture — regression tests catch extractor drift
- Document explicitly which patterns each extractor does NOT handle (confidence level per language)
- Ship TypeScript as the primary extractor, mark others as "beta" in v1 documentation

**Phase:** Per-language extractor phases.

---

## Minor Pitfalls

---

### Pitfall 12: WASM vs Native Binding Choice Has Long-Term Consequences

**What goes wrong:** Starting with native bindings (faster, less hassle in development) then being forced to switch to WASM for distribution reasons (or vice versa) is a significant rework. The two APIs are not identical: WASM (`web-tree-sitter`) requires explicit memory management (`tree.delete()`), returns slightly different node types, and does not support custom query predicates available in native bindings.

**Prevention:**
- Make the decision explicitly and early: WASM is the safer default for an npm CLI tool distributed to users with unknown system configurations. Native is appropriate only if the WASM performance penalty (roughly 2-3x slower parsing) is measurable and unacceptable.
- If using WASM, write a wrapper layer that hides the memory management from extractor code — all `tree.delete()` calls should happen in one place

**Phase:** Parser foundation phase.

**Sources:** [Pulsar tree-sitter pain points](https://blog.pulsar-edit.dev/posts/20240902-savetheclocktower-modern-tree-sitter-part-7/)

---

### Pitfall 13: npx First-Run Slowness Poisons First Impressions

**What goes wrong:** `npx code-graph ./my-project` downloads the package on first run. If the package has native dependencies, `node-gyp` triggers a C++ compile during download. On a clean machine this takes 60-120 seconds. On Windows it frequently fails outright. The user's first interaction with the tool is a minute of compiler output.

**Prevention:**
- Use `node-pre-gyp` or `prebuild` to ship prebuilt binaries for the common platforms (darwin-arm64, darwin-x64, linux-x64, win32-x64) — users download a binary, not a compile
- Keep total package size under 5MB to minimize download time
- Print a friendly startup message immediately before any heavy work: "Scanning [path]... (first run may take a moment)"
- Consider whether the WASM path eliminates native compilation entirely for the common case

**Phase:** Distribution/packaging phase.

---

### Pitfall 14: Missing Edge Types Make the Graph Misleading

**What goes wrong:** The graph shows `CALLS` edges but not `EXTENDS` or `IMPLEMENTS` edges. A developer investigating blast radius of a base class gets incomplete results — subtypes that override methods are invisible to the graph. Or the graph shows `IMPORTS` but not `REFERENCES` (a file imported but only used for its type, not at runtime). Both produce misleading dead code analysis.

**Prevention:**
- Define the full edge type vocabulary before writing the first extractor: `IMPORTS`, `CALLS`, `DEFINES`, `EXTENDS`, `IMPLEMENTS`, `INSTANTIATES`, `TYPE_REFERENCES`
- Each extractor must implement all edge types or explicitly document which are missing (with a TODO)
- The UI must filter by edge type — "show only CALLS edges" for blast radius, "show only IMPORTS" for module dependency view

**Phase:** Graph data model phase, alongside granularity decisions.

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|----------------|------------|
| Parser foundation | Native binding ABI mismatch on user machines | Lock versions, test on 3 Node LTS versions, consider WASM default |
| Parser foundation | Grammar/runtime ABI mismatch producing silent wrong output | Lock all tree-sitter packages to exact versions, test for ERROR nodes |
| Graph data model | Wrong granularity baked in before extractors are written | Decide file/symbol/internal hierarchy explicitly before coding |
| TypeScript extractor | Path aliases (`@hooks/`) break all cross-file edges | Parse tsconfig.json `paths` at startup |
| TypeScript extractor | Barrel file re-exports cause dead code false positives | Multi-hop re-export resolution |
| Visualization | SVG rendering freezes browser on real codebases | Canvas rendering, pre-settle simulation before render |
| Visualization | All nodes visible on load creates hairball | Progressive disclosure, default to top-N by centrality |
| Watch mode | Event storm from format-on-save causes thrashing | Debounce 300-500ms, incremental patch not full rebuild |
| Watch mode | Stale graph after file rename/delete | Version-counter sweep, explicit unlink handling |
| Dead code feature | False positives on dynamic imports and JSX | Confidence levels, never auto-delete, show resolution path |
| Blast radius feature | Infinite loop on cyclic dependencies | Iterative BFS with visited Set, depth cap |
| Go/Swift/Python extractors | Idiomatic patterns missed (protocol conformance, `init()`, `*` imports) | Fixture-based regression tests per language |
| npm distribution | npx compilation fails on Windows/Linux clean machines | Prebuilt binaries via prebuild or node-pre-gyp |

---

## Sources

- [tree-sitter ABI mismatch issue #2867](https://github.com/tree-sitter/tree-sitter/issues/2867)
- [tree-sitter node bindings ABI issue #188](https://github.com/tree-sitter/node-tree-sitter/issues/188)
- [tree-sitter npm version lag issue #5334](https://github.com/tree-sitter/tree-sitter/issues/5334)
- [tree-sitter grammar ABI version issue #151](https://github.com/tree-sitter/tree-sitter/issues/151)
- [tree-sitter node-types.json ABI break #4598](https://github.com/tree-sitter/tree-sitter/issues/4598)
- [Pulsar tree-sitter pain points (WASM, memory, predicates)](https://blog.pulsar-edit.dev/posts/20240902-savetheclocktower-modern-tree-sitter-part-7/)
- [Tree-sitter packaging mess blog post](https://ayats.org/blog/tree-sitter-packaging)
- [D3 force graph performance discussion](https://groups.google.com/g/d3-js/c/nwf_Jafk_E8)
- [Neo4j: Scale up your D3 graph visualisation part 2](https://medium.com/neo4j/scale-up-your-d3-graph-visualisation-part-2-2726a57301ec)
- [D3 force layout Nebula Graph optimization](https://dzone.com/articles/d3-force-directed-graph-layout-optimization-in-neb)
- [Cambridge Intelligence: Graph visualization UX pitfalls](https://cambridge-intelligence.com/graph-visualization-ux-how-to-avoid-wrecking-your-graph-visualization/)
- [Dead code 70% false positive rate (Medium)](https://medium.com/@usepharaoh/my-dead-code-detector-has-a-70-false-positive-rate-on-the-framework-everyone-uses-8f70353292f1)
- [LogRocket: Detecting dead code in frontend projects](https://blog.logrocket.com/how-detect-dead-code-frontend-project/)
- [ts-unused-exports dynamic import false positive](https://github.com/pzavolinsky/ts-unused-exports/issues/123)
- [Chokidar race condition on directory watch](https://github.com/paulmillr/chokidar/issues/1112)
- [Gradle stack overflow on cyclic dependency traversal](https://github.com/gradle/gradle/issues/22850)
