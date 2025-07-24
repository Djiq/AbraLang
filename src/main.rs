//! Main application entry point.
use abra_lang::cli;
fn main() {
    if let Err(e) = cli::run_app() {
        eprintln!("Error: {}", e);
        // Consider adding more context to the error if possible
        std::process::exit(1);
    }
}
