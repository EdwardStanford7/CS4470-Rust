use core::fmt;
use std::fmt::Display;

#[derive(PartialEq, Debug)]
pub enum TokenType<'a> {
    // Values - using &str references instead of owned Strings
    IntVal(&'a str),
    FloatVal(&'a str),
    Variable(&'a str),
    StringLiteral(&'a str),

    // Keywords - these don't need to change as they're fixed values
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

    // Operators - using &str references
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

impl<'a> Display for TokenType<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TokenType::IntVal(val) => write!(f, "INTVAL '{}'", val),
            TokenType::FloatVal(val) => write!(f, "FLOATVAL '{}'", val),
            TokenType::Variable(val) => write!(f, "VARIABLE '{}'", val),
            TokenType::StringLiteral(val) => write!(f, "STRING '{}'", val),
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
            TokenType::Op(val) => write!(f, "OP '{}'", val),
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

#[derive(Debug)]
pub struct Token<'a> {
    token_type: TokenType<'a>,
    line: usize,
    column: usize,
}

impl<'a> Token<'a> {
    pub fn token_type(&self) -> &TokenType<'a> {
        &self.token_type
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn column(&self) -> usize {
        self.column
    }
}

impl<'a> Display for Token<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.token_type)
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

        // Check if this is a keyword
        let token_type = match word {
            "array" => TokenType::Array,
            "assert" => TokenType::Assert,
            "bool" => TokenType::Bool,
            "else" => TokenType::Else,
            "false" => TokenType::False,
            "float" => TokenType::Float,
            "fn" => TokenType::Fn,
            "if" => TokenType::If,
            "image" => TokenType::Image,
            "int" => TokenType::Int,
            "let" => TokenType::Let,
            "print" => TokenType::Print,
            "read" => TokenType::Read,
            "return" => TokenType::Return,
            "show" => TokenType::Show,
            "struct" => TokenType::Struct,
            "sum" => TokenType::Sum,
            "then" => TokenType::Then,
            "time" => TokenType::Time,
            "to" => TokenType::To,
            "true" => TokenType::True,
            "void" => TokenType::Void,
            "write" => TokenType::Write,
            _ => TokenType::Variable(word),
        };

        Token {
            token_type,
            line: self.line,
            column: self.column,
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
        let token_type = if has_dot {
            TokenType::FloatVal(number)
        } else {
            TokenType::IntVal(number)
        };

        Token {
            token_type,
            line: self.line,
            column: self.column,
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

            let float_val = &self.program[start..self.position];
            Token {
                token_type: TokenType::FloatVal(float_val),
                line: self.line,
                column: self.column,
            }
        } else {
            // It's just a dot
            Token {
                token_type: TokenType::Dot,
                line: self.line,
                column: self.column - 1,
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
            token_type: TokenType::StringLiteral(string_literal),
            line: self.line,
            column: self.column,
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
        // Check for two-character operators
        if self.position + 1 < self.len {
            let potential_op = &self.program[self.position..self.position + 2];

            match potential_op {
                "==" | "<=" | ">=" | "!=" | "&&" | "||" => {
                    let token = Token {
                        token_type: TokenType::Op(potential_op),
                        line: self.line,
                        column: self.column,
                    };
                    self.position += 2;
                    self.column += 2;
                    return token;
                }
                _ => {}
            }
        }

        // Single-character tokens
        let byte = self.bytes[self.position];
        let token_type = match byte {
            b'(' => TokenType::LParen,
            b')' => TokenType::RParen,
            b'{' => TokenType::LCurly,
            b'}' => TokenType::RCurly,
            b'[' => TokenType::LSquare,
            b']' => TokenType::RSquare,
            b',' => TokenType::Comma,
            b':' => TokenType::Colon,
            b'=' => TokenType::Equals,
            _ => TokenType::Op(&self.program[self.position..self.position + 1]),
        };

        let token = Token {
            token_type,
            line: self.line,
            column: self.column,
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
                    if self.position + 1 < self.len {
                        match self.peek_byte() {
                            b'/' | b'*' => {
                                self.lex_comment()?;
                            }
                            _ => {
                                tokens.push(Token {
                                    token_type: TokenType::Op("/"),
                                    line: self.line,
                                    column: self.column,
                                });
                                self.advance();
                            }
                        }
                    } else {
                        // Division at end of input
                        tokens.push(Token {
                            token_type: TokenType::Op("/"),
                            line: self.line,
                            column: self.column,
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
                    if tokens.is_empty() || tokens.last().unwrap().token_type != TokenType::Newline
                    {
                        tokens.push(Token {
                            token_type: TokenType::Newline,
                            line: self.line,
                            column: self.column,
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
            token_type: TokenType::EndOfFile,
            line: self.line,
            column: self.column,
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
