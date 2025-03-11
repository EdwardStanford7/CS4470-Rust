use core::fmt;
use std::fmt::Display;

#[derive(Debug, PartialEq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, PartialEq)]
pub enum Token<'a> {
    // Values
    IntVal { position: Position, value: &'a str },
    FloatVal { position: Position, value: &'a str },
    Variable { position: Position, value: &'a str },
    StringLiteral { position: Position, value: &'a str },

    // Keywords
    Array { position: Position },
    Assert { position: Position },
    Bool { position: Position },
    Else { position: Position },
    Fn { position: Position },
    If { position: Position },
    Image { position: Position },
    Int { position: Position },
    Float { position: Position },
    Let { position: Position },
    Print { position: Position },
    Read { position: Position },
    Return { position: Position },
    Show { position: Position },
    Struct { position: Position },
    Sum { position: Position },
    Then { position: Position },
    Time { position: Position },
    To { position: Position },
    Void { position: Position },
    Write { position: Position },

    // Literals
    True { position: Position },
    False { position: Position },

    // Operators
    Op { position: Position, value: &'a str },
    Equals { position: Position },

    // Delimiters
    LParen { position: Position },
    RParen { position: Position },
    LCurly { position: Position },
    RCurly { position: Position },
    LSquare { position: Position },
    RSquare { position: Position },
    Comma { position: Position },
    Colon { position: Position },
    Dot { position: Position },

    // Other
    Newline { position: Position },
    EndOfFile { position: Position },
}

impl<'a> Display for Token<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::IntVal { position: _, value } => write!(f, "INTVAL '{}'", value),
            Token::FloatVal { position: _, value } => write!(f, "FLOATVAL '{}'", value),
            Token::Variable { position: _, value } => write!(f, "VARIABLE '{}'", value),
            Token::StringLiteral { position: _, value } => write!(f, "STRING '{}'", value),
            Token::Op { position: _, value } => write!(f, "OP '{}'", value),
            Token::Array { position: _ } => write!(f, "ARRAY 'array'"),
            Token::Assert { position: _ } => write!(f, "ASSERT 'assert'"),
            Token::Bool { position: _ } => write!(f, "BOOL 'bool'"),
            Token::Else { position: _ } => write!(f, "ELSE 'else'"),
            Token::Fn { position: _ } => write!(f, "FN 'fn'"),
            Token::If { position: _ } => write!(f, "IF 'if'"),
            Token::Image { position: _ } => write!(f, "IMAGE 'image'"),
            Token::Int { position: _ } => write!(f, "INT 'int'"),
            Token::Float { position: _ } => write!(f, "FLOAT 'float'"),
            Token::Let { position: _ } => write!(f, "LET 'let'"),
            Token::Print { position: _ } => write!(f, "PRINT 'print'"),
            Token::Read { position: _ } => write!(f, "READ 'read'"),
            Token::Return { position: _ } => write!(f, "RETURN 'return'"),
            Token::Show { position: _ } => write!(f, "SHOW 'show'"),
            Token::Struct { position: _ } => write!(f, "STRUCT 'struct'"),
            Token::Sum { position: _ } => write!(f, "SUM 'sum'"),
            Token::Then { position: _ } => write!(f, "THEN 'then'"),
            Token::Time { position: _ } => write!(f, "TIME 'time'"),
            Token::To { position: _ } => write!(f, "TO 'to'"),
            Token::Void { position: _ } => write!(f, "VOID 'void'"),
            Token::Write { position: _ } => write!(f, "WRITE 'write'"),
            Token::True { position: _ } => write!(f, "TRUE 'true'"),
            Token::False { position: _ } => write!(f, "FALSE 'false'"),
            Token::Equals { position: _ } => write!(f, "EQUALS '='"),
            Token::LParen { position: _ } => write!(f, "LPAREN '('"),
            Token::RParen { position: _ } => write!(f, "RPAREN ')'"),
            Token::LCurly { position: _ } => write!(f, "LCURLY '{{'"),
            Token::RCurly { position: _ } => write!(f, "RCURLY '}}'"),
            Token::LSquare { position: _ } => write!(f, "LSQUARE '['"),
            Token::RSquare { position: _ } => write!(f, "RSQUARE ']'"),
            Token::Comma { position: _ } => write!(f, "COMMA ','"),
            Token::Colon { position: _ } => write!(f, "COLON ':'"),
            Token::Dot { position: _ } => write!(f, "DOT '.'"),
            Token::Newline { position: _ } => write!(f, "NEWLINE"),
            Token::EndOfFile { position: _ } => write!(f, "END_OF_FILE"),
        }
    }
}

pub struct LexerError {
    message: String,
    file: String,
    line: usize,
    column: usize,
}

impl Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Lex error: {}:{}:{}: {}",
            self.file, self.line, self.column, self.message
        )
    }
}

/// Represents the state of the lexer and provides helper methods
struct Lexer<'a> {
    program: &'a str,
    bytes: &'a [u8],
    file_name: &'a str,
    len: usize,
    position: usize,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    /// Creates a new lexer for the given program
    fn new(program: &'a str, file_name: &'a str) -> Self {
        Lexer {
            program,
            bytes: program.as_bytes(),
            file_name,
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
            file: self.file_name.to_string(),
            line: self.line,
            column: self.column,
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
            "array" => Token::Array { position: pos },
            "assert" => Token::Assert { position: pos },
            "bool" => Token::Bool { position: pos },
            "else" => Token::Else { position: pos },
            "false" => Token::False { position: pos },
            "float" => Token::Float { position: pos },
            "fn" => Token::Fn { position: pos },
            "if" => Token::If { position: pos },
            "image" => Token::Image { position: pos },
            "int" => Token::Int { position: pos },
            "let" => Token::Let { position: pos },
            "print" => Token::Print { position: pos },
            "read" => Token::Read { position: pos },
            "return" => Token::Return { position: pos },
            "show" => Token::Show { position: pos },
            "struct" => Token::Struct { position: pos },
            "sum" => Token::Sum { position: pos },
            "then" => Token::Then { position: pos },
            "time" => Token::Time { position: pos },
            "to" => Token::To { position: pos },
            "true" => Token::True { position: pos },
            "void" => Token::Void { position: pos },
            "write" => Token::Write { position: pos },
            _ => Token::Variable {
                position: pos,
                value: word,
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
            Token::FloatVal {
                position: Position {
                    line: self.line,
                    column: self.column,
                },
                value: number,
            }
        } else {
            Token::IntVal {
                position: Position {
                    line: self.line,
                    column: self.column,
                },
                value: number,
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

            Token::FloatVal {
                position: Position {
                    line: self.line,
                    column: self.column,
                },
                value: &self.program[start..self.position],
            }
        } else {
            // It's just a dot
            Token::Dot {
                position: Position {
                    line: self.line,
                    column: self.column - 1,
                },
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
        Ok(Token::StringLiteral {
            position: Position {
                line: self.line,
                column: self.column,
            },
            value: string_literal,
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
                return Token::Op {
                    position: pos,
                    value: potential_op,
                };
            }
        }

        // Single-character tokens
        let token = match self.bytes[self.position] {
            b'(' => Token::LParen { position: pos },
            b')' => Token::RParen { position: pos },
            b'{' => Token::LCurly { position: pos },
            b'}' => Token::RCurly { position: pos },
            b'[' => Token::LSquare { position: pos },
            b']' => Token::RSquare { position: pos },
            b',' => Token::Comma { position: pos },
            b':' => Token::Colon { position: pos },
            b'=' => Token::Equals { position: pos },
            _ => Token::Op {
                position: pos,
                value: &self.program[self.position..self.position + 1],
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
                        tokens.push(Token::Op {
                            position: Position {
                                line: self.line,
                                column: self.column,
                            },
                            value: "/",
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
                    if tokens.is_empty() || !matches!(tokens.last().unwrap(), Token::Newline { .. })
                    {
                        tokens.push(Token::Newline {
                            position: Position {
                                line: self.line,
                                column: self.column,
                            },
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
        tokens.push(Token::EndOfFile {
            position: Position {
                line: self.line,
                column: self.column,
            },
        });

        Ok(tokens)
    }
}

// Helper function to check if a character is valid
fn is_valid(c: char) -> bool {
    (c as u32) >= 32 && (c as u32) <= 126
}

// Optimized lexer implementation - main entry point
pub fn lex<'a>(program: &'a str, file_name: &'a str) -> Result<Vec<Token<'a>>, LexerError> {
    let mut lexer = Lexer::new(program, file_name);
    lexer.lex()
}
