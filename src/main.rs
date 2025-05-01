mod assembly;
mod ast;
mod lexer;
mod parser;
mod typechecker;
mod utils;
mod herbie_optimizer;
use clap::{Parser, ArgAction};
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
    #[arg(short = 'O', long = "optimization", default_value = "0", conflicts_with_all = ["lex", "herbie"])]
    optimization_level: u8,

    /// Enable Herbie optimization
    #[arg(short = 'H', long = "herbie", action = ArgAction::SetTrue, default_value_t = false, conflicts_with_all = ["optimization_level"])]
    herbie: bool,
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
    println!("Args: {:?}", args);

    let file_contents = std::fs::read_to_string(&args.file_name)?;
    println!("Read {} bytes from `{}`", file_contents.len(), args.file_name);

    let tokens = lexer::lex(&file_contents)?;
    let commands = parser::parse(tokens)?;
    let (commands, env) = typechecker::typecheck(commands, args.optimization_level)?;
    println!("Typecheck complete, {} commands, herbie={}", commands.len(), args.herbie);

    let mut optimized = Vec::with_capacity(commands.len());
    for (i, cmd) in commands.into_iter().enumerate() {
        println!("Command[{}] before: {}", i, cmd);
        let mut cmd = cmd;
        if args.herbie {
            println!("  → applying herbie_optimizer");
            herbie_optimizer::apply_herbie_optimization(&mut cmd);
            println!("  → after herbie: {}", cmd);
        }
        optimized.push(cmd);
    }
    // After your optimization loop, add:

println!("--- DEBUG: optimized_commands after applying Herbie ---");
for (i, cmd) in optimized.iter().enumerate() {
    println!("  cmd[{}]: {}", i, cmd);
}

// Then immediately before your typecheck‐mode print return:
if args.typecheck {
    println!("--- DEBUG: entering typecheck print branch ---");
    let stdout = io::stdout();
    let mut buffer = io::BufWriter::new(stdout.lock());

    for command in &optimized {
        writeln!(buffer, "{}", command)?;
    }

    writeln!(buffer, "Compilation succeeded, typechecking complete.")?;
    buffer.flush()?;
    return Ok(());
}
    
    let args = Args::parse();

    // Open file and read contents
    let file_contents = std::fs::read_to_string(&args.file_name)?;

    // Lex the file - convert any error to CompilerError
    let tokens = lexer::lex(&file_contents)?;

    // Print tokens if in lex mode
    if args.lex {
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
    if args.parse {
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
    let (commands, env) = typechecker::typecheck(commands, args.optimization_level)?;

    // Create new vector for optimized commands
    let mut optimized_commands = Vec::with_capacity(commands.len());
    for command in commands {
        let mut opt_command = command;
        if args.herbie {
            herbie_optimizer::apply_herbie_optimization(&mut opt_command);
        }
        optimized_commands.push(opt_command);
    }

    // Print typechecked AST if in typecheck mode
    if args.typecheck {
        let stdout = io::stdout();
        let mut buffer = io::BufWriter::new(stdout.lock());

        for command in &optimized_commands {
            writeln!(buffer, "{}", command)?;
        }

        writeln!(buffer, "Compilation succeeded, typechecking complete.")?;
        buffer.flush()?;
        return Ok(());
    }

    // Generate assembly code
    let assembly = assembly::generate_assembly(optimized_commands, args.optimization_level, env);

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
