use ghostmark_core::text_scrubber;
use ghostmark_core::image_stripper;
mod proxy;

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
    /// Strips cryptographic C2PA and tracking metadata from JPEG/PNG images
    CleanImage {
        /// The path to the input image
        #[arg(short, long)]
        input: String,

        /// The path to save the cleaned output image
        #[arg(short, long)]
        output: String,
    },
    /// Starts the GhostMark HTTP proxy server
    Serve {
        /// Host address to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Port to listen on
        #[arg(short, long, default_value_t = 8080)]
        port: u16,
    },
}

#[tokio::main]
async fn main() {
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
        },
        Commands::CleanImage { input, output } => {
            match image_stripper::strip_image_metadata(input, output) {
                Ok(_) => println!("✅ Successfully stripped all tracking metadata from {} -> {}", input, output),
                Err(e) => {
                    eprintln!("❌ Error stripping image metadata: {}", e);
                    std::process::exit(1);
                }
            }
        },
        Commands::Serve { host, port } => {
            proxy::start_server(host, *port).await;
        }
    }
}
