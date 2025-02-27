use std::{
    collections::HashMap, ffi::OsString, fmt::Display, fs::{read_to_string, File}, io::{BufReader, Write}, ops::{Add, Div, Mul, Sub}, path::PathBuf, sync::Mutex
};
use std::env;
use anyhow::*;
mod bytecode;
mod value;
mod bytecode_machine;
mod tokenizer;
mod errors;
mod token_to_ast;
mod compiler;
mod typedata;
#[cfg(test)]
mod test;

use bytecode::*;

use value::*;
use tokenizer::*;
use token_to_ast::*;
use compiler::*;
use bytecode_machine::*;
use typedata::*;

use clap::{arg, crate_version, value_parser, Arg, ArgAction, Command};
use std::result::Result::Ok;
use std::result::Result::Err;


fn cli() -> Command {
    Command::new("abra")
        .about("Abra compiler and runner")
        .subcommand_required(true)
        .author("Przemyslaw Wrona")
        .version(crate_version!())
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .arg(Arg::new("debug").short('D').long("debug").action(ArgAction::Set).value_parser(value_parser!(u16).range(0..=3)).default_value("0").help("Sets debug level"))
        .subcommand(
            Command::new("compile")
                .about("Compiles code into bytecode format")
                .short_flag('c')
                .arg(Arg::new("pretty-bytecode").short('p').long("pretty").action(ArgAction::SetTrue).help("Makes the compier spit out json in human readable format"))
                .arg(arg!(<IN> "file to compile"))
                .arg(arg!( -o --output <OUT> "output").default_value("o.abx"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("run")
                .short_flag('r')
                .about("Runs the program")
                .arg(arg!(<FILE> "file to run"))
                .arg_required_else_help(true),
        ).subcommand(
            Command::new("go")
                .short_flag('g')
                .about("Compiles and runs the program")
                .arg(arg!(<FILE> "file to run"))
                .arg_required_else_help(true))
}


fn main() {
    let matches = cli().get_matches();
    let debug: u16 = *matches.get_one::<u16>("debug").unwrap();
    println!("debug lvl : {}",debug);
    match matches.subcommand() {
        Some(("go",submatches)) => {
            let file = submatches.get_one::<String>("FILE").unwrap();
            let open_file = File::open(file).unwrap();
            let rdr = BufReader::new(open_file);
            let code : Code = bincode::deserialize_from(rdr).unwrap();
            if debug > 0{
                println!("AbraASM \n{}",code.string_representation());
            }
            ByteCodeMachine::new(code,debug > 1).run();
        }
        Some(("compile",submatches)) => {
            let infile = submatches.get_one::<String>("IN").unwrap();
            let outfile = submatches.get_one::<String>("output").unwrap();
            let pretty = submatches.get_flag("pretty-bytecode");
            
            compile(infile, outfile,debug,pretty);
        }
        
        Some(("run",submatches)) => {
            let file = submatches.get_one::<String>("FILE").unwrap();
            let x = match tokenize(read_to_string(file).unwrap()){
                Ok(z) => z,
                Err(err) => {
                    println!("Tokenization Error!\n{:?}",err);
                    return
                }
            };
            if debug > 2 {
                for token in x.iter().enumerate() {
                    println!("{} - {}", token.0, token.1.token());
                }
            }
            let mut parser: Parser = Parser::new(x);

            let parsed = match parser.parse() {
                Ok(z) => z,
                Err(err) => {
                    for cause in err.chain() {
                        println!("{:?}",cause);
                    }
                    println!("Parsing Error!\n{:?}",err);
                    return
                }
            };
            let mut compiler : Compiler = Compiler::new();
            compiler.compile_from_ast(parsed);
            if debug > 0{
                println!("AbraASM \n{}",compiler.string_representation());
            }
            let code: Code = compiler.into();
            ByteCodeMachine::new(code,debug > 1).run();
        }
        _ => unreachable!(),
    }
}


fn compile(infile: &String, outfile: &String,debug:u16,pretty:bool) {
    let x = match tokenize(read_to_string(infile).unwrap()){
        Ok(z) => z,
        Err(err) => {
            println!("Tokenization Error!\n{:?}",err);
            return
        }
    };
    if debug > 2 {
        for token in x.iter().enumerate() {
            println!("{} - {}", token.0, token.1.token());
        }
    }
    let mut parser: Parser = Parser::new(x);

    let parsed = match parser.parse() {
        Ok(z) => z,
        Err(err) => {
            println!("Parsing Error!\n{:?}",err);
            return
        }
    };
    let mut compiler : Compiler = Compiler::new();
    compiler.compile_from_ast(parsed);
    if debug > 0{
        println!("AbraASM \n{}",compiler.string_representation());
    }
    let code : Code = compiler.into();
    let mut file = File::create(outfile).unwrap();
    let serialized_data = match pretty {
        false => bincode::serialize(&code).unwrap(),
        true => bincode::serialize(&code).unwrap(),
    } ;
    file.write_all(serialized_data.as_slice()).unwrap();
}