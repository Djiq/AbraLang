//! Command-line interface handling.

use crate::compiler::Code; // Only Code is needed here from compiler
use crate::runtime::vm::ByteCodeMachine; // Only ByteCodeMachine is needed here
use anyhow::Result;
use clap::{arg, command, value_parser, Arg, Command}; // Removed ArgAction
use std::{
    fs::{read_to_string, File},
    io::Write,
}; // Removed Path // Make sure anyhow is a dependency

// --- CLI Definition ---

/// Builds the command-line interface configuration using clap.
fn build_cli() -> Command {
    command!()
        .subcommand_required(true)
        .arg(
            Arg::new("debug")
                .short('d')
                .long("debug")
                .value_parser(value_parser!(u16))
                .default_value("0"),
        )
        .subcommand(
            Command::new("run")
                .short_flag('r')
                .about("Compiles and runs file")
                .arg(arg!([IN] "file to compile and run").value_parser(value_parser!(String))),
        )
        .subcommand(
            Command::new("compile")
                .short_flag('c')
                .about("Compiles file")
                .arg(arg!([IN] "file to compile").value_parser(value_parser!(String)))
                .arg(arg!([OUT] "output path").value_parser(value_parser!(String))),
        )
        .subcommand(
            Command::new("execute")
                .short_flag('x')
                .about("Runs compiled file")
                .arg(arg!([FILE] "file to run").value_parser(value_parser!(String))),
        )
}

// --- Public Execution Function ---

/// Runs the main application logic based on parsed arguments.
pub fn run_app() -> Result<()> {
    let matches = build_cli().get_matches();
    let debug: u16 = *matches.get_one::<u16>("debug").unwrap_or(&0); // Get debug level safely
    println!("Debug level: {}", debug);

    match matches.subcommand() {
        Some(("run", submatches)) => {
            let infile_path = submatches
                .get_one::<String>("IN")
                .ok_or_else(|| anyhow::anyhow!("Missing input file for 'go' command"))?;
            println!("Compiling '{}'...", infile_path);
            let compiled_code = compile(infile_path, debug)?;
            println!("Running...");
            let exit_code = run(&compiled_code, debug)?;
            println!("Program exited with code: {}", exit_code);
        }
        Some(("compile", submatches)) => {
            let out_file = submatches
                .get_one::<String>("OUT")
                .ok_or_else(|| anyhow::anyhow!("Missing output file for 'compile' command"))?;
            let in_file = submatches
                .get_one::<String>("IN")
                .ok_or_else(|| anyhow::anyhow!("Missing input file for 'compile' command"))?;

            println!("Compiling '{}' to '{}'...", in_file, out_file);
            let compiled_code = compile(in_file, debug)?;

            let mut file = File::create(out_file).map_err(|e| {
                anyhow::anyhow!("Failed to create output file '{}': {}", out_file, e)
            })?;
            let serialized = bincode::serialize(&compiled_code)
                .map_err(|e| anyhow::anyhow!("Failed to serialize bytecode: {}", e))?;
            file.write_all(&serialized).map_err(|e| {
                anyhow::anyhow!("Failed to write bytecode to file '{}': {}", out_file, e)
            })?;
            println!("Compilation successful.");
        }
        Some(("execute", submatches)) => {
            let in_file = submatches
                .get_one::<String>("FILE")
                .ok_or_else(|| anyhow::anyhow!("Missing input file for 'run' command"))?;

            println!("Loading bytecode from '{}'...", in_file);
            let file = File::open(in_file).map_err(|e| {
                anyhow::anyhow!("Failed to open bytecode file '{}': {}", in_file, e)
            })?;
            let compiled_code: Code = bincode::deserialize_from(file).map_err(|e| {
                anyhow::anyhow!("Failed to deserialize bytecode from '{}': {}", in_file, e)
            })?;

            println!("Running...");
            let exit_code = run(&compiled_code, debug)?;
            println!("Program exited with code: {}", exit_code);
        }
        _ => unreachable!("Subcommand is required"),
    }
    Ok(())
}

// --- Compile and Run Helpers (Moved from original cli.rs/main.rs) ---

/// Compiles the source file, potentially optimizes, and returns the Code.
pub fn compile(infile_path: &str, debug: u16) -> Result<Code> {
    // Use paths relative to the new module structure
    use crate::compiler::Compiler;
    use crate::frontend::{parser::Parser, tokenizer::Tokenizer};

    let source_code = read_to_string(infile_path)
        .map_err(|e| anyhow::anyhow!("Failed to read input file '{}': {}", infile_path, e))?;

    // 1. Tokenize
    let mut tokenizer = Tokenizer::new(&source_code);
    if debug & 1 == 1 {
        // Tokenizer debug flag
        let tokens: Vec<_> = tokenizer.collect(); // Collect for printing
        println!("--- Tokens ---");
        for token_res in tokens {
            match token_res {
                Ok((start, tok, end)) => println!("[{}..{}] {:?}", start, end, tok),
                Err(e) => eprintln!("Tokenizer Error: {}", e),
            }
        }
        println!("--------------");
        // Re-create tokenizer as it was consumed by the debug print
        tokenizer = Tokenizer::new(&source_code);
    }

    // 2. Parse
    let mut parser = Parser::new(tokenizer);
    let ast_result = parser.parse_program();
    // Check for error before unwrapping
    if let Err(e) = &ast_result {
        // Use eprintln for errors, provide context
        eprintln!("Parser Error: {}", e);
        // Consider adding more specific error context if possible from 'e'
        return Err(anyhow::anyhow!("Parsing failed for '{}'", infile_path).context(e.to_string()));
        // Propagate error with context
    }
    let ast = ast_result.unwrap(); // Safe to unwrap now

    // 3. Optimize AST (Optional)

    // 4. Compile
    let mut compiler = Compiler::new();
    compiler.compilation_pipepline(ast)?; // Compile the potentially optimized AST
    let code: Code = compiler.into();

    // 5. Optimize Bytecode (Optional)

    Ok(code)
}

/// Runs the compiled bytecode using the virtual machine.
pub fn run(code: &Code, debug: u16) -> Result<usize> {
    // Debug level > 1 enables VM debug mode
    let vm_debug_mode = debug > 1;
    // Pass necessary context like VTables if they become separate
    let mut machine =
        ByteCodeMachine::new(code.clone(), vm_debug_mode /*, pass vtables here */);
    let exit_code = machine.run();
    Ok(exit_code)
}
