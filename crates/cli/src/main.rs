use anyhow::{bail, Result};
use clap::Parser;
use std::collections::HashMap;
use std::path::PathBuf;
use cgraph_core::{scan_directory, DetectionResult};

#[derive(Parser, Debug)]
#[command(version, about = "Code graph visualization — cgraph")]
pub struct Cli {
    /// Path to the project directory to scan
    pub path: PathBuf,

    /// Print verbose output including per-file language detection
    #[arg(short, long)]
    pub verbose: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Validate path exists and is a directory (security: T-01-05)
    if !cli.path.exists() {
        bail!("Path does not exist: {}", cli.path.display());
    }
    if !cli.path.is_dir() {
        bail!("Path is not a directory: {}", cli.path.display());
    }

    // Run language detection via core
    let result = scan_directory(&cli.path)?;

    // Print scan summary per D-19, D-11
    print_summary(&result, cli.verbose);

    Ok(())
}

fn print_summary(result: &DetectionResult, verbose: bool) {
    // Count files by language for the parseable set
    let mut parseable_counts: HashMap<String, usize> = HashMap::new();
    for (_, lang) in &result.parseable {
        let key = format!("{:?}", lang);
        *parseable_counts.entry(key).or_insert(0) += 1;
    }

    // Count skipped by extension
    let mut skipped_counts: HashMap<String, usize> = HashMap::new();
    for (_, ext) in &result.skipped {
        *skipped_counts.entry(ext.clone()).or_insert(0) += 1;
    }

    let total = result.detected.len() + result.skipped.len();
    println!("cgraph scan summary");
    println!("===================");
    println!("Total files found: {}", total);
    println!();

    // Parseable languages
    if result.parseable.is_empty() {
        println!("Parseable: none");
    } else {
        println!("Parseable ({} files):", result.parseable.len());
        // Sort for deterministic output
        let mut sorted: Vec<_> = parseable_counts.iter().collect();
        sorted.sort_by_key(|(k, _)| k.as_str());
        for (lang, count) in sorted {
            println!("  {} — {} files", lang, count);
        }
    }
    println!();

    // Skipped (unsupported)
    if result.skipped.is_empty() {
        println!("Skipped: none");
    } else {
        println!("Skipped ({} files — not parseable):", result.skipped.len());
        let mut sorted: Vec<_> = skipped_counts.iter().collect();
        sorted.sort_by_key(|(k, _)| k.as_str());
        for (ext, count) in sorted {
            println!("  .{} — {} files", ext, count);
        }
    }

    // Verbose: list every file
    if verbose {
        println!();
        println!("--- Detailed file list ---");
        for (path, lang) in &result.parseable {
            println!("  [parseable] {:?} — {}", lang, path.display());
        }
        for (path, ext) in &result.skipped {
            println!("  [skipped]   .{} — {}", ext, path.display());
        }
    }
}
