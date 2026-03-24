
use abra_lang::cli;
fn main() {
    if let Err(e) = cli::run_app() {
        eprintln!("Error: {}", e);
       
        std::process::exit(1);
    }
}
