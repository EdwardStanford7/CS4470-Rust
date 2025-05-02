use crate::ast::Type;
use std::{
    collections::HashMap,
    fmt::{self, Display},
};

// --------------------------------------------------------------------------------------- Universal utils ---------------------------------------------------------------------------------------------

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Position {
    line: usize,
    column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Position {
        Position { line, column }
    }
}

// ---------------------------------------------------------------------------------------- Lexer utils ---------------------------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
pub struct Token<'a> {
    pub position: Position,
    pub token_type: TokenType<'a>,
}

impl Display for Token<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.token_type)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum TokenType<'a> {
    // Values
    IntVal(&'a str),
    FloatVal(&'a str),
    Variable(&'a str),
    String(&'a str),

    // Keywords
    Array,
    Assert,
    Bool,
    Else,
    Fn,
    If,
    Image,
    Int,
    Float,
    Let,
    Print,
    Read,
    Return,
    Show,
    Struct,
    Sum,
    Then,
    Time,
    To,
    Void,
    Write,

    // Literals
    True,
    False,

    // Operators
    Op(&'a str),
    Equals,

    // Delimiters
    LParen,
    RParen,
    LCurly,
    RCurly,
    LSquare,
    RSquare,
    Comma,
    Colon,
    Dot,

    // Other
    Newline,
    EndOfFile,
}

impl Display for TokenType<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenType::IntVal(contents) => write!(f, "INTVAL '{}'", contents),
            TokenType::FloatVal(contents) => write!(f, "FLOATVAL '{}'", contents),
            TokenType::Variable(contents) => write!(f, "VARIABLE '{}'", contents),
            TokenType::String(contents) => write!(f, "STRING '\"{}\"'", contents),
            TokenType::Op(contents) => write!(f, "OP '{}'", contents),
            TokenType::Array => write!(f, "ARRAY 'array'"),
            TokenType::Assert => write!(f, "ASSERT 'assert'"),
            TokenType::Bool => write!(f, "BOOL 'bool'"),
            TokenType::Else => write!(f, "ELSE 'else'"),
            TokenType::Fn => write!(f, "FN 'fn'"),
            TokenType::If => write!(f, "IF 'if'"),
            TokenType::Image => write!(f, "IMAGE 'image'"),
            TokenType::Int => write!(f, "INT 'int'"),
            TokenType::Float => write!(f, "FLOAT 'float'"),
            TokenType::Let => write!(f, "LET 'let'"),
            TokenType::Print => write!(f, "PRINT 'print'"),
            TokenType::Read => write!(f, "READ 'read'"),
            TokenType::Return => write!(f, "RETURN 'return'"),
            TokenType::Show => write!(f, "SHOW 'show'"),
            TokenType::Struct => write!(f, "STRUCT 'struct'"),
            TokenType::Sum => write!(f, "SUM 'sum'"),
            TokenType::Then => write!(f, "THEN 'then'"),
            TokenType::Time => write!(f, "TIME 'time'"),
            TokenType::To => write!(f, "TO 'to'"),
            TokenType::Void => write!(f, "VOID 'void'"),
            TokenType::Write => write!(f, "WRITE 'write'"),
            TokenType::True => write!(f, "TRUE 'true'"),
            TokenType::False => write!(f, "FALSE 'false'"),
            TokenType::Equals => write!(f, "EQUALS '='"),
            TokenType::LParen => write!(f, "LPAREN '('"),
            TokenType::RParen => write!(f, "RPAREN ')'"),
            TokenType::LCurly => write!(f, "LCURLY '{{'"),
            TokenType::RCurly => write!(f, "RCURLY '}}'"),
            TokenType::LSquare => write!(f, "LSQUARE '['"),
            TokenType::RSquare => write!(f, "RSQUARE ']'"),
            TokenType::Comma => write!(f, "COMMA ','"),
            TokenType::Colon => write!(f, "COLON ':'"),
            TokenType::Dot => write!(f, "DOT '.'"),
            TokenType::Newline => write!(f, "NEWLINE"),
            TokenType::EndOfFile => write!(f, "END_OF_FILE"),
        }
    }
}

pub struct LexError {
    message: String,
    position: Position,
}

impl LexError {
    pub fn new(message: String, position: Position) -> LexError {
        LexError { message, position }
    }
}

impl Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Lex error: {}:{}: {}",
            self.position.line, self.position.column, self.message
        )
    }
}

// ---------------------------------------------------------------------------------------- Parser utils ---------------------------------------------------------------------------------------------

pub struct ParseError {
    message: String,
    position: Position,
}

impl ParseError {
    pub fn new(message: String, position: Position) -> ParseError {
        ParseError { message, position }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Parse error: {}:{}: {}",
            self.position.line, self.position.column, self.message
        )
    }
}

// ---------------------------------------------------------------------------------------- TypeChecker utils ---------------------------------------------------------------------------------------------

pub struct TypeError {
    message: String,
    position: Position,
}

impl TypeError {
    pub fn new(message: String, position: Position) -> TypeError {
        TypeError { message, position }
    }
}

impl Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Type error: {}:{}: {}",
            self.position.line, self.position.column, self.message
        )
    }
}

pub struct TypeEnvironment<'a> {
    // Map of scopes to (parent scope, local identifiers)
    environments: HashMap<&'a str, (&'a str, HashMap<&'a str, Type<'a>>)>,
}

impl<'a> TypeEnvironment<'a> {
    pub fn new() -> TypeEnvironment<'a> {
        TypeEnvironment {
            environments: HashMap::new(),
        }
    }

    pub fn add_scope(&mut self, scope: &'a str, parent: &'a str) {
        self.environments.insert(scope, (parent, HashMap::new()));
    }

    pub fn remove_scope(&mut self, scope: &'a str) {
        self.environments.remove(scope);
    }

    pub fn add_identifier(
        &mut self,
        scope: &'a str,
        name: &'a str,
        typ: Type<'a>,
        position: Position,
    ) -> Result<(), TypeError> {
        // Iteratively check all parent scopes and check if the name is already defined
        let mut current_scope = scope;
        while let Some((parent, identifiers)) = self.environments.get(current_scope) {
            if identifiers.contains_key(name) {
                return Err(TypeError {
                    message: format!("Identifier {} is already defined", name),
                    position,
                });
            }

            current_scope = parent;
        }

        // Add the identifier to the current scope
        if let Some((_, identifiers)) = self.environments.get_mut(scope) {
            identifiers.insert(name, typ);
            Ok(())
        } else {
            Err(TypeError {
                message: format!("Scope {} is not defined", scope),
                position,
            })
        }
    }

    pub fn get_identifier(
        &self,
        scope: &'a str,
        position: Position,
        name: &'a str,
    ) -> Result<Type<'a>, TypeError> {
        // Iteratively check all parent scopes and check if the name is already defined
        let mut current_scope = scope;
        while let Some((parent, identifiers)) = self.environments.get(current_scope) {
            if let Some(typ) = identifiers.get(name) {
                return Ok(typ.clone());
            }

            current_scope = parent;
        }

        Err(TypeError {
            message: format!("Identifier {} is not defined", name),
            position,
        })
    }
}

// ---------------------------------------------------------------------------------------- AsmGen utils ---------------------------------------------------------------------------------------------
