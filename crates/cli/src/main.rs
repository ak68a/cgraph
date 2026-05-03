use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{bail, Result};
use clap::Parser;
use cgraph_indexer::{Indexer, dead_code, detect_cycles, DeadCodeResult, DeadCodeEntry, Confidence, CycleResult};
use cgraph_ts_extractor::TsExtractor;
use cgraph_core::Extractor;
use cgraph_server::{ScanStats, AppState, enriched_projection, find_available_port};

#[derive(Parser, Debug)]
#[command(version, about = "Code graph visualization — cgraph")]
pub struct Cli {
    /// Path to the project directory to scan
    pub path: PathBuf,

    /// Print verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Print detailed dead code report grouped by file
    #[arg(long)]
    pub dead_code: bool,

    /// Print detailed circular dependency report
    #[arg(long)]
    pub cycles: bool,

    /// Do not auto-open browser (for headless/CI use)
    #[arg(long)]
    pub no_open: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Validate path exists and is a directory (security: T-01-05)
    if !cli.path.exists() {
        bail!("Path does not exist: {}", cli.path.display());
    }
    if !cli.path.is_dir() {
        bail!("Path is not a directory: {}", cli.path.display());
    }

    // Build extractor registry (D-48: dynamic, language-agnostic)
    let extractors: Vec<Box<dyn Extractor>> = vec![
        Box::new(TsExtractor::new()),
    ];

    // Run indexer with timing (INFR-03)
    let indexer = Indexer::new(extractors);
    let start = Instant::now();
    let code_graph = indexer.index(&cli.path)?;
    let elapsed = start.elapsed();

    // Print scan statistics (INFR-03, D-43)
    println!(
        "cgraph scan: {} files, {} symbols, {} edges ({:.0}ms)",
        code_graph.file_count(),
        code_graph.node_count(),
        code_graph.edge_count(),
        elapsed.as_secs_f64() * 1000.0
    );

    // Run analysis
    let dead_result = dead_code(&code_graph, &cli.path);
    let cycle_result = detect_cycles(&code_graph);

    // Print analysis summary (D-43: always shown in default output)
    println!();
    println!("analysis:");
    println!(
        "  dead code: {} confirmed, {} suspicious",
        dead_result.confirmed.len(),
        dead_result.suspicious.len()
    );
    println!(
        "  circular dependencies: {}",
        cycle_result.cycles.len()
    );

    // Print detailed dead code report if --dead-code flag (D-44)
    if cli.dead_code {
        println!();
        print_dead_code_report(&dead_result);
    }

    // Print detailed cycles report if --cycles flag (D-44)
    if cli.cycles {
        println!();
        print_cycles_report(&cycle_result);
    }

    // -- Server startup (Phase 4: INFR-02) --

    // Derive project name from the scanned path (last directory component)
    let project_name = cli.path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "cgraph".to_string());

    // Build ScanStats for the API response
    let stats = ScanStats {
        files: code_graph.file_count(),
        symbols: code_graph.node_count(),
        edges: code_graph.edge_count(),
        elapsed_ms: elapsed.as_millis() as u64,
    };

    // Pre-compute enriched projection (file nodes + symbol nodes + typed edges + dead code flags)
    let file_graph = enriched_projection(&code_graph, &dead_result, stats, project_name);
    let state = AppState {
        file_graph: Arc::new(file_graph),
    };

    // Find available port (D-60: start at 3000, increment if taken)
    let (port, listener) = find_available_port(3000).await?;
    let url = format!("http://localhost:{}", port);

    // Spawn server using cgraph_server::serve() wrapper (encapsulates axum)
    tokio::spawn(async move {
        if let Err(e) = cgraph_server::serve(listener, state).await {
            eprintln!("Server error: {}", e);
        }
    });

    // Open browser (D-62: auto-open, --no-open suppresses)
    if cli.no_open {
        println!("cgraph listening on {} -- open in browser to view graph", url);
    } else {
        println!("cgraph listening on {} -- opening browser...", url);
        if let Err(e) = webbrowser::open(&url) {
            eprintln!("Could not open browser: {}. Open manually: {}", e, url);
        }
    }

    // Block until Ctrl-C
    println!("Press Ctrl-C to stop the server.");
    tokio::signal::ctrl_c().await?;
    println!("\nShutting down.");

    Ok(())
}

fn print_dead_code_report(result: &DeadCodeResult) {
    if result.confirmed.is_empty() && result.suspicious.is_empty() {
        println!("dead code: none found");
        return;
    }

    // Group confirmed entries by file path (D-44: grouped by file for scannability)
    if !result.confirmed.is_empty() {
        println!("dead code — confirmed ({}):", result.confirmed.len());
        print_entries_by_file(&result.confirmed);
    }

    if !result.suspicious.is_empty() {
        println!();
        println!("dead code — suspicious ({}):", result.suspicious.len());
        print_entries_by_file(&result.suspicious);
    }
}

fn print_entries_by_file(entries: &[DeadCodeEntry]) {
    let mut by_file: HashMap<&str, Vec<&DeadCodeEntry>> = HashMap::new();
    for entry in entries {
        by_file.entry(entry.file_path.as_str()).or_default().push(entry);
    }
    // Sort file paths for deterministic output
    let mut sorted_files: Vec<&str> = by_file.keys().copied().collect();
    sorted_files.sort();
    for file in sorted_files {
        println!("  {}", file);
        let file_entries = &by_file[file];
        for entry in file_entries {
            match &entry.confidence {
                Confidence::Confirmed => {
                    println!(
                        "    {:?} {} (lines {}-{})",
                        entry.kind, entry.symbol_name, entry.line_start, entry.line_end
                    );
                }
                Confidence::Suspicious(reason) => {
                    println!(
                        "    {:?} {} (lines {}-{}) — {}",
                        entry.kind, entry.symbol_name, entry.line_start, entry.line_end, reason
                    );
                }
            }
        }
    }
}

fn print_cycles_report(result: &CycleResult) {
    if result.cycles.is_empty() {
        println!("circular dependencies: none found");
        return;
    }

    println!("circular dependencies ({}):", result.cycles.len());
    for (i, cycle) in result.cycles.iter().enumerate() {
        println!("  cycle {}:", i + 1);
        for file in cycle {
            println!("    -> {}", file);
        }
        // Show the cycle closing back to the first file
        if let Some(first) = cycle.first() {
            println!("    -> {} (cycle)", first);
        }
    }
}
