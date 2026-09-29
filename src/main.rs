mod assembly;
mod ast;
mod lexer;
mod parser;
mod typechecker;
mod utils;
use clap::Parser;
use std::fmt::Write as _;
use std::io::{self, Write};
use utils::*;

// Stupid autograder requires very specific flags
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// File name to compile
    #[arg(required(true))]
    file_name: String,

    /// Lex mode
    #[arg(short = 'l', long = "lex", default_value = "false", conflicts_with_all = ["parse", "typecheck", "assembly"])]
    lex: bool,

    /// Parse mode
    #[arg(short = 'p', long = "parse", default_value = "false", conflicts_with_all = ["lex", "typecheck", "assembly"])]
    parse: bool,

    /// Typecheck mode
    #[arg(short = 't', long = "typecheck", default_value = "false", conflicts_with_all = ["lex", "parse", "assembly"])]
    typecheck: bool,

    /// Assembly mode
    #[arg(short = 's', long = "assembly", default_value = "false", conflicts_with_all = ["lex", "parse", "typecheck"])]
    assembly: bool,

    /// Optimization level
    #[arg(short = 'O', long = "optimization", default_value = "0", conflicts_with_all = ["lex", "parse", "typecheck"])]
    optimization_level: u8,
}

enum CompilerError {
    Io(io::Error),
    Lex(LexError),
    Parse(ParseError),
    Type(TypeError),
}

// Implement Display trait for CompilerError
impl std::fmt::Display for CompilerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompilerError::Io(err) => write!(f, "Compilation failed: {}", err),
            CompilerError::Lex(err) => write!(f, "Compilation failed: {}", err),
            CompilerError::Parse(err) => write!(f, "Compilation failed: {}", err),
            CompilerError::Type(err) => write!(f, "Compilation failed: {}", err),
        }
    }
}

// Automatic conversion from std::io::Error
impl From<io::Error> for CompilerError {
    fn from(err: io::Error) -> Self {
        CompilerError::Io(err)
    }
}

// Automatic conversion from LexerError
impl From<LexError> for CompilerError {
    fn from(err: LexError) -> Self {
        CompilerError::Lex(err)
    }
}

// Automatic conversion from ParseError
impl From<ParseError> for CompilerError {
    fn from(err: ParseError) -> Self {
        CompilerError::Parse(err)
    }
}

// Automatic conversion from TypeError
impl From<TypeError> for CompilerError {
    fn from(err: TypeError) -> Self {
        CompilerError::Type(err)
    }
}

fn main() {
    if let Err(err) = compile() {
        println!("{}", err);
    }
}

/// Render compiler diagnostics in memory, then write them to stdout once.
fn print_diagnostics<T: std::fmt::Display>(
    items: &[T],
    success_message: &str,
) -> Result<(), CompilerError> {
    let mut output = String::new();
    for item in items {
        writeln!(output, "{}", item).expect("writing to a String is infallible");
    }
    writeln!(output, "{}", success_message).expect("writing to a String is infallible");

    io::stdout().lock().write_all(output.as_bytes())?;
    Ok(())
}

fn compile() -> Result<(), CompilerError> {
    let args = Args::parse();

    // Open file and read contents
    let file_contents = std::fs::read_to_string(&args.file_name)?;

    // Lex the file - convert any error to CompilerError
    let tokens = lexer::lex(&file_contents)?;

    // Print tokens if in lex mode
    if args.lex {
        print_diagnostics(&tokens, "Compilation succeeded, lexical analysis complete.")?;
        return Ok(());
    }

    // Parse the tokens
    let commands = parser::parse(tokens)?;

    // Print AST if in parse mode
    if args.parse {
        print_diagnostics(&commands, "Compilation succeeded, parsing complete.")?;
        return Ok(());
    }

    // Typecheck the AST
    let commands = typechecker::typecheck(commands)?;

    // Print typechecked AST if in typecheck mode
    if args.typecheck {
        print_diagnostics(&commands, "Compilation succeeded, typechecking complete.")?;
        return Ok(());
    }

    // Generate assembly code
    let assembly = assembly::generate_assembly(commands, args.optimization_level);

    // Print assembly code if in assembly mode
    if args.assembly {
        println!(
            "{}\nCompilation succeeded, assembly generation complete.",
            assembly
        );
        return Ok(());
    }

    Ok(())
}
