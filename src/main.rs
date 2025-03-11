mod lexer;

use clap::Parser;
use clap::ValueEnum;

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
    let tokens = match lexer::lex(&file_contents, &args.file_name) {
        Ok(tokens) => tokens,
        Err(e) => {
            println!("Compilation failed: {}", e);
            std::process::exit(1);
        }
    };

    // Print tokens if in lex mode
    if args.mode == CompilationMode::Lex {
        for token in tokens {
            println!("{}", token);
        }
        println!("Compilation succeeded, lexical analysis complete.");
        std::process::exit(0);
    }

    // Parse the tokens
}
