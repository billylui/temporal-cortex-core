//! `toon` CLI — encode, decode, and analyze TOON files from the command line.
//!
//! ## Usage
//!
//! ```sh
//! # Encode JSON to TOON (stdin → stdout)
//! echo '{"name":"Alice","age":30}' | toon encode
//!
//! # Encode from file to file
//! toon encode -i data.json -o data.toon
//!
//! # Encode with field filtering
//! echo '{"name":"Event","etag":"abc"}' | toon encode --filter etag
//!
//! # Encode with Google Calendar preset filter
//! toon encode --filter-preset google -i calendar.json
//!
//! # Decode TOON back to pretty-printed JSON
//! toon decode -i data.toon
//!
//! # Show compression statistics
//! toon stats -i data.json
//!
//! # Managed Cortex mode (stub)
//! toon --managed-cortex --api-key YOUR_KEY
//! ```

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::io::{self, Read};
use std::process;
use toon_core::CalendarFilter;

#[derive(Parser)]
#[command(
    name = "toon",
    version,
    about = "TOON (Token-Oriented Object Notation) CLI"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Connect to the managed Temporal Cortex service
    #[arg(long)]
    managed_cortex: bool,

    /// API key for the managed Cortex service (requires --managed-cortex)
    #[arg(long, requires = "managed_cortex")]
    api_key: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Encode JSON to TOON format
    Encode {
        /// Input file (reads from stdin if omitted)
        #[arg(short, long)]
        input: Option<String>,
        /// Output file (writes to stdout if omitted)
        #[arg(short, long)]
        output: Option<String>,
        /// Comma-separated field patterns to strip before encoding
        #[arg(long)]
        filter: Option<String>,
        /// Use a predefined filter preset (e.g., "google" for Google Calendar)
        #[arg(long)]
        filter_preset: Option<String>,
    },
    /// Decode TOON back to JSON format
    Decode {
        /// Input file (reads from stdin if omitted)
        #[arg(short, long)]
        input: Option<String>,
        /// Output file (writes to stdout if omitted)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Show encoding statistics (token counts, compression ratio)
    Stats {
        /// Input JSON file (reads from stdin if omitted)
        #[arg(short, long)]
        input: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle --managed-cortex before subcommands
    if cli.managed_cortex {
        if cli.api_key.is_none() {
            println!("Error: --managed-cortex requires --api-key. Sign up at https://temporal-cortex.dev to get an API key.");
        } else {
            println!("Managed Cortex mode is not yet available. Coming in Phase 4.");
        }
        process::exit(0);
    }

    // If no subcommand was provided and we're not in managed-cortex mode,
    // print help and exit.
    let command = match cli.command {
        Some(cmd) => cmd,
        None => {
            // Re-parse with --help to show usage (clap handles this)
            Cli::parse_from(["toon", "--help"]);
            unreachable!();
        }
    };

    match command {
        Commands::Encode {
            input,
            output,
            filter,
            filter_preset,
        } => {
            let json = read_input(input.as_deref())?;

            // Build the filter patterns from --filter and/or --filter-preset
            let patterns = build_filter_patterns(filter.as_deref(), filter_preset.as_deref())?;

            let toon = if patterns.is_empty() {
                toon_core::encode(&json).context("Failed to encode JSON to TOON")?
            } else {
                let pattern_refs: Vec<&str> = patterns.iter().map(|s| s.as_str()).collect();
                toon_core::filter_and_encode(&json, &pattern_refs)
                    .context("Failed to filter and encode JSON to TOON")?
            };

            write_output(output.as_deref(), &toon)?;
        }
        Commands::Decode { input, output } => {
            let toon = read_input(input.as_deref())?;
            let json = toon_core::decode(&toon).context("Failed to decode TOON to JSON")?;
            // Pretty-print the JSON output
            let value: serde_json::Value = serde_json::from_str(&json)?;
            let pretty = serde_json::to_string_pretty(&value)?;
            write_output(output.as_deref(), &pretty)?;
        }
        Commands::Stats { input } => {
            let json = read_input(input.as_deref())?;
            let toon = toon_core::encode(&json).context("Failed to encode JSON to TOON")?;
            let json_bytes = json.len();
            let toon_bytes = toon.len();
            let ratio = if json_bytes > 0 {
                (1.0 - (toon_bytes as f64 / json_bytes as f64)) * 100.0
            } else {
                0.0
            };
            println!("JSON size:  {} bytes", json_bytes);
            println!("TOON size:  {} bytes", toon_bytes);
            println!("Reduction:  {:.1}%", ratio);
        }
    }

    Ok(())
}

/// Build filter patterns from the --filter and --filter-preset arguments.
///
/// - `--filter etag,kind` produces `["etag", "kind"]`
/// - `--filter-preset google` produces the Google Calendar default patterns
/// - Both can be combined (patterns are merged)
/// - An empty --filter string produces no patterns (preserves all fields)
fn build_filter_patterns(filter: Option<&str>, filter_preset: Option<&str>) -> Result<Vec<String>> {
    let mut patterns = Vec::new();

    if let Some(raw) = filter {
        for part in raw.split(',') {
            let trimmed = part.trim();
            if !trimmed.is_empty() {
                patterns.push(trimmed.to_string());
            }
        }
    }

    if let Some(preset) = filter_preset {
        match preset {
            "google" => {
                for p in CalendarFilter::google_default() {
                    patterns.push(p.to_string());
                }
            }
            other => {
                anyhow::bail!(
                    "Unknown filter preset: '{}'. Available presets: google",
                    other
                );
            }
        }
    }

    Ok(patterns)
}

fn read_input(path: Option<&str>) -> Result<String> {
    match path {
        Some(path) => {
            std::fs::read_to_string(path).with_context(|| format!("Failed to read file: {}", path))
        }
        None => {
            let mut buf = String::new();
            io::stdin()
                .read_to_string(&mut buf)
                .context("Failed to read from stdin")?;
            Ok(buf)
        }
    }
}

fn write_output(path: Option<&str>, content: &str) -> Result<()> {
    match path {
        Some(path) => {
            std::fs::write(path, content)
                .with_context(|| format!("Failed to write file: {}", path))?;
        }
        None => {
            print!("{}", content);
        }
    }
    Ok(())
}
