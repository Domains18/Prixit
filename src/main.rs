mod error;
mod models;
mod cli;
mod models;
mod parser;


use clap::Parser;
use cli::{Cli, Commands};
use colored::*;

fn main() {
    let cli = Cli::parse();


    match cli.command {
        Commands::Analyze { path, format } =>{
            println!("{}", "Analyzing migrations...".bright_blue().bold());

            match run_analysis(&path, &format) {
                Ok(_) => println!("\n{}", "Analysis complete!".green().bold()),
                Err(e) => {
                    eprintln!("{} {}", "Error:".red().bold());
                    std::process::exit(1)
                }
            }
        }

        Commands::Init =>{
            println!("{}", "initializing migration-analyzer...".bright_blue());
            //TODO: Create a config file
        }
    }

    fn run_analysis(path: &str, format: &str)-> error::Result<()>{
        println!("analyzing migrations at: {}", path);
        println!("output format: {}", format);
        Ok(())
    }

}
