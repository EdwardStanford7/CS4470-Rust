use crate::ast::*;
use crate::lexer::*;
use core::fmt;
use std::cell::Cell;
use std::cell::RefCell;
use std::fmt::Display;
use std::rc::Rc;

pub struct ParserError {
    message: String,
    position: Position,
}

impl Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Parse error: {}:{}: {}",
            self.position.line, self.position.column, self.message
        )
    }
}

struct Parser<'a> {
    tokens: Vec<Token<'a>>,
    index: Cell<usize>,
}

impl<'a> Parser<'a> {
    fn new(tokens: Vec<Token<'a>>) -> Self {
        Self {
            tokens,
            index: 0.into(),
        }
    }

    fn parse(&mut self) -> Result<Vec<Command<'a>>, ParserError> {
        let mut commands = Vec::new();

        if self.peek_token().token_type == TokenType::Newline {
            self.expect_token(TokenType::Newline)?;
        }

        while self.peek_token().token_type != TokenType::EndOfFile {
            commands.push(self.parse_command()?);
            self.expect_token(TokenType::Newline)?;
            if self.peek_token().token_type == TokenType::EndOfFile {
                break;
            }
        }

        Ok(commands)
    }

    fn expect_token<'b>(&'b self, expected: TokenType) -> Result<&'b Token<'a>, ParserError> {
        let token = &self.tokens[self.index.get()];
        self.index.replace(self.index.get() + 1);
        if token.token_type == expected {
            Ok(token)
        } else {
            Err(ParserError {
                message: format!("Expected token {}, got {}", expected, token.token_type),
                position: token.position.clone(),
            })
        }
    }

    fn peek_token(&self) -> &Token<'a> {
        self.tokens.get(self.index.get()).unwrap()
    }

    fn peek_next_token(&self) -> &Token<'a> {
        self.tokens.get(self.index.get() + 1).unwrap()
    }

    // ----------------------------------------------------------------------------------- Command Parsers ----------------------------------------------------------------------------------------------

    fn parse_command(&mut self) -> Result<Command<'a>, ParserError> {
        match self.peek_token().token_type {
            TokenType::Read => self.parse_read_command(),
            TokenType::Write => self.parse_write_command(),
            TokenType::Let => self.parse_let_command(),
            TokenType::Assert => self.parse_assert_command(),
            TokenType::Print => self.parse_print_command(),
            TokenType::Show => self.parse_show_command(),
            TokenType::Time => self.parse_time_command(),
            TokenType::Fn => self.parse_fn_command(),
            TokenType::Struct => self.parse_struct_command(),
            _ => Err(ParserError {
                message: format!("Expected command, got {}", self.peek_token().token_type),
                position: self.peek_token().position.clone(),
            }),
        }
    }

    fn parse_read_command(&mut self) -> Result<Command<'a>, ParserError> {
        let position = self.expect_token(TokenType::Read)?.position.clone();
        self.expect_token(TokenType::Image)?;
        let source = self.expect_token(TokenType::String)?.value.unwrap();
        self.expect_token(TokenType::To)?;
        let destination = self.parse_lvalue()?;

        Ok(Command {
            position,
            node: CommandType::Read {
                source,
                destination: Box::new(destination),
            },
        })
    }

    fn parse_write_command(&mut self) -> Result<Command<'a>, ParserError> {
        let position = self.expect_token(TokenType::Write)?.position.clone();
        self.expect_token(TokenType::Image)?;
        let source = self.parse_precedence1_expr()?;
        self.expect_token(TokenType::To)?;
        let destination = self.expect_token(TokenType::String)?.value.unwrap();

        Ok(Command {
            position,
            node: CommandType::Write {
                source: Box::new(source),
                destination,
            },
        })
    }

    fn parse_let_command(&mut self) -> Result<Command<'a>, ParserError> {
        let position = self.expect_token(TokenType::Let)?.position.clone();
        let variable = self.parse_lvalue()?;
        self.expect_token(TokenType::Equals)?;
        let expression = self.parse_precedence1_expr()?;

        Ok(Command {
            position,
            node: CommandType::Let {
                variable: Box::new(variable),
                rvalue: Box::new(expression),
            },
        })
    }

    fn parse_assert_command(&mut self) -> Result<Command<'a>, ParserError> {
        let position = self.expect_token(TokenType::Assert)?.position.clone();
        let expression = self.parse_precedence1_expr()?;
        self.expect_token(TokenType::Comma)?;
        let message = self.expect_token(TokenType::String)?.value.unwrap();
        Ok(Command {
            position,
            node: CommandType::Assert {
                condition: Box::new(expression),
                message,
            },
        })
    }

    fn parse_print_command(&mut self) -> Result<Command<'a>, ParserError> {
        let position = self.expect_token(TokenType::Print)?.position.clone();
        let message = self.expect_token(TokenType::String)?.value.unwrap();
        Ok(Command {
            position,
            node: CommandType::Print { message },
        })
    }

    fn parse_show_command(&mut self) -> Result<Command<'a>, ParserError> {
        let position = self.expect_token(TokenType::Show)?.position.clone();
        let expression = self.parse_precedence1_expr()?;
        Ok(Command {
            position,
            node: CommandType::Show {
                expression: Box::new(expression),
            },
        })
    }

    fn parse_time_command(&mut self) -> Result<Command<'a>, ParserError> {
        let position = self.expect_token(TokenType::Time)?.position.clone();
        let command = self.parse_command()?;
        Ok(Command {
            position,
            node: CommandType::Time {
                command: Box::new(command),
            },
        })
    }

    fn parse_struct_command(&mut self) -> Result<Command<'a>, ParserError> {
        let start_position = self.expect_token(TokenType::Struct)?.position.clone();
        let struct_name = self.expect_token(TokenType::Variable)?.value.unwrap();
        self.expect_token(TokenType::LCurly)?;
        self.expect_token(TokenType::Newline)?;

        let mut elements = Vec::new();
        while self.peek_token().token_type != TokenType::RCurly {
            let field_name = self.expect_token(TokenType::Variable)?.value.unwrap();
            self.expect_token(TokenType::Colon)?;
            let field_type = self.parse_type()?; // Still todo
            self.expect_token(TokenType::Newline)?;
            elements.push((field_name, field_type));
        }
        self.expect_token(TokenType::RCurly)?;
        Ok(Command {
            position: start_position,
            node: CommandType::Struct {
                name: struct_name,
                elements,
            },
        })
    }

    fn parse_fn_command(&mut self) -> Result<Command<'a>, ParserError> {
        let start_position = self.expect_token(TokenType::Fn)?.position.clone();
        let function_name = self.expect_token(TokenType::Variable)?.value.unwrap();
        self.expect_token(TokenType::LParen)?;

        let mut parameters = Vec::new();
        if self.peek_token().token_type != TokenType::RParen {
            let param_lvalue = self.parse_lvalue()?;
            self.expect_token(TokenType::Colon)?;
            let param_type = self.parse_type()?; // Still todo
            parameters.push((param_lvalue, param_type));
            while self.peek_token().token_type != TokenType::RParen {
                self.expect_token(TokenType::Comma)?;
                let param_lvalue = self.parse_lvalue()?;
                self.expect_token(TokenType::Colon)?;
                let param_type = self.parse_type()?;
                parameters.push((param_lvalue, param_type));
            }
        }
        self.expect_token(TokenType::RParen)?;
        self.expect_token(TokenType::Colon)?;
        let return_type = self.parse_type()?; // Still todo
        self.expect_token(TokenType::LCurly)?;
        self.expect_token(TokenType::Newline)?;

        let mut statements = Vec::new();
        while self.peek_token().token_type != TokenType::RCurly {
            statements.push(self.parse_statement()?);
            self.expect_token(TokenType::Newline)?;
        }
        self.expect_token(TokenType::RCurly)?;
        Ok(Command {
            position: start_position,
            node: CommandType::Function {
                name: function_name,
                parameters,
                return_type: Box::new(return_type),
                statements,
                has_return: false,
                local_scope: None,
            },
        })
    }

    // ----------------------------------------------------------------------------------- Expression Parsers ----------------------------------------------------------------------------------------------

    fn parse_int_expr(&mut self) -> Result<Expression<'a>, ParserError> {
        let token = self.expect_token(TokenType::IntVal)?;
        let value_str = token.value.as_ref().unwrap();
        let value = value_str.parse::<i64>().map_err(|_| ParserError {
            message: format!("Integer constant {} is too large", value_str),
            position: token.position.clone(),
        })?;
        Ok(Expression {
            position: token.position.clone(),
            node: ExpressionType::Int { value },
            resolved_type: Rc::new(RefCell::new(None)),
        })
    }

    fn parse_float_expr(&mut self) -> Result<Expression<'a>, ParserError> {
        let token = self.expect_token(TokenType::FloatVal)?;
        let value_str = token.value.as_ref().unwrap();
        let value = value_str.parse::<f64>().map_err(|_| ParserError {
            message: format!("Float constant {} is not valid", value_str),
            position: token.position.clone(),
        })?;
        if value.is_infinite() {
            return Err(ParserError {
                message: format!("Float constant {} is too large", value_str),
                position: token.position.clone(),
            });
        }
        Ok(Expression {
            position: token.position.clone(),
            node: ExpressionType::Float { value },
            resolved_type: Rc::new(RefCell::new(None)),
        })
    }

    fn parse_true_expr(&mut self) -> Result<Expression<'a>, ParserError> {
        let token = self.expect_token(TokenType::True)?;
        Ok(Expression {
            position: token.position.clone(),
            node: ExpressionType::True,
            resolved_type: Rc::new(RefCell::new(None)),
        })
    }

    fn parse_false_expr(&mut self) -> Result<Expression<'a>, ParserError> {
        let token = self.expect_token(TokenType::False)?;
        Ok(Expression {
            position: token.position.clone(),
            node: ExpressionType::False,
            resolved_type: Rc::new(RefCell::new(None)),
        })
    }

    fn parse_variable_expr(&mut self) -> Result<Expression<'a>, ParserError> {
        let token = self.expect_token(TokenType::Variable)?;
        Ok(Expression {
            position: token.position.clone(),
            node: ExpressionType::Variable {
                name: token.value.unwrap(),
            },
            resolved_type: Rc::new(RefCell::new(None)),
        })
    }

    fn parse_array_literal_expr(&mut self) -> Result<Expression<'a>, ParserError> {
        let position = self.expect_token(TokenType::LSquare)?.position.clone();
        // Empty array literal.
        if self.peek_token().token_type == TokenType::RSquare {
            self.expect_token(TokenType::RSquare)?;
            return Ok(Expression {
                position,
                node: ExpressionType::ArrayLiteral {
                    elements: Vec::new(),
                },
                resolved_type: Rc::new(RefCell::new(None)),
            });
        }
        let mut elements = Vec::new();
        // First element.
        elements.push(self.parse_precedence1_expr()?);
        while self.peek_token().token_type != TokenType::RSquare {
            self.expect_token(TokenType::Comma)?;
            elements.push(self.parse_precedence1_expr()?);
        }
        self.expect_token(TokenType::RSquare)?;
        Ok(Expression {
            position,
            node: ExpressionType::ArrayLiteral { elements },
            resolved_type: Rc::new(RefCell::new(None)),
        })
    }

    fn parse_call_expr(&mut self) -> Result<Expression<'a>, ParserError> {
        let start_token = self.expect_token(TokenType::Variable)?;
        let start_position = start_token.position.clone();
        let func_name = start_token.value.unwrap();

        self.expect_token(TokenType::LParen)?;
        let mut arguments = Vec::new();
        if self.peek_token().token_type != TokenType::RParen {
            arguments.push(self.parse_precedence1_expr()?);
            while self.peek_token().token_type != TokenType::RParen {
                self.expect_token(TokenType::Comma)?;
                arguments.push(self.parse_precedence1_expr()?);
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(Expression {
            position: start_position,
            node: ExpressionType::Call {
                function: func_name,
                arguments,
            },
            resolved_type: Rc::new(RefCell::new(None)),
        })
    }

    fn parse_struct_literal_expr(&mut self) -> Result<Expression<'a>, ParserError> {
        let start_token = self.expect_token(TokenType::Variable)?;
        let start_position = start_token.position.clone();
        let struct_name = start_token.value.unwrap();

        self.expect_token(TokenType::LCurly)?;
        let mut fields = Vec::new();
        if self.peek_token().token_type != TokenType::RCurly {
            fields.push(self.parse_precedence1_expr()?);
            while self.peek_token().token_type != TokenType::RCurly {
                self.expect_token(TokenType::Comma)?;
                fields.push(self.parse_precedence1_expr()?);
            }
        }
        self.expect_token(TokenType::RCurly)?;
        Ok(Expression {
            position: start_position,
            node: ExpressionType::StructLiteral {
                name: struct_name,
                fields,
            },
            resolved_type: Rc::new(RefCell::new(None)),
        })
    }

    fn parse_dot_expr(&mut self, left: Expression<'a>) -> Result<Expression<'a>, ParserError> {
        let dot_token = self.expect_token(TokenType::Dot)?;
        let field_token = self.expect_token(TokenType::Variable)?;
        Ok(Expression {
            position: dot_token.position.clone(),
            node: ExpressionType::Dot {
                struct_variable: Box::new(left),
                field: field_token.value.unwrap(),
            },
            resolved_type: Rc::new(RefCell::new(None)),
        })
    }

    fn parse_array_index_expr(
        &mut self,
        array: Expression<'a>,
    ) -> Result<Expression<'a>, ParserError> {
        let position = self.expect_token(TokenType::LSquare)?.position.clone();
        let mut indices = Vec::new();
        if self.peek_token().token_type == TokenType::RSquare {
            self.expect_token(TokenType::RSquare)?;
            return Ok(Expression {
                position,
                node: ExpressionType::ArrayIndex {
                    array: Box::new(array),
                    indices,
                },
                resolved_type: Rc::new(RefCell::new(None)),
            });
        }
        indices.push(self.parse_precedence1_expr()?);
        while self.peek_token().token_type != TokenType::RSquare {
            self.expect_token(TokenType::Comma)?;
            indices.push(self.parse_precedence1_expr()?);
        }
        self.expect_token(TokenType::RSquare)?;
        Ok(Expression {
            position,
            node: ExpressionType::ArrayIndex {
                array: Box::new(array),
                indices,
            },
            resolved_type: Rc::new(RefCell::new(None)),
        })
    }

    fn parse_precedent_expr(&mut self) -> Result<Expression<'a>, ParserError> {
        self.expect_token(TokenType::LParen)?;
        let expr = self.parse_precedence1_expr()?;
        self.expect_token(TokenType::RParen)?;
        Ok(expr)
    }

    fn parse_void_expr(&mut self) -> Result<Expression<'a>, ParserError> {
        let token = self.expect_token(TokenType::Void)?;
        Ok(Expression {
            position: token.position.clone(),
            node: ExpressionType::Void,
            resolved_type: Rc::new(RefCell::new(None)),
        })
    }

    fn parse_if_expr(&mut self) -> Result<Expression<'a>, ParserError> {
        let position = self.expect_token(TokenType::If)?.position.clone();
        let condition = Box::new(self.parse_precedence1_expr()?);
        self.expect_token(TokenType::Then)?;
        let then_branch = Box::new(self.parse_precedence1_expr()?);
        self.expect_token(TokenType::Else)?;
        let else_branch = Box::new(self.parse_precedence1_expr()?);
        Ok(Expression {
            position,
            node: ExpressionType::If {
                condition,
                then_branch,
                else_branch,
            },
            resolved_type: Rc::new(RefCell::new(None)),
        })
    }

    fn parse_array_loop_expr(&mut self) -> Result<Expression<'a>, ParserError> {
        let position = self.expect_token(TokenType::Array)?.position.clone();
        self.expect_token(TokenType::LSquare)?;
        let mut iterations = Vec::new();
        if self.peek_token().token_type != TokenType::RSquare {
            let variable = self.expect_token(TokenType::Variable)?.value.unwrap();
            self.expect_token(TokenType::Colon)?;
            let expr = self.parse_precedence1_expr()?;
            iterations.push((variable, expr));
        }
        while self.peek_token().token_type != TokenType::RSquare {
            self.expect_token(TokenType::Comma)?;
            let variable = self.expect_token(TokenType::Variable)?.value.unwrap();
            self.expect_token(TokenType::Colon)?;
            let expr = self.parse_precedence1_expr()?;
            iterations.push((variable, expr));
        }
        self.expect_token(TokenType::RSquare)?;
        let element_expr = Box::new(self.parse_precedence1_expr()?);
        Ok(Expression {
            position,
            node: ExpressionType::ArrayLoop {
                range: iterations,
                body: element_expr,
            },
            resolved_type: Rc::new(RefCell::new(None)),
        })
    }

    fn parse_sum_loop_expr(&mut self) -> Result<Expression<'a>, ParserError> {
        let position = self.expect_token(TokenType::Sum)?.position.clone();
        self.expect_token(TokenType::LSquare)?;
        let mut iterations = Vec::new();
        if self.peek_token().token_type != TokenType::RSquare {
            let variable = self.expect_token(TokenType::Variable)?.value.unwrap();
            self.expect_token(TokenType::Colon)?;
            let expr = self.parse_precedence1_expr()?;
            iterations.push((variable, expr));
        }
        while self.peek_token().token_type != TokenType::RSquare {
            self.expect_token(TokenType::Comma)?;
            let variable = self.expect_token(TokenType::Variable)?.value.unwrap();
            self.expect_token(TokenType::Colon)?;
            let expr = self.parse_precedence1_expr()?;
            iterations.push((variable, expr));
        }
        self.expect_token(TokenType::RSquare)?;
        let element_expr = Box::new(self.parse_precedence1_expr()?);
        Ok(Expression {
            position,
            node: ExpressionType::SumLoop {
                range: iterations,
                body: element_expr,
            },
            resolved_type: Rc::new(RefCell::new(None)),
        })
    }

    fn parse_precedence7_expr(&mut self) -> Result<Expression<'a>, ParserError> {
        match self.peek_token().token_type {
            TokenType::IntVal => self.parse_int_expr(),
            TokenType::FloatVal => self.parse_float_expr(),
            TokenType::True => self.parse_true_expr(),
            TokenType::False => self.parse_false_expr(),
            TokenType::Variable => {
                // Peek ahead to decide between struct literal, call, or plain variable.
                match self.peek_next_token().token_type {
                    TokenType::LCurly => self.parse_struct_literal_expr(),
                    TokenType::LParen => self.parse_call_expr(),
                    _ => self.parse_variable_expr(),
                }
            }
            TokenType::LSquare => self.parse_array_literal_expr(),
            TokenType::LParen => self.parse_precedent_expr(),
            TokenType::Void => self.parse_void_expr(),
            _ => Err(ParserError {
                message: format!("{} is not a valid expression", self.peek_token().token_type),
                position: self.peek_token().position.clone(),
            }),
        }
    }

    fn parse_precedence6_expr(&mut self) -> Result<Expression<'a>, ParserError> {
        let mut expr = self.parse_precedence7_expr()?;
        loop {
            match self.peek_token().token_type {
                TokenType::Dot => {
                    expr = self.parse_dot_expr(expr)?;
                }
                TokenType::LSquare => {
                    expr = self.parse_array_index_expr(expr)?;
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_precedence5_expr(&mut self) -> Result<Expression<'a>, ParserError> {
        let token = self.peek_token();
        if let Some(value) = token.value {
            if value == "-" {
                let position = token.position.clone();
                self.expect_token(TokenType::Op)?;
                let right = self.parse_precedence5_expr()?;
                return Ok(Expression {
                    position,
                    node: ExpressionType::Unop {
                        operator: Unop::Negative,
                        expression: Box::new(right),
                    },
                    resolved_type: Rc::new(RefCell::new(None)),
                });
            } else if value == "!" {
                let position = token.position.clone();
                self.expect_token(TokenType::Op)?;
                let right = self.parse_precedence5_expr()?;
                return Ok(Expression {
                    position,
                    node: ExpressionType::Unop {
                        operator: Unop::Not,
                        expression: Box::new(right),
                    },
                    resolved_type: Rc::new(RefCell::new(None)),
                });
            }
        } else if self.peek_token().token_type == TokenType::Array {
            return self.parse_array_loop_expr();
        } else if self.peek_token().token_type == TokenType::Sum {
            return self.parse_sum_loop_expr();
        } else if self.peek_token().token_type == TokenType::If {
            return self.parse_if_expr();
        }
        self.parse_precedence6_expr()
    }

    fn parse_precedence4_expr(&mut self) -> Result<Expression<'a>, ParserError> {
        let mut left = self.parse_precedence5_expr()?;
        while self.peek_token().token_type == TokenType::Op {
            let op_str = self.peek_token().value.unwrap();
            if op_str == "*" || op_str == "/" || op_str == "%" {
                self.expect_token(TokenType::Op)?;
                let right = self.parse_precedence5_expr()?;
                let operator = Binop::from_str(op_str);
                left = Expression {
                    position: left.position.clone(),
                    node: ExpressionType::Binop {
                        operator,
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    resolved_type: Rc::new(RefCell::new(None)),
                };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_precedence3_expr(&mut self) -> Result<Expression<'a>, ParserError> {
        let mut left = self.parse_precedence4_expr()?;
        while self.peek_token().token_type == TokenType::Op {
            let op_str = self.peek_token().value.unwrap();
            if op_str == "+" || op_str == "-" {
                self.expect_token(TokenType::Op)?;
                let right = self.parse_precedence4_expr()?;
                let operator = Binop::from_str(op_str);
                left = Expression {
                    position: left.position.clone(),
                    node: ExpressionType::Binop {
                        operator,
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    resolved_type: Rc::new(RefCell::new(None)),
                };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_precedence2_expr(&mut self) -> Result<Expression<'a>, ParserError> {
        let mut left = self.parse_precedence3_expr()?;
        while self.peek_token().token_type == TokenType::Op {
            let op_str = self.peek_token().value.unwrap();
            if ["<", ">", "<=", ">=", "==", "!="].contains(&op_str) {
                self.expect_token(TokenType::Op)?;
                let right = self.parse_precedence3_expr()?;
                let operator = Binop::from_str(op_str);
                left = Expression {
                    position: left.position.clone(),
                    node: ExpressionType::Binop {
                        operator,
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    resolved_type: Rc::new(RefCell::new(None)),
                };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_precedence1_expr(&mut self) -> Result<Expression<'a>, ParserError> {
        let mut left = self.parse_precedence2_expr()?;
        while self.peek_token().token_type == TokenType::Op {
            let op_str = self.peek_token().value.unwrap();
            if op_str == "&&" || op_str == "||" {
                self.expect_token(TokenType::Op)?;
                let right = self.parse_precedence2_expr()?;
                let operator = Binop::from_str(op_str);
                left = Expression {
                    position: left.position.clone(),
                    node: ExpressionType::Binop {
                        operator,
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    resolved_type: Rc::new(RefCell::new(None)),
                };
            } else {
                break;
            }
        }
        Ok(left)
    }

    // ----------------------------------------------------------------------------------- Statement Parsers ----------------------------------------------------------------------------------------------

    fn parse_statement(&mut self) -> Result<Statement<'a>, ParserError> {
        match self.peek_token().token_type {
            TokenType::Let => self.parse_let_statement(),
            TokenType::Assert => self.parse_assert_statement(),
            TokenType::Return => self.parse_return_statement(),
            _ => Err(ParserError {
                message: format!("Expected statement, got {}", self.peek_token().token_type),
                position: self.peek_token().position.clone(),
            }),
        }
    }

    fn parse_let_statement(&mut self) -> Result<Statement<'a>, ParserError> {
        let position = self.expect_token(TokenType::Let)?.position.clone();
        let variable = self.parse_lvalue()?;
        self.expect_token(TokenType::Equals)?;
        let expression = self.parse_precedence1_expr()?;
        Ok(Statement {
            position,
            node: StatementType::Let {
                variable: Box::new(variable),
                rvalue: Box::new(expression),
            },
        })
    }

    fn parse_assert_statement(&mut self) -> Result<Statement<'a>, ParserError> {
        let position = self.expect_token(TokenType::Assert)?.position.clone();
        let expression = self.parse_precedence1_expr()?;
        self.expect_token(TokenType::Comma)?;
        let message = self.expect_token(TokenType::String)?.value.unwrap();
        Ok(Statement {
            position,
            node: StatementType::Assert {
                condition: Box::new(expression),
                message,
            },
        })
    }

    fn parse_return_statement(&mut self) -> Result<Statement<'a>, ParserError> {
        let position = self.expect_token(TokenType::Return)?.position.clone();
        let return_value = self.parse_precedence1_expr()?;
        Ok(Statement {
            position,
            node: StatementType::Return {
                value: Box::new(return_value),
            },
        })
    }

    // ----------------------------------------------------------------------------------- LValue Parsers ----------------------------------------------------------------------------------------------

    fn parse_lvalue(&mut self) -> Result<LValue<'a>, ParserError> {
        if self.peek_next_token().token_type == TokenType::LSquare {
            // Array LValue branch
            let token = self.expect_token(TokenType::Variable)?;
            let position = token.position.clone();
            let array_name = token.value.unwrap();
            self.expect_token(TokenType::LSquare)?;

            // Empty array.
            if self.peek_token().token_type == TokenType::RSquare {
                self.expect_token(TokenType::RSquare)?;
                return Ok(LValue {
                    position,
                    node: LValueType::Array {
                        name: array_name,
                        indices: Vec::new(),
                    },
                });
            }

            let mut indices = Vec::new();
            // First element.
            let elem_token = self.expect_token(TokenType::Variable)?;
            indices.push(elem_token.value.unwrap());

            // Parse remaining elements.
            while self.peek_token().token_type != TokenType::RSquare {
                self.expect_token(TokenType::Comma)?;
                let elem_token = self.expect_token(TokenType::Variable)?;
                indices.push(elem_token.value.unwrap());
            }

            self.expect_token(TokenType::RSquare)?;
            Ok(LValue {
                position,
                node: LValueType::Array {
                    name: array_name,
                    indices,
                },
            })
        } else {
            // Variable LValue
            let token = self.expect_token(TokenType::Variable)?;
            Ok(LValue {
                position: token.position.clone(),
                node: LValueType::Variable {
                    name: token.value.unwrap(),
                },
            })
        }
    }

    // ----------------------------------------------------------------------------------- Type Parsers ----------------------------------------------------------------------------------------------

    fn parse_type(&mut self) -> Result<Type<'a>, ParserError> {
        let mut typ = self.parse_type_terminal()?;
        while self.peek_token().token_type == TokenType::LSquare {
            typ = self.parse_array_type(typ)?;
        }
        Ok(typ)
    }

    fn parse_type_terminal(&mut self) -> Result<Type<'a>, ParserError> {
        match self.peek_token().token_type {
            TokenType::Int => self.parse_int_type(),
            TokenType::Float => self.parse_float_type(),
            TokenType::Bool => self.parse_bool_type(),
            TokenType::Void => self.parse_void_type(),
            TokenType::Variable => self.parse_struct_type(),
            _ => Err(ParserError {
                message: format!("{} is not a valid type", self.peek_token().token_type),
                position: self.peek_token().position.clone(),
            }),
        }
    }

    fn parse_int_type(&mut self) -> Result<Type<'a>, ParserError> {
        let token = self.expect_token(TokenType::Int)?;
        Ok(Type {
            position: token.position.clone(),
            node: TypeValue::Int,
        })
    }

    fn parse_float_type(&mut self) -> Result<Type<'a>, ParserError> {
        let token = self.expect_token(TokenType::Float)?;
        Ok(Type {
            position: token.position.clone(),
            node: TypeValue::Float,
        })
    }

    fn parse_bool_type(&mut self) -> Result<Type<'a>, ParserError> {
        let token = self.expect_token(TokenType::Bool)?;
        Ok(Type {
            position: token.position.clone(),
            node: TypeValue::Bool,
        })
    }

    fn parse_void_type(&mut self) -> Result<Type<'a>, ParserError> {
        let token = self.expect_token(TokenType::Void)?;
        Ok(Type {
            position: token.position.clone(),
            node: TypeValue::Void,
        })
    }

    fn parse_struct_type(&mut self) -> Result<Type<'a>, ParserError> {
        let token = self.expect_token(TokenType::Variable)?;
        Ok(Type {
            position: token.position.clone(),
            node: TypeValue::Struct {
                name: token.value.unwrap(),
                elements: None,
            },
        })
    }

    fn parse_array_type(&mut self, base: Type<'a>) -> Result<Type<'a>, ParserError> {
        let start_token = self.expect_token(TokenType::LSquare)?;
        let mut rank = 1;
        while self.peek_token().token_type == TokenType::Comma {
            self.expect_token(TokenType::Comma)?;
            rank += 1;
        }
        self.expect_token(TokenType::RSquare)?;
        Ok(Type {
            position: start_token.position.clone(),
            node: TypeValue::Array {
                element_type: Box::new(base),
                rank,
            },
        })
    }
}

pub fn parse<'a>(tokens: Vec<Token<'a>>) -> Result<Vec<Command<'a>>, ParserError> {
    let mut parser = Parser::new(tokens);
    parser.parse()
}
