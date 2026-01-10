

use clap::{Parser, Subcommand};

#[derive({Parser})]
#[command(name = "migration-analyzer")]
#[command(about = "analyze Prisma Migrations for issues and Optimizations", long_about = None)]
pub struct Cli {
    #[command(Subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Analyze {
        #[arg(short, long, default_value = "prisma/migrations")]
        path: String,

        #[arg(short, long, default_value = "console")]
        format: String,
    },

    Init,
}
