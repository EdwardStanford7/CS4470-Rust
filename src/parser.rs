use crate::ast::{ASTNode, ASTVisitor, Binop, Command, Expression, LValue, Statement, Type, Unop};
use crate::lexer::{Position, Token};
use core::fmt;
use std::fmt::Display;
use std::iter::Peekable;
use std::slice::Iter;

pub struct ParserError {
    message: String,
    file: String,
    line: usize,
    column: usize,
}

impl Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Parse error: {}:{}:{}: {}",
            self.file, self.line, self.column, self.message
        )
    }
}

pub struct Parser<'a> {
    tokens: Peekable<Iter<'a, Token<'a>>>,
    file_name: &'a str,
    index: usize,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a Vec<Token<'a>>, file_name: &'a str) -> Self {
        Self {
            tokens: tokens.iter().peekable(),
            file_name,
            index: 0,
        }
    }

    pub fn parse(&mut self) -> Result<Vec<Command<'a>>, ParserError> {
        let mut commands = Vec::new();

        while let Some(token) = self.tokens.peek() {
            match token {
                Token::Let { .. } => commands.push(self.parse_let_command()),
                Token::Read { .. } => commands.push(self.parse_read_command()),
                Token::Write { .. } => commands.push(self.parse_write_command()),
                Token::Assert { .. } => commands.push(self.parse_assert_command()),
                Token::Print { .. } => commands.push(self.parse_print_command()),
                Token::Show { .. } => commands.push(self.parse_show_command()),
                Token::Time { .. } => commands.push(self.parse_time_command()),
                Token::Struct { .. } => commands.push(self.parse_struct_command()),
                _ => {
                    return Err(ParserError {
                        message: "Unexpected token".to_string(),
                        file: "unknown".to_string(),
                        line: 0,
                        column: 0,
                    });
                }
            }
        }
        Ok(commands)
    }
}

// Example usage
pub fn parse<'a>(
    tokens: &'a Vec<Token<'a>>,
    file_name: &'a str,
) -> Result<Vec<Command<'a>>, ParserError> {
    let mut parser = Parser::new(tokens, file_name);
    parser.parse()
}
