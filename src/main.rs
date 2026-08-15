mod text_scrubber;

use clap::{Parser, Subcommand};
use std::fs;

#[derive(Parser)]
#[command(author, version, about = "GhostMark: AI Watermark Stripper", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Strips invisible unicode watermarks from text
    CleanText {
        /// The text string to clean, or a file path if --file is used
        input: String,

        /// Treat the input argument as a file path
        #[arg(short, long)]
        file: bool,

        /// Optional output file path (if not provided, prints to stdout)
        #[arg(short, long)]
        output: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::CleanText { input, file, output } => {
            let text = if *file {
                fs::read_to_string(input).unwrap_or_else(|err| {
                    eprintln!("Error reading file '{}': {}", input, err);
                    std::process::exit(1);
                })
            } else {
                input.clone()
            };

            let clean = text_scrubber::sanitize_text(&text);

            if let Some(out_path) = output {
                fs::write(out_path, &clean).unwrap_or_else(|err| {
                    eprintln!("Error writing file '{}': {}", out_path, err);
                    std::process::exit(1);
                });
                println!("Successfully cleaned text and saved to {}", out_path);
            } else {
                println!("{}", clean);
            }
        }
    }
}
