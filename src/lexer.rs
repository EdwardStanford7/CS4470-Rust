use core::fmt;
use std::fmt::Display;

#[derive(PartialEq, Debug)]
pub enum TokenType {
    // Values
    IntVal(String),
    FloatVal(String),
    Variable(String),
    StringLiteral(String),

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
    Op(String),
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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let token = match self {
            TokenType::IntVal(val) => format!("INTVAL '{}'", val),
            TokenType::FloatVal(val) => format!("FLOATVAL '{}'", val),
            TokenType::Variable(val) => format!("VARIABLE '{}'", val),
            TokenType::StringLiteral(val) => format!("STRING '{}'", val),
            TokenType::Array => "ARRAY 'array'".to_string(),
            TokenType::Assert => "ASSERT 'assert'".to_string(),
            TokenType::Bool => "BOOL 'bool'".to_string(),
            TokenType::Else => "ELSE 'else'".to_string(),
            TokenType::Fn => "FN 'fn'".to_string(),
            TokenType::If => "IF 'if'".to_string(),
            TokenType::Image => "IMAGE 'image'".to_string(),
            TokenType::Int => "INT 'int'".to_string(),
            TokenType::Float => "FLOAT 'float'".to_string(),
            TokenType::Let => "LET 'let'".to_string(),
            TokenType::Print => "PRINT 'print'".to_string(),
            TokenType::Read => "READ 'read'".to_string(),
            TokenType::Return => "RETURN 'return'".to_string(),
            TokenType::Show => "SHOW 'show'".to_string(),
            TokenType::Struct => "STRUCT 'struct'".to_string(),
            TokenType::Sum => "SUM 'sum'".to_string(),
            TokenType::Then => "THEN 'then'".to_string(),
            TokenType::Time => "TIME 'time'".to_string(),
            TokenType::To => "TO 'to'".to_string(),
            TokenType::Void => "VOID 'void'".to_string(),
            TokenType::Write => "WRITE 'write'".to_string(),
            TokenType::True => "TRUE 'true'".to_string(),
            TokenType::False => "FALSE 'false'".to_string(),
            TokenType::Op(val) => format!("OP '{}'", val),
            TokenType::Equals => "EQUALS '='".to_string(),
            TokenType::LParen => "LPAREN '('".to_string(),
            TokenType::RParen => "RPAREN ')'".to_string(),
            TokenType::LCurly => "LCURLY '{'".to_string(),
            TokenType::RCurly => "RCURLY '}'".to_string(),
            TokenType::LSquare => "LSQUARE '['".to_string(),
            TokenType::RSquare => "RSQUARE ']'".to_string(),
            TokenType::Comma => "COMMA ','".to_string(),
            TokenType::Colon => "COLON ':'".to_string(),
            TokenType::Dot => "DOT '.'".to_string(),
            TokenType::Newline => "NEWLINE".to_string(),
            TokenType::EndOfFile => "END_OF_FILE".to_string(),
        };

        write!(f, "{}", token)
    }
}

#[derive(Debug)]
pub struct Token {
    token_type: TokenType,
    line: usize,
    column: usize,
}

// impl Token {
//     pub fn token_type(&self) -> &TokenType {
//         &self.token_type
//     }

//     pub fn line(&self) -> usize {
//         self.line
//     }

//     pub fn column(&self) -> usize {
//         self.column
//     }
// }

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.token_type,)
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

pub fn lex(program: &str, file_name: &str) -> Result<Vec<Token>, LexerError> {
    let mut tokens: Vec<Token> = vec![];
    let mut position = 0;
    let mut line = 1;
    let mut column = 0;

    loop {
        if position >= program.len() {
            break;
        }

        let character = program.chars().nth(position).unwrap();
        match character {
            c if c.is_alphabetic() => {
                // Handle keywords and variables
                tokens.push(lex_keyword_variable(
                    program,
                    &mut position,
                    line,
                    &mut column,
                ));
            }
            c if c.is_numeric() => {
                // Handle numbers
                tokens.push(lex_number(program, &mut position, &line, &mut column));
            }
            '\"' => {
                // Handle string literals
                let token = lex_string(program, file_name, &mut position, &line, &mut column);
                match token {
                    Ok(token) => tokens.push(token),
                    Err(e) => return Err(e),
                }
            }
            '/' => {
                match program.chars().nth(position + 1).unwrap() {
                    '/' | '*' => {
                        // Handle comments
                        lex_comment(program, file_name, &mut position, &mut line, &mut column)?
                    }
                    _ => {
                        tokens.push(Token {
                            token_type: TokenType::Op("/".to_string()),
                            line,
                            column,
                        });
                        position += 1;
                        column += 1;
                    }
                }
            }
            '.' => {
                // Handle dot operator (could be float or struct access)
                tokens.push(lex_dot(program, &mut position, &line, &mut column));
            }
            '\\' => {
                // Line continuation
                position += 2;
                line += 1;
                column = 0;
            }
            '\n' => {
                // Add newline token if the last token is not a newline
                if tokens.is_empty() || tokens.last().unwrap().token_type != TokenType::Newline {
                    tokens.push(Token {
                        token_type: TokenType::Newline,
                        line,
                        column,
                    });
                }

                // Skip newline character
                position += 1;
                line += 1;
                column = 0;
            }
            ' ' => {
                // Whitespace
                position += 1;
                column += 1;
            }
            c if is_valid(c) => {
                // Handle other characters (operators, delimiters)
                let token =
                    lex_punctuation_operator(program, file_name, &mut position, &line, &mut column);
                match token {
                    Ok(token) => tokens.push(token),
                    Err(e) => return Err(e),
                }
            }
            _ => {
                // Handle invalid characters
                return Err(LexerError {
                    message: "invalid character".to_string(),
                    file: file_name.to_string(),
                    line,
                    column,
                });
            }
        }
    }

    tokens.push(Token {
        token_type: TokenType::EndOfFile,
        line,
        column,
    });

    Ok(tokens)
}

// TODO, is this necessary since utf8 invalid is checked when reading in file?
fn is_valid(c: char) -> bool {
    32 <= c as u8 && c as u8 <= 126
}

fn lex_keyword_variable(
    program: &str,
    position: &mut usize,
    line: usize,
    column: &mut usize,
) -> Token {
    let start = *position;
    while *position < program.len()
        && (program.chars().nth(*position).unwrap().is_alphabetic()
            || program.chars().nth(*position).unwrap().is_numeric()
            || program.chars().nth(*position).unwrap() == '_')
    {
        *position += 1;
        *column += 1;
    }

    let column = *column;

    let substr = &program[start..*position];
    match substr {
        "array" => Token {
            token_type: TokenType::Array,
            line,
            column,
        },
        "assert" => Token {
            token_type: TokenType::Assert,
            line,
            column,
        },
        "bool" => Token {
            token_type: TokenType::Bool,
            line,
            column,
        },
        "else" => Token {
            token_type: TokenType::Else,
            line,
            column,
        },
        "false" => Token {
            token_type: TokenType::False,
            line,
            column,
        },
        "float" => Token {
            token_type: TokenType::Float,
            line,
            column,
        },
        "fn" => Token {
            token_type: TokenType::Fn,
            line,
            column,
        },
        "if" => Token {
            token_type: TokenType::If,
            line,
            column,
        },
        "image" => Token {
            token_type: TokenType::Image,
            line,
            column,
        },
        "int" => Token {
            token_type: TokenType::Int,
            line,
            column,
        },
        "let" => Token {
            token_type: TokenType::Let,
            line,
            column,
        },
        "print" => Token {
            token_type: TokenType::Print,
            line,
            column,
        },
        "read" => Token {
            token_type: TokenType::Read,
            line,
            column,
        },
        "return" => Token {
            token_type: TokenType::Return,
            line,
            column,
        },
        "show" => Token {
            token_type: TokenType::Show,
            line,
            column,
        },
        "struct" => Token {
            token_type: TokenType::Struct,
            line,
            column,
        },
        "sum" => Token {
            token_type: TokenType::Sum,
            line,
            column,
        },
        "then" => Token {
            token_type: TokenType::Then,
            line,
            column,
        },
        "time" => Token {
            token_type: TokenType::Time,
            line,
            column,
        },
        "to" => Token {
            token_type: TokenType::To,
            line,
            column,
        },
        "true" => Token {
            token_type: TokenType::True,
            line,
            column,
        },
        "void" => Token {
            token_type: TokenType::Void,
            line,
            column,
        },
        "write" => Token {
            token_type: TokenType::Write,
            line,
            column,
        },
        _ => Token {
            token_type: TokenType::Variable(substr.to_string()),
            line,
            column,
        },
    }
}

fn lex_number(program: &str, position: &mut usize, line: &usize, column: &mut usize) -> Token {
    let start = *position;
    let mut has_dot = false;

    while let Some(c) = program.chars().nth(*position) {
        if c.is_numeric() {
            *position += 1;
            *column += 1;
        } else if c == '.' && !has_dot {
            has_dot = true;
            *position += 1;
            *column += 1;
        } else {
            break;
        }
    }

    let substr = &program[start..*position];
    if has_dot {
        Token {
            token_type: TokenType::FloatVal(substr.to_string()),
            line: *line,
            column: *column,
        }
    } else {
        Token {
            token_type: TokenType::IntVal(substr.to_string()),
            line: *line,
            column: *column,
        }
    }
}

fn lex_dot(program: &str, position: &mut usize, line: &usize, column: &mut usize) -> Token {
    *position += 1;
    *column += 1;

    if program.chars().nth(*position).unwrap().is_numeric() {
        let start = *position - 1;
        while let Some(c) = program.chars().nth(*position) {
            if c.is_numeric() {
                *position += 1;
                *column += 1;
            } else {
                break;
            }
        }

        let substr = &program[start..*position];
        return Token {
            token_type: TokenType::FloatVal(substr.to_string()),
            line: *line,
            column: *column,
        };
    };

    Token {
        token_type: TokenType::Dot,
        line: *line,
        column: *column,
    }
}

fn lex_string(
    program: &str,
    file_name: &str,
    position: &mut usize,
    line: &usize,
    column: &mut usize,
) -> Result<Token, LexerError> {
    let start = *position;

    *position += 1;
    *column += 1;
    loop {
        let c = program.chars().nth(*position).unwrap();
        if c == '"' {
            break;
        }
        if !is_valid(c) {
            return Err(LexerError {
                message: "invalid character".to_string(),
                file: file_name.to_string(),
                line: *line,
                column: *column,
            });
        }
        *position += 1;
        *column += 1;
    }
    *position += 1;
    *column += 1;

    let substr = &program[start..*position];

    Ok(Token {
        token_type: TokenType::StringLiteral(substr.to_string()),
        line: *line,
        column: *column,
    })
}

fn lex_comment(
    program: &str,
    file_name: &str,
    position: &mut usize,
    line: &mut usize,
    column: &mut usize,
) -> Result<(), LexerError> {
    if program.chars().nth(*position + 1).ok_or(LexerError {
        message: "/ is not a valid token'".to_string(),
        file: file_name.to_string(),
        line: *line,
        column: *column,
    })? == '/'
    {
        // Inline comment, go until new line.
        while let Some(c) = program.chars().nth(*position) {
            if c == '\n' {
                break;
            }
            if !is_valid(c) {
                return Err(LexerError {
                    message: "invalid character".to_string(),
                    file: file_name.to_string(),
                    line: *line,
                    column: *column,
                });
            }
            *position += 1;
            *column += 1;
        }
    } else if program.chars().nth(*position + 1).ok_or(LexerError {
        message: "Could not find '*/'".to_string(),
        file: file_name.to_string(),
        line: *line,
        column: *column,
    })? == '*'
    {
        // Block comment: go until finding "*/"
        *position += 2;
        *column += 2;
        while *position < program.len() {
            let c = program.chars().nth(*position).ok_or(LexerError {
                message: "Could not find '*/'".to_string(),
                file: file_name.to_string(),
                line: *line,
                column: *column,
            })?;
            if c == '*' && program.chars().nth(*position + 1).unwrap_or('\0') == '/' {
                *position += 2;
                *column += 2;
                break;
            }
            if !(is_valid(c) || c == '\n') {
                return Err(LexerError {
                    message: "invalid character".to_string(),
                    file: file_name.to_string(),
                    line: *line,
                    column: *column,
                });
            }
            if c == '\n' {
                *line += 1;
                *column = 0;
            } else {
                *column += 1;
            }
            *position += 1;
        }
    }
    Ok(())
}

fn lex_punctuation_operator(
    program: &str,
    file_name: &str,
    position: &mut usize,
    line: &usize,
    column: &mut usize,
) -> Result<Token, LexerError> {
    let character = program.chars().nth(*position).unwrap();
    let double_char_operator = &program[*position..*position + 2];

    if !is_valid(character) {
        return Err(LexerError {
            message: "invalid character".to_string(),
            file: file_name.to_string(),
            line: *line,
            column: *column,
        });
    }

    let token = match double_char_operator {
        "==" | "<=" | ">=" | "!=" | "&&" | "||" => {
            *position += 2;
            *column += 2;
            TokenType::Op(double_char_operator.to_string())
        }
        _ => {
            *position += 1;
            *column += 1;
            match character {
                '(' => TokenType::LParen,
                ')' => TokenType::RParen,
                '{' => TokenType::LCurly,
                '}' => TokenType::RCurly,
                '[' => TokenType::LSquare,
                ']' => TokenType::RSquare,
                ',' => TokenType::Comma,
                ':' => TokenType::Colon,
                '=' => TokenType::Equals,
                _ => TokenType::Op(character.to_string()),
            }
        }
    };

    Ok(Token {
        token_type: token,
        line: *line,
        column: *column,
    })
}
