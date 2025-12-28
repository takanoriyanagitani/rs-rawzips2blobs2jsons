use clap::Parser;
use rs_rawzips2blobs2jsons::stdin2zfilenames2zip2blobs2jsons2stdout;
use std::process;

const MAX_ZIP_BYTES_DEFAULT: u64 = 1 << 20; // 1MiB
const MAX_ITEM_BYTES_DEFAULT: u64 = 1 << 17; // 128KiB

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Converts zip archives into a stream of JSON blobs.",
    long_about = "Reads zip filenames from stdin (one per line), and for each file inside the zips, outputs a JSON blob. The blob contains metadata and base64-encoded content."
)]
struct Cli {
    #[arg(
        long,
        default_value_t = MAX_ZIP_BYTES_DEFAULT,
        help = "Max size in bytes for each zip file (skipped if exceeded)."
    )]
    zip_size_max: u64,

    #[arg(
        long,
        default_value_t = MAX_ITEM_BYTES_DEFAULT,
        help = "Max size in bytes for a file within a zip (skipped if exceeded)."
    )]
    item_size_max: u64,

    #[arg(
        long,
        default_value = "application/octet-stream",
        help = "Default Content-Type for zip entries."
    )]
    item_content_type: String,

    #[arg(
        long,
        default_value = "identity",
        help = "Default Content-Encoding for zip entries."
    )]
    item_content_encoding: String,

    #[arg(
        short,
        long,
        default_value_t = false,
        help = "Enable verbose output (warnings for skipped files)."
    )]
    verbose: bool,
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = stdin2zfilenames2zip2blobs2jsons2stdout(
        cli.zip_size_max,
        &cli.item_content_type,
        &cli.item_content_encoding,
        cli.item_size_max,
        cli.verbose,
    ) {
        eprintln!("Error: Failed to process zip files from stdin: {}", e);
        process::exit(1);
    }
}
