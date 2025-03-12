use core::{fmt, str};
use std::fmt::Display;

#[derive(Debug, PartialEq, Clone)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, PartialEq)]
pub struct Token<'a> {
    pub position: Position,
    pub token_type: TokenType,
    pub value: Option<&'a str>,
}

impl Display for Token<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(contents) = self.value {
            write!(f, "{} '{}'", self.token_type, contents)
        } else {
            write!(f, "{}", self.token_type)
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum TokenType {
    // Values
    IntVal,
    FloatVal,
    Variable,
    String,

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
    Op,
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

impl Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenType::IntVal => write!(f, "INTVAL"),
            TokenType::FloatVal => write!(f, "FLOATVAL"),
            TokenType::Variable => write!(f, "VARIABLE"),
            TokenType::String => write!(f, "STRING"),
            TokenType::Op => write!(f, "OP"),
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

pub struct LexerError {
    message: String,
    position: Position,
}

impl Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Lex error: {}:{}: {}",
            self.position.line, self.position.column, self.message
        )
    }
}

/// Represents the state of the lexer and provides helper methods
struct Lexer<'a> {
    program: &'a str,
    bytes: &'a [u8],
    len: usize,
    position: usize,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    /// Creates a new lexer for the given program
    fn new(program: &'a str) -> Self {
        Lexer {
            program,
            bytes: program.as_bytes(),
            len: program.len(),
            position: 0,
            line: 1,
            column: 0,
        }
    }

    /// Check if we've reached the end of input
    fn at_end(&self) -> bool {
        self.position >= self.len
    }

    /// Get the current byte
    fn current_byte(&self) -> u8 {
        if self.position < self.len {
            self.bytes[self.position]
        } else {
            0
        }
    }

    /// Peek at the next byte
    fn peek_byte(&self) -> u8 {
        if self.position + 1 < self.len {
            self.bytes[self.position + 1]
        } else {
            0
        }
    }

    /// Advance position by one character
    fn advance(&mut self) {
        self.position += 1;
        self.column += 1;
    }

    /// Advance the lexer after encountering a newline
    fn advance_newline(&mut self) {
        self.position += 1;
        self.line += 1;
        self.column = 0;
    }

    /// Create an error at the current position
    fn error(&self, message: &str) -> LexerError {
        LexerError {
            message: message.to_string(),
            position: Position {
                line: self.line,
                column: self.column,
            },
        }
    }

    /// Lex an identifier or keyword
    fn lex_identifier(&mut self) -> Token<'a> {
        let start = self.position;

        // Consume all alphanumeric characters
        while self.position < self.len
            && (self.bytes[self.position].is_ascii_alphanumeric()
                || self.bytes[self.position] == b'_')
        {
            self.advance();
        }

        let word = &self.program[start..self.position];

        let pos = Position {
            line: self.line,
            column: self.column,
        };
        // Check if this is a keyword
        match word {
            "array" => Token {
                position: pos,
                token_type: TokenType::Array,
                value: None,
            },
            "assert" => Token {
                position: pos,
                token_type: TokenType::Assert,
                value: None,
            },
            "bool" => Token {
                position: pos,
                token_type: TokenType::Bool,
                value: None,
            },
            "else" => Token {
                position: pos,
                token_type: TokenType::Else,
                value: None,
            },
            "false" => Token {
                position: pos,
                token_type: TokenType::False,
                value: None,
            },
            "float" => Token {
                position: pos,
                token_type: TokenType::Float,
                value: None,
            },
            "fn" => Token {
                position: pos,
                token_type: TokenType::Fn,
                value: None,
            },
            "if" => Token {
                position: pos,
                token_type: TokenType::If,
                value: None,
            },
            "image" => Token {
                position: pos,
                token_type: TokenType::Image,
                value: None,
            },
            "int" => Token {
                position: pos,
                token_type: TokenType::Int,
                value: None,
            },
            "let" => Token {
                position: pos,
                token_type: TokenType::Let,
                value: None,
            },
            "print" => Token {
                position: pos,
                token_type: TokenType::Print,
                value: None,
            },
            "read" => Token {
                position: pos,
                token_type: TokenType::Read,
                value: None,
            },
            "return" => Token {
                position: pos,
                token_type: TokenType::Return,
                value: None,
            },
            "show" => Token {
                position: pos,
                token_type: TokenType::Show,
                value: None,
            },
            "struct" => Token {
                position: pos,
                token_type: TokenType::Struct,
                value: None,
            },
            "sum" => Token {
                position: pos,
                token_type: TokenType::Sum,
                value: None,
            },
            "then" => Token {
                position: pos,
                token_type: TokenType::Then,
                value: None,
            },
            "time" => Token {
                position: pos,
                token_type: TokenType::Time,
                value: None,
            },
            "to" => Token {
                position: pos,
                token_type: TokenType::To,
                value: None,
            },
            "true" => Token {
                position: pos,
                token_type: TokenType::True,
                value: None,
            },
            "void" => Token {
                position: pos,
                token_type: TokenType::Void,
                value: None,
            },
            "write" => Token {
                position: pos,
                token_type: TokenType::Write,
                value: None,
            },
            _ => Token {
                position: pos,
                token_type: TokenType::Variable,
                value: Some(word),
            },
        }
    }

    /// Lex a number (integer or float)
    fn lex_number(&mut self) -> Token<'a> {
        let start = self.position;
        let mut has_dot = false;

        // Consume all numeric characters
        while self.position < self.len {
            if self.bytes[self.position].is_ascii_digit() {
                self.advance();
            } else if self.bytes[self.position] == b'.' && !has_dot {
                has_dot = true;
                self.advance();
            } else {
                break;
            }
        }

        let number = &self.program[start..self.position];
        if has_dot {
            Token {
                position: Position {
                    line: self.line,
                    column: self.column,
                },
                token_type: TokenType::FloatVal,
                value: Some(number),
            }
        } else {
            Token {
                position: Position {
                    line: self.line,
                    column: self.column,
                },
                token_type: TokenType::IntVal,
                value: Some(number),
            }
        }
    }

    /// Lex a dot (could be a float or struct access)
    fn lex_dot(&mut self) -> Token<'a> {
        let start = self.position;
        self.advance(); // Consume the dot

        if self.position < self.len && self.bytes[self.position].is_ascii_digit() {
            // It's a float starting with .
            while self.position < self.len && self.bytes[self.position].is_ascii_digit() {
                self.advance();
            }

            Token {
                position: Position {
                    line: self.line,
                    column: self.column,
                },
                token_type: TokenType::FloatVal,
                value: Some(&self.program[start..self.position]),
            }
        } else {
            // It's just a dot
            Token {
                position: Position {
                    line: self.line,
                    column: self.column - 1,
                },
                token_type: TokenType::Dot,
                value: None,
            }
        }
    }

    /// Lex a string literal
    fn lex_string(&mut self) -> Result<Token<'a>, LexerError> {
        let start = self.position;
        self.advance(); // Skip opening quote

        // Find the closing quote
        while self.position < self.len && self.bytes[self.position] != b'"' {
            if !is_valid(self.bytes[self.position] as char) {
                return Err(self.error("invalid character in string"));
            }
            self.advance();
        }

        if self.position >= self.len {
            return Err(self.error("unterminated string literal"));
        }

        self.advance(); // Skip closing quote

        let string_literal = &self.program[start..self.position];
        Ok(Token {
            position: Position {
                line: self.line,
                column: self.column,
            },
            token_type: TokenType::String,
            value: Some(string_literal),
        })
    }

    /// Lex a comment (line or block)
    fn lex_comment(&mut self) -> Result<(), LexerError> {
        if self.peek_byte() == b'/' {
            // Line comment
            self.position += 2; // Skip //
            self.column += 2;

            while self.position < self.len && self.bytes[self.position] != b'\n' {
                if !is_valid(self.bytes[self.position] as char) {
                    return Err(self.error("invalid character in comment"));
                }
                self.advance();
            }
            Ok(())
        } else {
            // Block comment
            self.position += 2; // Skip /*
            self.column += 2;

            let mut found_end = false;
            while self.position < self.len && !found_end {
                if self.bytes[self.position] == b'*'
                    && self.position + 1 < self.len
                    && self.bytes[self.position + 1] == b'/'
                {
                    self.position += 2; // Skip */
                    self.column += 2;
                    found_end = true;
                } else if self.bytes[self.position] == b'\n' {
                    self.advance_newline();
                } else {
                    if !is_valid(self.bytes[self.position] as char)
                        && self.bytes[self.position] != b'\n'
                    {
                        return Err(self.error("invalid character in block comment"));
                    }
                    self.advance();
                }
            }

            if !found_end {
                return Err(self.error("unterminated block comment"));
            }
            Ok(())
        }
    }

    /// Lex operators and delimiters
    fn lex_operator(&mut self) -> Token<'a> {
        let pos = Position {
            line: self.line,
            column: self.column,
        };

        // Check for two-character operators
        if self.position + 1 < self.len {
            let potential_op = &self.program[self.position..self.position + 2];
            if ["==", "<=", ">=", "!=", "&&", "||"].contains(&potential_op) {
                self.position += 2;
                self.column += 2;
                return Token {
                    position: pos,
                    token_type: TokenType::Op,
                    value: Some(potential_op),
                };
            }
        }

        // Single-character tokens
        let token = match self.bytes[self.position] {
            b'(' => Token {
                position: pos,
                token_type: TokenType::LParen,
                value: None,
            },
            b')' => Token {
                position: pos,
                token_type: TokenType::RParen,
                value: None,
            },
            b'{' => Token {
                position: pos,
                token_type: TokenType::LCurly,
                value: None,
            },
            b'}' => Token {
                position: pos,
                token_type: TokenType::RCurly,
                value: None,
            },
            b'[' => Token {
                position: pos,
                token_type: TokenType::LSquare,
                value: None,
            },
            b']' => Token {
                position: pos,
                token_type: TokenType::RSquare,
                value: None,
            },
            b',' => Token {
                position: pos,
                token_type: TokenType::Comma,
                value: None,
            },
            b'=' => Token {
                position: pos,
                token_type: TokenType::Equals,
                value: None,
            },
            b':' => Token {
                position: pos,
                token_type: TokenType::Colon,
                value: None,
            },
            _ => Token {
                position: pos,
                token_type: TokenType::Op,
                value: Some(&self.program[self.position..self.position + 1]),
            },
        };

        self.advance();
        token
    }

    /// Run the lexer to produce tokens
    fn lex(&mut self) -> Result<Vec<Token<'a>>, LexerError> {
        let mut tokens: Vec<Token<'a>> = Vec::with_capacity(self.program.len() / 4);

        while !self.at_end() {
            let byte = self.current_byte();

            match byte {
                // Alphabetic characters (keywords or variables)
                b'a'..=b'z' | b'A'..=b'Z' => {
                    tokens.push(self.lex_identifier());
                }

                // Number literals
                b'0'..=b'9' => {
                    tokens.push(self.lex_number());
                }

                // Dot (could be a float or struct access)
                b'.' => {
                    tokens.push(self.lex_dot());
                }

                // String literals
                b'"' => {
                    tokens.push(self.lex_string()?);
                }

                // Comments or division
                b'/' => {
                    let next = self.peek_byte();
                    if next == b'/' || next == b'*' {
                        self.lex_comment()?;
                    } else {
                        tokens.push(Token {
                            position: Position {
                                line: self.line,
                                column: self.column,
                            },
                            token_type: TokenType::Op,
                            value: Some("/"),
                        });
                        self.advance();
                    }
                }

                // Line continuation
                b'\\' => {
                    if self.position + 1 < self.len && self.peek_byte() == b'\n' {
                        self.position += 2;
                        self.line += 1;
                        self.column = 0;
                    } else {
                        return Err(self.error("invalid line continuation"));
                    }
                }

                // Newline
                b'\n' => {
                    // Add newline token if the last token is not a newline
                    if tokens.is_empty()
                        || !matches!(tokens.last().unwrap().token_type, TokenType::Newline)
                    {
                        tokens.push(Token {
                            position: Position {
                                line: self.line,
                                column: self.column,
                            },
                            token_type: TokenType::Newline,
                            value: None,
                        });
                    }

                    self.advance_newline();
                }

                // Whitespace
                b' ' | b'\t' | b'\r' => {
                    self.advance();
                }

                // Operators and delimiters
                _ if is_valid(byte as char) => {
                    tokens.push(self.lex_operator());
                }

                // Invalid character
                _ => {
                    return Err(self.error("invalid character"));
                }
            }
        }

        // Add EOF token
        tokens.push(Token {
            position: Position {
                line: self.line,
                column: self.column,
            },
            token_type: TokenType::EndOfFile,
            value: None,
        });

        Ok(tokens)
    }
}

// Helper function to check if a character is valid
fn is_valid(c: char) -> bool {
    (c as u32) >= 32 && (c as u32) <= 126
}

// Optimized lexer implementation - main entry point
pub fn lex<'a>(program: &'a str) -> Result<Vec<Token<'a>>, LexerError> {
    let mut lexer = Lexer::new(program);
    lexer.lex()
}
