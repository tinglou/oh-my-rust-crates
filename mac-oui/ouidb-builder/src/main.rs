mod convert;
mod corp_suffix;
mod download;
mod region_map;

use clap::{Parser, Subcommand};
use std::process;

#[derive(Parser)]
#[command(name = "ouidb-builder", about = "IEEE OUI database builder")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Download IEEE OUI CSV files to db/
    Download,
    /// Convert downloaded CSVs to a single simplified file
    Convert {
        /// Keep corporate suffixes (LLC, Inc., Ltd., etc.)
        #[arg(long, default_value_t = false)]
        keep_suffix: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Download => download::run(),
        Commands::Convert { keep_suffix } => convert::run(keep_suffix),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}