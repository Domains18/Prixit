mod error;
mod models;
mod cli;
mod parser;
mod analyzer;
mod reporter;

use clap::Parser;
use cli::{Cli, Commands};
use colored::*;
use parser::migration::MigrationDiscovery;
use reporter::OutputFormat;

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze { path, format } => {
            println!("{}", "Analyzing migrations...".bright_blue().bold());

            match run_analysis(&path, &format) {
                Ok(has_errors) => {
                    println!("\n{}", "Analysis complete!".green().bold());
                    if has_errors {
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                    std::process::exit(1)
                }
            }
        }

        Commands::Init => {
            println!("{}", "initializing migration-analyzer...".bright_blue());
            //TODO: Create a config file
        }
    }
}

fn run_analysis(path: &str, format: &str) -> error::Result<bool> {
    let migrations = MigrationDiscovery::discover(path)?;

    if migrations.is_empty() {
        println!("{}", "No migrations found.".yellow());
        return Ok(false);
    }

    println!(
        "Found {} migration(s). Running analysis...\n",
        migrations.len()
    );

    let findings = analyzer::analyze(&migrations);

    let output_format = OutputFormat::from_str(format);
    reporter::report(&findings, &output_format);

    let has_errors = findings.iter().any(|f| {
        matches!(f.severity, models::Severity::Error)
    });

    Ok(has_errors)
}
