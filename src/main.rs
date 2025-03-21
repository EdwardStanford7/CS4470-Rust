mod ast;
mod lexer;
mod parser;
mod typechecker;
mod utils;
use clap::Parser;
use clap::ValueEnum;
use std::io::{self, Write};
use utils::*;

#[derive(Debug, Clone, ValueEnum, PartialEq)]
enum CompilationMode {
    /// Only run the lexer
    Lex,
    /// Run lexer and parser
    Parse,
    /// Typecheck after parsing
    Typecheck,
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

fn compile() -> Result<(), CompilerError> {
    let args = Args::parse();

    // Open file and read contents
    let file_contents = std::fs::read_to_string(&args.file_name)?;

    // Lex the file - convert any error to CompilerError
    let tokens = lexer::lex(&file_contents)?;

    // Print tokens if in lex mode
    if args.mode == CompilationMode::Lex {
        let stdout = io::stdout();
        let mut buffer = io::BufWriter::new(stdout.lock());

        for token in &tokens {
            writeln!(buffer, "{}", token)?;
        }

        writeln!(buffer, "Compilation succeeded, lexical analysis complete.")?;
        buffer.flush()?;
        return Ok(());
    }

    // Parse the tokens
    let commands = parser::parse(tokens)?;

    // Print AST if in parse mode
    if args.mode == CompilationMode::Parse {
        let stdout = io::stdout();
        let mut buffer = io::BufWriter::new(stdout.lock());

        for command in &commands {
            writeln!(buffer, "{}", command)?;
        }

        writeln!(buffer, "Compilation succeeded, parsing complete.")?;
        buffer.flush()?;
        return Ok(());
    }

    // Typecheck the AST
    let (commands, environment) = typechecker::typecheck(commands)?;

    // Print typechecked AST if in typecheck mode
    if args.mode == CompilationMode::Typecheck {
        let stdout = io::stdout();
        let mut buffer = io::BufWriter::new(stdout.lock());

        for command in &commands {
            writeln!(buffer, "{}", command)?;
        }

        writeln!(buffer, "Compilation succeeded, typechecking complete.")?;
        buffer.flush()?;
        return Ok(());
    }

    Ok(())
}
