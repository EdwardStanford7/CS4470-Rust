mod ast;
mod lexer;
mod parser;
mod typechecker;

use clap::Parser;
use clap::ValueEnum;
use std::io::{self, Write};

#[derive(Debug, Clone, ValueEnum, PartialEq)]
enum CompilationMode {
    /// Only run the lexer
    Lex,
    /// Run lexer and parser
    Parse,
    /// Typecheck after parsing
    Typecheck,
    /// Generate C intermediate representation
    IntermediateRepresentation,
    /// Generate assembly code
    Assembly,
    /// Full compilation pipeline
    Full,
}

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// File name to compile
    #[arg(required(true))]
    file_name: String,

    /// Compilation mode (lex, parse, typecheck, IR, Assembly, full)
    #[arg(short, long, default_value = "f")]
    mode: CompilationMode,
}

fn main() {
    let args = Args::parse();

    // Open file and read contents
    let file_contents = std::fs::read_to_string(&args.file_name);
    if let Err(e) = file_contents {
        println!("Compilation failed: error reading file: {}", e);
        std::process::exit(1);
    }
    let file_contents = file_contents.unwrap();

    // Lex the file
    let tokens = match lexer::lex(&file_contents) {
        Ok(tokens) => tokens,
        Err(e) => {
            println!("Compilation failed: {}", e);
            std::process::exit(1);
        }
    };

    // Print tokens if in lex mode
    if args.mode == CompilationMode::Lex {
        let stdout = io::stdout();
        let mut buffer = io::BufWriter::new(stdout.lock());

        for token in &tokens {
            writeln!(buffer, "{}", token).unwrap();
        }

        writeln!(buffer, "Compilation succeeded, lexical analysis complete.").unwrap();
        buffer.flush().unwrap();
        std::process::exit(0);
    }

    // Parse the tokens
    let commands = match parser::parse(tokens) {
        Ok(ast) => ast,
        Err(e) => {
            println!("Compilation failed: {}", e);
            std::process::exit(1);
        }
    };

    // Print AST if in parse mode
    if args.mode == CompilationMode::Parse {
        let stdout = io::stdout();
        let mut buffer = io::BufWriter::new(stdout.lock());

        for command in &commands {
            writeln!(buffer, "{}", command).unwrap();
        }

        writeln!(buffer, "Compilation succeeded, parsing complete.").unwrap();
        buffer.flush().unwrap();
        std::process::exit(0);
    }

    // Typecheck the AST
    let global_env = match typechecker::typecheck(&commands) {
        Ok(global_env) => global_env,
        Err(e) => {
            println!("Compilation failed: {}", e);
            std::process::exit(1);
        }
    };

    // Print typechecked AST if in typecheck mode
    if args.mode == CompilationMode::Typecheck {
        let stdout = io::stdout();
        let mut buffer = io::BufWriter::new(stdout.lock());

        for command in &commands {
            writeln!(buffer, "{}", command).unwrap();
        }

        writeln!(buffer, "Compilation succeeded, typechecking complete.").unwrap();
        buffer.flush().unwrap();
        std::process::exit(0);
    }
}
