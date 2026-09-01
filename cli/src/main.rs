use ghostmark_core::document_stripper;
use ghostmark_core::eval;
use ghostmark_core::image_stripper;
use ghostmark_core::text_scrubber;
mod proxy;

use clap::{Parser, Subcommand};
use serde_json::json;
use std::fs;
use walkdir::WalkDir;

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

        /// Aggressively perturb tokens to shatter statistical watermarks like SynthID-Text
        #[arg(long)]
        shatter_synthid: bool,
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
    /// Recursively batch-cleans all text and image files in a directory in-place
    BatchClean {
        /// The target directory
        #[arg(short, long)]
        dir: String,

        /// Aggressively perturb tokens to shatter statistical watermarks like SynthID-Text
        #[arg(long)]
        shatter_synthid: bool,
    },
    /// Process text using a local Ollama server before applying homoglyphs
    Ollama {
        /// The text string to clean, or a file path if --file is used
        input: String,

        /// Treat the input argument as a file path
        #[arg(short, long)]
        file: bool,

        /// The Ollama model to use
        #[arg(short, long, default_value = "llama3")]
        model: String,

        /// Optional output file path
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Run the water removal eval harness (green/red-list oracle) on text
    Eval {
        /// The text string to evaluate, or a file path if --file is used
        /// (not needed when --demo is given)
        input: Option<String>,

        /// Treat the input argument as a file path
        #[arg(short, long)]
        file: bool,

        /// Secret key (u64) for the watermark oracle. Defaults to 1
        #[arg(long, default_value_t = 1)]
        key: u64,

        /// Also run the built-in demo corpus instead of reading input
        #[arg(long)]
        demo: bool,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::CleanText {
            input,
            file,
            output,
            shatter_synthid,
        } => {
            let text = if *file {
                fs::read_to_string(input).unwrap_or_else(|err| {
                    eprintln!("Error reading file '{}': {}", input, err);
                    std::process::exit(1);
                })
            } else {
                input.clone()
            };

            let clean = if *shatter_synthid {
                text_scrubber::shatter_synthid_text(&text)
            } else {
                text_scrubber::sanitize_text(&text, false)
            };

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
        Commands::CleanImage { input, output } => {
            match image_stripper::strip_image_metadata(input, output) {
                Ok(_) => println!(
                    "✅ Successfully stripped all tracking metadata from {} -> {}",
                    input, output
                ),
                Err(e) => {
                    eprintln!("❌ Error stripping image metadata: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Serve { host, port } => {
            proxy::start_server(host, *port).await;
        }
        Commands::BatchClean {
            dir,
            shatter_synthid,
        } => {
            println!("🔍 Scanning directory: {}", dir);
            let mut cleaned_files = 0;

            for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_file() {
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_lowercase();

                    if ext == "txt" || ext == "md" || ext == "json" {
                        if let Ok(text) = fs::read_to_string(path) {
                            let clean = if *shatter_synthid {
                                text_scrubber::shatter_synthid_text(&text)
                            } else {
                                text_scrubber::sanitize_text(&text, false)
                            };
                            if text != clean {
                                fs::write(path, clean).unwrap_or_else(|e| {
                                    eprintln!("Failed to write {}: {}", path.display(), e)
                                });
                                println!("✅ Scrubbed text: {}", path.display());
                                cleaned_files += 1;
                            }
                        }
                    } else if ext == "png"
                        || ext == "jpg"
                        || ext == "jpeg"
                        || ext == "webp"
                        || ext == "bmp"
                        || ext == "gif"
                    {
                        // Pass same input and output path for in-place edit
                        let path_str = path.to_str().unwrap();
                        if image_stripper::strip_image_metadata(path_str, path_str).is_ok() {
                            println!("✅ Stripped image metadata: {}", path_str);
                            cleaned_files += 1;
                        }
                    } else if ext == "pdf" {
                        if let Ok(bytes) = fs::read(path) {
                            if let Ok(clean) = document_stripper::strip_pdf_metadata(&bytes) {
                                fs::write(path, clean).unwrap_or_else(|e| {
                                    eprintln!("Failed to write {}: {}", path.display(), e)
                                });
                                println!("✅ Stripped PDF metadata: {}", path.display());
                                cleaned_files += 1;
                            }
                        }
                    } else if ext == "docx" {
                        if let Ok(bytes) = fs::read(path) {
                            if let Ok(clean) = document_stripper::strip_docx_metadata(&bytes) {
                                fs::write(path, clean).unwrap_or_else(|e| {
                                    eprintln!("Failed to write {}: {}", path.display(), e)
                                });
                                println!("✅ Stripped DOCX metadata: {}", path.display());
                                cleaned_files += 1;
                            }
                        }
                    } else if ext == "svg" {
                        if let Ok(bytes) = fs::read(path) {
                            if let Ok(clean) = document_stripper::strip_svg_metadata(&bytes) {
                                fs::write(path, clean).unwrap_or_else(|e| {
                                    eprintln!("Failed to write {}: {}", path.display(), e)
                                });
                                println!("✅ Stripped SVG metadata: {}", path.display());
                                cleaned_files += 1;
                            }
                        }
                    } else if ext == "epub" {
                        if let Ok(bytes) = fs::read(path) {
                            if let Ok(clean) = document_stripper::strip_epub_metadata(&bytes) {
                                fs::write(path, clean).unwrap_or_else(|e| {
                                    eprintln!("Failed to write {}: {}", path.display(), e)
                                });
                                println!("✅ Stripped EPUB metadata: {}", path.display());
                                cleaned_files += 1;
                            }
                        }
                    } else if ext == "odt" {
                        if let Ok(bytes) = fs::read(path) {
                            if let Ok(clean) = document_stripper::strip_odt_metadata(&bytes) {
                                fs::write(path, clean).unwrap_or_else(|e| {
                                    eprintln!("Failed to write {}: {}", path.display(), e)
                                });
                                println!("✅ Stripped ODT metadata: {}", path.display());
                                cleaned_files += 1;
                            }
                        }
                    }
                }
            }
            println!("🎉 Batch complete! Scrubbed {} files.", cleaned_files);
        }
        Commands::Ollama {
            input,
            file,
            model,
            output,
        } => {
            let text = if *file {
                fs::read_to_string(input).unwrap_or_else(|err| {
                    eprintln!("Error reading file '{}': {}", input, err);
                    std::process::exit(1);
                })
            } else {
                input.clone()
            };

            let clean_base = text_scrubber::sanitize_text(&text, false);

            println!(
                "🤖 Sending to Ollama server ({}) for deep rewriting...",
                model
            );
            let client = reqwest::Client::new();

            let req_body = json!({
                "model": model,
                "system": "You are an expert editor. Rewrite the user's text to sound conversational and human. You MUST preserve the exact same meaning, names, genders, and pronouns (he/she/they) as the original. Output only the rewritten text.",
                "prompt": clean_base,
                "stream": false
            });

            match client
                .post("http://localhost:11434/api/generate")
                .json(&req_body)
                .send()
                .await
            {
                Ok(resp) => {
                    if let Ok(json_resp) = resp.json::<serde_json::Value>().await {
                        if let Some(response_str) = json_resp["response"].as_str() {
                            let final_text = text_scrubber::apply_homoglyphs(response_str);

                            if let Some(out_path) = output {
                                fs::write(out_path, &final_text).unwrap_or_else(|err| {
                                    eprintln!("Error writing file '{}': {}", out_path, err);
                                    std::process::exit(1);
                                });
                                println!("✅ Successfully rewritten and saved to {}", out_path);
                            } else {
                                println!("\n{}", final_text);
                            }
                        } else {
                            eprintln!("❌ Unexpected response format from Ollama.");
                        }
                    } else {
                        eprintln!("❌ Failed to parse response from Ollama.");
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to reach Ollama server: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Eval {
            input,
            file,
            key,
            demo,
        } => {
            let text = if *demo {
                eval::DEMO_CORPUS.to_string()
            } else if *file {
                let path = input.as_deref().unwrap_or_default();
                fs::read_to_string(path).unwrap_or_else(|err| {
                    eprintln!("Error reading file '{}': {}", path, err);
                    std::process::exit(1);
                })
            } else {
                input
                    .clone()
                    .unwrap_or_else(|| {
                        eprintln!("No input provided. Pass text, --file <path>, or --demo.");
                        std::process::exit(1);
                    })
            };

            let report = eval::run_eval(&text, *key);
            println!("🧪 GhostMark Eval Harness");
            println!("========================");
            println!(
                "Corpus   : {} tokens ({} words)",
                report.tokens, report.tokens
            );
            println!("Oracle key: {}", report.key);
            println!(
                "Before   : z = {:.2}  (green {:.1}%)  {}{}",
                report.z_before,
                report.green_before * 100.0,
                if report.z_before >= eval::Z_THRESHOLD {
                    "WATERMARKED"
                } else {
                    "no signal"
                },
                if report.tokens < 20 {
                    "  ⚠️  too few tokens for a reliable verdict"
                } else {
                    ""
                }
            );
            println!();
            println!("Pipeline            | z-score | verdict");
            println!("--------------------|---------|------------------");
            println!(
                "Fast WASM scrub     |  {:>5.2}  | {}",
                report.z_fast,
                verdict(report.z_fast)
            );
            println!(
                "Fast + Homoglyph    |  {:>5.2}  | {}",
                report.z_homoglyph,
                verdict(report.z_homoglyph)
            );
            println!(
                "Shatter SynthID     |  {:>5.2}  | {}",
                report.z_shatter,
                verdict(report.z_shatter)
            );
            println!();
            println!(
                "Fidelity  (shatter): {:.0}% of word tokens preserved",
                report.fidelity_shatter * 100.0
            );
            println!(
                "Entropy   (shatter): {:.2} → {:.2} bits/token",
                report.entropy_before, report.entropy_shatter
            );
            println!();
            let passing = report.passing();
            if report.z_before < eval::Z_THRESHOLD {
                println!("No statistical watermark detected before scrubbing (z = {:.2}); the pipeline scores are informational only.", report.z_before);
            } else if passing.is_empty() {
                println!("Result: no pipeline dropped the watermark below z < {:.1}.", eval::Z_THRESHOLD);
            } else {
                println!("Result: watermark destroyed by: {}", passing.join(", "));
            }
            println!("Note: this is GhostMark's own green/red-list oracle (same statistical family as SynthID-Text), NOT the vendors' secret-key detectors.");
        }
    }
}

fn verdict(z: f64) -> &'static str {
    if z >= eval::Z_THRESHOLD {
        "🔴 WATERMARK"
    } else if z >= 2.0 {
        "🟡 weak signal"
    } else {
        "🟢 clean"
    }
}
