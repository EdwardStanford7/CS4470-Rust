use crate::utils::*;
use core::str;

/// Represents the state of the lexer and provides helper methods
struct Lexer<'a> {
    program: &'a str,
    position: usize,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    /// Creates a new lexer for the given program
    fn new(program: &'a str) -> Self {
        Lexer {
            program,
            position: 0,
            line: 1,
            column: 0,
        }
    }

    // Helper function to check if a character is valid
    fn is_valid(c: u8) -> bool {
        (32..=126).contains(&c)
    }

    /// Get the current byte
    /// Returns 0 if the position is at the end of the program
    fn current_byte(&self) -> u8 {
        self.program
            .as_bytes()
            .get(self.position)
            .cloned()
            .unwrap_or(0)
    }

    /// Get the next byte
    /// Returns 0 if the position is at the end of the program
    fn peek_byte(&self) -> u8 {
        self.program
            .as_bytes()
            .get(self.position + 1)
            .cloned()
            .unwrap_or(0)
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
    fn error(&self, message: &str) -> LexError {
        LexError::new(message.to_string(), self.current_position())
    }

    /// Get the current position in a Position struct
    fn current_position(&self) -> Position {
        Position::new(self.line, self.column)
    }

    /// Lex an identifier or keyword
    fn lex_identifier(&mut self) -> Token<'a> {
        let start = self.position;

        // Consume all alphanumeric characters
        loop {
            let c = self.current_byte();
            if !(c.is_ascii_alphanumeric() || c == b'_') {
                break;
            }
            self.advance();
        }

        let word = &self.program[start..self.position];
        let pos = self.current_position();

        // Check if this is a keyword
        match word {
            "array" => Token {
                position: pos,
                token_type: TokenType::Array,
            },
            "assert" => Token {
                position: pos,
                token_type: TokenType::Assert,
            },
            "bool" => Token {
                position: pos,
                token_type: TokenType::Bool,
            },
            "else" => Token {
                position: pos,
                token_type: TokenType::Else,
            },
            "false" => Token {
                position: pos,
                token_type: TokenType::False,
            },
            "float" => Token {
                position: pos,
                token_type: TokenType::Float,
            },
            "fn" => Token {
                position: pos,
                token_type: TokenType::Fn,
            },
            "if" => Token {
                position: pos,
                token_type: TokenType::If,
            },
            "image" => Token {
                position: pos,
                token_type: TokenType::Image,
            },
            "int" => Token {
                position: pos,
                token_type: TokenType::Int,
            },
            "let" => Token {
                position: pos,
                token_type: TokenType::Let,
            },
            "print" => Token {
                position: pos,
                token_type: TokenType::Print,
            },
            "read" => Token {
                position: pos,
                token_type: TokenType::Read,
            },
            "return" => Token {
                position: pos,
                token_type: TokenType::Return,
            },
            "show" => Token {
                position: pos,
                token_type: TokenType::Show,
            },
            "struct" => Token {
                position: pos,
                token_type: TokenType::Struct,
            },
            "sum" => Token {
                position: pos,
                token_type: TokenType::Sum,
            },
            "then" => Token {
                position: pos,
                token_type: TokenType::Then,
            },
            "time" => Token {
                position: pos,
                token_type: TokenType::Time,
            },
            "to" => Token {
                position: pos,
                token_type: TokenType::To,
            },
            "true" => Token {
                position: pos,
                token_type: TokenType::True,
            },
            "void" => Token {
                position: pos,
                token_type: TokenType::Void,
            },
            "write" => Token {
                position: pos,
                token_type: TokenType::Write,
            },
            _ => Token {
                position: pos,
                token_type: TokenType::Variable(word),
            },
        }
    }

    /// Lex a number (integer or float)
    fn lex_number(&mut self) -> Token<'a> {
        let start = self.position;
        let mut has_dot = false;

        // Consume all numeric characters
        loop {
            let c = self.current_byte();
            if c.is_ascii_digit() {
                self.advance();
            } else if c == b'.' && !has_dot {
                has_dot = true;
                self.advance();
            } else {
                break;
            }
        }

        let number = &self.program[start..self.position];
        let position = self.current_position();

        if has_dot {
            Token {
                position,
                token_type: TokenType::FloatVal(number),
            }
        } else {
            Token {
                position,
                token_type: TokenType::IntVal(number),
            }
        }
    }

    /// Lex a dot (could be a float or struct access)
    fn lex_dot(&mut self) -> Token<'a> {
        let start = self.position;
        self.advance(); // Consume the dot

        // Check if it's a float starting with a dot
        if self.current_byte().is_ascii_digit() {
            while self.current_byte().is_ascii_digit() {
                self.advance();
            }

            return Token {
                position: self.current_position(),
                token_type: TokenType::FloatVal(&self.program[start..self.position]),
            };
        }

        // It's just a dot
        Token {
            position: Position::new(self.line, self.column - 1),
            token_type: TokenType::Dot,
        }
    }

    /// Lex a string literal
    fn lex_string(&mut self) -> Result<Token<'a>, LexError> {
        let start = self.position;
        self.advance(); // Skip opening quote

        while self.current_byte() != b'"' {
            let c = self.current_byte();

            if c == 0 {
                return Err(self.error("unterminated string"));
            }

            if !Self::is_valid(c) {
                return Err(self.error("invalid character in string"));
            }

            self.advance();
        }

        self.advance(); // Skip closing quote

        Ok(Token {
            position: self.current_position(),
            token_type: TokenType::String(&self.program[start..self.position]),
        })
    }

    /// Lex a comment (line or block)
    fn lex_comment(&mut self) -> Result<(), LexError> {
        self.advance(); // Skip the first /
        let mut c = self.current_byte();

        if c == b'/' {
            // Line comment
            self.advance(); // Skip the second /

            while self.current_byte() != b'\n' {
                if !Self::is_valid(self.current_byte()) {
                    return Err(self.error("invalid character in comment"));
                }
                self.advance();
            }
            Ok(())
        } else {
            // Block comment
            self.advance(); // Skip the *

            // let mut found_end = false;
            loop {
                c = self.current_byte();
                if c == b'*' && self.peek_byte() == b'/' {
                    self.advance(); // Skip *
                    self.advance(); // Skip /
                    break;
                } else if c == 0 {
                    return Err(self.error("unterminated block comment"));
                } else if c == b'\n' {
                    self.advance_newline();
                } else {
                    if !Self::is_valid(c) && c != b'\n' {
                        return Err(self.error("invalid character in block comment"));
                    }
                    self.advance();
                }
            }

            Ok(())
        }
    }

    /// Lex operators and delimiters
    fn lex_operator(&mut self) -> Token<'a> {
        let pos = self.current_position();

        // Check for two-character operators
        if self.peek_byte() != 0 {
            let potential_op = &self.program[self.position..self.position + 2];
            if ["==", "<=", ">=", "!=", "&&", "||"].contains(&potential_op) {
                self.position += 2;
                self.column += 2;
                return Token {
                    position: pos,
                    token_type: TokenType::Op(potential_op),
                };
            }
        }

        // Single-character tokens
        let token = match self.current_byte() {
            b'(' => Token {
                position: pos,
                token_type: TokenType::LParen,
            },
            b')' => Token {
                position: pos,
                token_type: TokenType::RParen,
            },
            b'{' => Token {
                position: pos,
                token_type: TokenType::LCurly,
            },
            b'}' => Token {
                position: pos,
                token_type: TokenType::RCurly,
            },
            b'[' => Token {
                position: pos,
                token_type: TokenType::LSquare,
            },
            b']' => Token {
                position: pos,
                token_type: TokenType::RSquare,
            },
            b',' => Token {
                position: pos,
                token_type: TokenType::Comma,
            },
            b'=' => Token {
                position: pos,
                token_type: TokenType::Equals,
            },
            b':' => Token {
                position: pos,
                token_type: TokenType::Colon,
            },
            _ => Token {
                position: pos,
                token_type: TokenType::Op(&self.program[self.position..self.position + 1]),
            },
        };

        self.advance();
        token
    }

    /// Run the lexer to produce tokens
    fn lex(&mut self) -> Result<Vec<Token<'a>>, LexError> {
        let mut tokens: Vec<Token<'a>> = Vec::with_capacity(self.program.len() / 4);

        while self.current_byte() != 0 {
            let c = self.current_byte();

            match c {
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
                            position: self.current_position(),
                            token_type: TokenType::Op(
                                &self.program[self.position..self.position + 1],
                            ),
                        });
                        self.advance();
                    }
                }

                // Line continuation
                b'\\' => {
                    self.advance();
                    if self.current_byte() == b'\n' {
                        self.advance_newline();
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
                            position: self.current_position(),
                            token_type: TokenType::Newline,
                        });
                    }

                    self.advance_newline();
                }

                // Whitespace
                b' ' => {
                    self.advance();
                }

                // Operators and delimiters
                _ if Self::is_valid(c) => {
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
            position: self.current_position(),
            token_type: TokenType::EndOfFile,
        });

        Ok(tokens)
    }
}

pub fn lex(program: &str) -> Result<Vec<Token>, LexError> {
    let mut lexer = Lexer::new(program);
    lexer.lex()
}
