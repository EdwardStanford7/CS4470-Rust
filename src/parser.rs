use crate::ast::*;
use crate::lexer::*;
use crate::typechecker::TypeEnvironment;
use core::fmt;
use std::cell::Cell;
use std::cell::RefCell;
use std::fmt::Display;
use std::rc::Rc;

pub struct ParseError {
    message: String,
    position: Position,
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

    fn parse(&mut self) -> Result<Vec<Command<'a>>, ParseError> {
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

    fn expect_token(&self, expected: TokenType) -> Result<Position, ParseError> {
        let token = &self.tokens[self.index.get()];
        self.index.replace(self.index.get() + 1);

        if token.token_type == expected {
            Ok(token.position)
        } else {
            Err(ParseError {
                message: format!("Expected token {}, got {}", expected, token.token_type),
                position: token.position,
            })
        }
    }

    fn expect_token_match(
        &self,
        does_match: impl FnOnce(&TokenType<'a>) -> bool,
    ) -> Result<(Position, &'a str), ParseError> {
        let token = &self.tokens[self.index.get()];
        self.index.replace(self.index.get() + 1);

        match &token.token_type {
            TokenType::IntVal(value)
            | TokenType::FloatVal(value)
            | TokenType::Variable(value)
            | TokenType::String(value)
            | TokenType::Op(value) => {
                if does_match(&token.token_type) {
                    return Ok((token.position, value));
                }
            }
            _ => {}
        }

        Err(ParseError {
            message: format!("Expected value token, got {}", token.token_type),
            position: token.position,
        })
    }

    fn peek_token(&self) -> &Token<'a> {
        self.tokens.get(self.index.get()).unwrap()
    }

    fn peek_next_token(&self) -> &Token<'a> {
        self.tokens.get(self.index.get() + 1).unwrap()
    }

    // ----------------------------------------------------------------------------------- Command Parsers ----------------------------------------------------------------------------------------------

    fn parse_command(&mut self) -> Result<Command<'a>, ParseError> {
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
            _ => Err(ParseError {
                message: format!("Expected command, got {}", self.peek_token()),
                position: self.peek_token().position,
            }),
        }
    }

    fn parse_read_command(&mut self) -> Result<Command<'a>, ParseError> {
        let position = self.expect_token(TokenType::Read)?;
        self.expect_token(TokenType::Image)?;
        let (_, source) =
            self.expect_token_match(|token_type| matches!(token_type, TokenType::String(_)))?;
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

    fn parse_write_command(&mut self) -> Result<Command<'a>, ParseError> {
        let position = self.expect_token(TokenType::Write)?;
        self.expect_token(TokenType::Image)?;
        let source = self.parse_precedence1_expr()?;
        self.expect_token(TokenType::To)?;
        let (_, destination) =
            self.expect_token_match(|token_type| matches!(token_type, TokenType::String(_)))?;

        Ok(Command {
            position,
            node: CommandType::Write {
                source: Box::new(source),
                destination,
            },
        })
    }

    fn parse_let_command(&mut self) -> Result<Command<'a>, ParseError> {
        let position = self.expect_token(TokenType::Let)?;
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

    fn parse_assert_command(&mut self) -> Result<Command<'a>, ParseError> {
        let position = self.expect_token(TokenType::Assert)?;
        let expression = self.parse_precedence1_expr()?;
        self.expect_token(TokenType::Comma)?;
        let (_, message) =
            self.expect_token_match(|token_type| matches!(token_type, TokenType::String(_)))?;
        Ok(Command {
            position,
            node: CommandType::Assert {
                condition: Box::new(expression),
                message,
            },
        })
    }

    fn parse_print_command(&mut self) -> Result<Command<'a>, ParseError> {
        let position = self.expect_token(TokenType::Print)?;
        let (_, message) =
            self.expect_token_match(|token_type| matches!(token_type, TokenType::String(_)))?;
        Ok(Command {
            position,
            node: CommandType::Print { message },
        })
    }

    fn parse_show_command(&mut self) -> Result<Command<'a>, ParseError> {
        let position = self.expect_token(TokenType::Show)?;
        let expression = self.parse_precedence1_expr()?;
        Ok(Command {
            position,
            node: CommandType::Show {
                expression: Box::new(expression),
            },
        })
    }

    fn parse_time_command(&mut self) -> Result<Command<'a>, ParseError> {
        let position = self.expect_token(TokenType::Time)?;
        let command = self.parse_command()?;
        Ok(Command {
            position,
            node: CommandType::Time {
                command: Box::new(command),
            },
        })
    }

    fn parse_struct_command(&mut self) -> Result<Command<'a>, ParseError> {
        let start_position = self.expect_token(TokenType::Struct)?;
        let (_, struct_name) =
            self.expect_token_match(|token_type| matches!(token_type, TokenType::Variable(_)))?;
        self.expect_token(TokenType::LCurly)?;
        self.expect_token(TokenType::Newline)?;

        let mut elements = Vec::new();
        while self.peek_token().token_type != TokenType::RCurly {
            let (_, field_name) =
                self.expect_token_match(|token_type| matches!(token_type, TokenType::Variable(_)))?;
            self.expect_token(TokenType::Colon)?;
            let field_type = self.parse_type()?;
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

    fn parse_fn_command(&mut self) -> Result<Command<'a>, ParseError> {
        let start_position = self.expect_token(TokenType::Fn)?;
        let (_, function_name) =
            self.expect_token_match(|token_type| matches!(token_type, TokenType::Variable(_)))?;
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
                local_env: Rc::new(RefCell::new(TypeEnvironment::new(None))),
            },
        })
    }

    // ----------------------------------------------------------------------------------- Expression Parsers ----------------------------------------------------------------------------------------------

    fn parse_int_expr(&mut self) -> Result<Expression<'a>, ParseError> {
        let (position, value_str) =
            self.expect_token_match(|token_type| matches!(token_type, TokenType::IntVal(_)))?;
        let value = value_str.parse::<i64>().map_err(|_| ParseError {
            message: format!("Integer constant {} is too large", value_str),
            position,
        })?;
        Ok(Expression {
            position,
            node: ExpressionType::Int { value },
            resolved_type: RefCell::new(None),
        })
    }

    fn parse_float_expr(&mut self) -> Result<Expression<'a>, ParseError> {
        let (position, value_str) =
            self.expect_token_match(|token_type| matches!(token_type, TokenType::FloatVal(_)))?;
        let value = value_str.parse::<f64>().map_err(|_| ParseError {
            message: format!("Float constant {} is not valid", value_str),
            position,
        })?;
        if value.is_infinite() {
            return Err(ParseError {
                message: format!("Float constant {} is too large", value_str),
                position,
            });
        }
        Ok(Expression {
            position,
            node: ExpressionType::Float { value },
            resolved_type: RefCell::new(None),
        })
    }

    fn parse_true_expr(&mut self) -> Result<Expression<'a>, ParseError> {
        let position = self.expect_token(TokenType::True)?;
        Ok(Expression {
            position,
            node: ExpressionType::True,
            resolved_type: RefCell::new(None),
        })
    }

    fn parse_false_expr(&mut self) -> Result<Expression<'a>, ParseError> {
        let position = self.expect_token(TokenType::False)?;
        Ok(Expression {
            position,
            node: ExpressionType::False,
            resolved_type: RefCell::new(None),
        })
    }

    fn parse_variable_expr(&mut self) -> Result<Expression<'a>, ParseError> {
        let (position, name) =
            self.expect_token_match(|token_type| matches!(token_type, TokenType::Variable(_)))?;
        Ok(Expression {
            position,
            node: ExpressionType::Variable { name },
            resolved_type: RefCell::new(None),
        })
    }

    fn parse_array_literal_expr(&mut self) -> Result<Expression<'a>, ParseError> {
        let position = self.expect_token(TokenType::LSquare)?;
        // Empty array literal.
        if self.peek_token().token_type == TokenType::RSquare {
            self.expect_token(TokenType::RSquare)?;
            return Ok(Expression {
                position,
                node: ExpressionType::ArrayLiteral {
                    elements: Vec::new(),
                },
                resolved_type: RefCell::new(None),
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
            resolved_type: RefCell::new(None),
        })
    }

    fn parse_call_expr(&mut self) -> Result<Expression<'a>, ParseError> {
        let (start_position, func_name) =
            self.expect_token_match(|token_type| matches!(token_type, TokenType::Variable(_)))?;

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
            resolved_type: RefCell::new(None),
        })
    }

    fn parse_struct_literal_expr(&mut self) -> Result<Expression<'a>, ParseError> {
        let (start_position, struct_name) =
            self.expect_token_match(|token_type| matches!(token_type, TokenType::Variable(_)))?;

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
            resolved_type: RefCell::new(None),
        })
    }

    fn parse_dot_expr(&mut self, left: Expression<'a>) -> Result<Expression<'a>, ParseError> {
        let position = self.expect_token(TokenType::Dot)?;
        let (_, field) =
            self.expect_token_match(|token_type| matches!(token_type, TokenType::Variable(_)))?;

        Ok(Expression {
            position,
            node: ExpressionType::Dot {
                struct_variable: Box::new(left),
                field,
            },
            resolved_type: RefCell::new(None),
        })
    }

    fn parse_array_index_expr(
        &mut self,
        array: Expression<'a>,
    ) -> Result<Expression<'a>, ParseError> {
        let position = self.expect_token(TokenType::LSquare)?;
        let mut indices = Vec::new();
        if self.peek_token().token_type == TokenType::RSquare {
            self.expect_token(TokenType::RSquare)?;
            return Ok(Expression {
                position,
                node: ExpressionType::ArrayIndex {
                    array: Box::new(array),
                    indices,
                },
                resolved_type: RefCell::new(None),
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
            resolved_type: RefCell::new(None),
        })
    }

    fn parse_precedent_expr(&mut self) -> Result<Expression<'a>, ParseError> {
        self.expect_token(TokenType::LParen)?;
        let expr = self.parse_precedence1_expr()?;
        self.expect_token(TokenType::RParen)?;
        Ok(expr)
    }

    fn parse_void_expr(&mut self) -> Result<Expression<'a>, ParseError> {
        let position = self.expect_token(TokenType::Void)?;
        Ok(Expression {
            position,
            node: ExpressionType::Void,
            resolved_type: RefCell::new(None),
        })
    }

    fn parse_if_expr(&mut self) -> Result<Expression<'a>, ParseError> {
        let position = self.expect_token(TokenType::If)?;
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
            resolved_type: RefCell::new(None),
        })
    }

    fn parse_array_loop_expr(&mut self) -> Result<Expression<'a>, ParseError> {
        let position = self.expect_token(TokenType::Array)?;
        self.expect_token(TokenType::LSquare)?;
        let mut iterations = Vec::new();
        if self.peek_token().token_type != TokenType::RSquare {
            let (_, variable) =
                self.expect_token_match(|token_type| matches!(token_type, TokenType::Variable(_)))?;

            self.expect_token(TokenType::Colon)?;
            let expr = self.parse_precedence1_expr()?;
            iterations.push((variable, expr));
        }
        while self.peek_token().token_type != TokenType::RSquare {
            self.expect_token(TokenType::Comma)?;
            let (_, variable) =
                self.expect_token_match(|token_type| matches!(token_type, TokenType::Variable(_)))?;
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
            resolved_type: RefCell::new(None),
        })
    }

    fn parse_sum_loop_expr(&mut self) -> Result<Expression<'a>, ParseError> {
        let position = self.expect_token(TokenType::Sum)?;
        self.expect_token(TokenType::LSquare)?;
        let mut iterations = Vec::new();
        if self.peek_token().token_type != TokenType::RSquare {
            let (_, variable) =
                self.expect_token_match(|token_type| matches!(token_type, TokenType::Variable(_)))?;
            self.expect_token(TokenType::Colon)?;
            let expr = self.parse_precedence1_expr()?;
            iterations.push((variable, expr));
        }
        while self.peek_token().token_type != TokenType::RSquare {
            self.expect_token(TokenType::Comma)?;
            let (_, variable) =
                self.expect_token_match(|token_type| matches!(token_type, TokenType::Variable(_)))?;
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
            resolved_type: RefCell::new(None),
        })
    }

    fn parse_precedence7_expr(&mut self) -> Result<Expression<'a>, ParseError> {
        match self.peek_token().token_type {
            TokenType::IntVal(_) => self.parse_int_expr(),
            TokenType::FloatVal(_) => self.parse_float_expr(),
            TokenType::True => self.parse_true_expr(),
            TokenType::False => self.parse_false_expr(),
            TokenType::Variable(_) => {
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
            _ => Err(ParseError {
                message: format!("{} is not a valid expression", self.peek_token().token_type),
                position: self.peek_token().position,
            }),
        }
    }

    fn parse_precedence6_expr(&mut self) -> Result<Expression<'a>, ParseError> {
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

    fn parse_precedence5_expr(&mut self) -> Result<Expression<'a>, ParseError> {
        let token = self.peek_token();
        let position = token.position;

        match token.token_type {
            TokenType::Op(operator) => {
                if operator == "-" || operator == "!" {
                    self.index.replace(self.index.get() + 1);
                    let right = self.parse_precedence5_expr()?;
                    return Ok(Expression {
                        position,
                        node: ExpressionType::Unop {
                            operator: Unop::from_str(operator),
                            expression: Box::new(right),
                        },
                        resolved_type: RefCell::new(None),
                    });
                }
            }
            TokenType::Array => {
                return self.parse_array_loop_expr();
            }
            TokenType::Sum => {
                return self.parse_sum_loop_expr();
            }
            TokenType::If => {
                return self.parse_if_expr();
            }
            _ => {
                // Do nothing
            }
        }

        self.parse_precedence6_expr()
    }

    fn parse_precedence4_expr(&mut self) -> Result<Expression<'a>, ParseError> {
        let mut left = self.parse_precedence5_expr()?;

        while let TokenType::Op(operator) = self.peek_token().token_type {
            if operator == "*" || operator == "/" || operator == "%" {
                self.index.replace(self.index.get() + 1);
                let right = self.parse_precedence5_expr()?;
                left = Expression {
                    position: left.position,
                    node: ExpressionType::Binop {
                        operator: Binop::from_str(operator),
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    resolved_type: RefCell::new(None),
                };
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_precedence3_expr(&mut self) -> Result<Expression<'a>, ParseError> {
        let mut left = self.parse_precedence4_expr()?;

        while let TokenType::Op(operator) = self.peek_token().token_type {
            if operator == "+" || operator == "-" {
                self.index.replace(self.index.get() + 1);
                let right = self.parse_precedence4_expr()?;
                left = Expression {
                    position: left.position,
                    node: ExpressionType::Binop {
                        operator: Binop::from_str(operator),
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    resolved_type: RefCell::new(None),
                };
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_precedence2_expr(&mut self) -> Result<Expression<'a>, ParseError> {
        let mut left = self.parse_precedence3_expr()?;

        while let TokenType::Op(operator) = self.peek_token().token_type {
            if operator == "<"
                || operator == "<="
                || operator == ">"
                || operator == ">="
                || operator == "=="
                || operator == "!="
            {
                self.index.replace(self.index.get() + 1);
                let right = self.parse_precedence3_expr()?;
                left = Expression {
                    position: left.position,
                    node: ExpressionType::Binop {
                        operator: Binop::from_str(operator),
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    resolved_type: RefCell::new(None),
                };
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_precedence1_expr(&mut self) -> Result<Expression<'a>, ParseError> {
        let mut left = self.parse_precedence2_expr()?;

        while let TokenType::Op(operator) = self.peek_token().token_type {
            if operator == "&&" || operator == "||" {
                self.index.replace(self.index.get() + 1);
                let right = self.parse_precedence2_expr()?;
                left = Expression {
                    position: left.position,
                    node: ExpressionType::Binop {
                        operator: Binop::from_str(operator),
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    resolved_type: RefCell::new(None),
                };
            } else {
                break;
            }
        }

        Ok(left)
    }

    // ----------------------------------------------------------------------------------- Statement Parsers ----------------------------------------------------------------------------------------------

    fn parse_statement(&mut self) -> Result<Statement<'a>, ParseError> {
        match self.peek_token().token_type {
            TokenType::Let => self.parse_let_statement(),
            TokenType::Assert => self.parse_assert_statement(),
            TokenType::Return => self.parse_return_statement(),
            _ => Err(ParseError {
                message: format!("Expected statement, got {}", self.peek_token().token_type),
                position: self.peek_token().position,
            }),
        }
    }

    fn parse_let_statement(&mut self) -> Result<Statement<'a>, ParseError> {
        self.expect_token(TokenType::Let)?;
        let variable = self.parse_lvalue()?;
        self.expect_token(TokenType::Equals)?;
        let expression = self.parse_precedence1_expr()?;
        Ok(Statement {
            node: StatementType::Let {
                variable: Box::new(variable),
                rvalue: Box::new(expression),
            },
        })
    }

    fn parse_assert_statement(&mut self) -> Result<Statement<'a>, ParseError> {
        self.expect_token(TokenType::Assert)?;
        let expression = self.parse_precedence1_expr()?;
        self.expect_token(TokenType::Comma)?;
        let (_, message) =
            self.expect_token_match(|token_type| matches!(token_type, TokenType::String(_)))?;
        Ok(Statement {
            node: StatementType::Assert {
                condition: Box::new(expression),
                message,
            },
        })
    }

    fn parse_return_statement(&mut self) -> Result<Statement<'a>, ParseError> {
        self.expect_token(TokenType::Return)?;
        let return_value = self.parse_precedence1_expr()?;
        Ok(Statement {
            node: StatementType::Return {
                value: Box::new(return_value),
            },
        })
    }

    // ----------------------------------------------------------------------------------- LValue Parsers ----------------------------------------------------------------------------------------------

    fn parse_lvalue(&mut self) -> Result<LValue<'a>, ParseError> {
        if self.peek_next_token().token_type == TokenType::LSquare {
            // Array LValue branch
            let (position, name) =
                self.expect_token_match(|token_type| matches!(token_type, TokenType::Variable(_)))?;
            self.expect_token(TokenType::LSquare)?;

            // Empty array.
            if self.peek_token().token_type == TokenType::RSquare {
                self.expect_token(TokenType::RSquare)?;
                return Ok(LValue {
                    position,
                    name,
                    node: LValueType::Array {
                        indices: Vec::new(),
                    },
                });
            }

            let mut indices = Vec::new();
            // First element.
            let (_, elem_name) =
                self.expect_token_match(|token_type| matches!(token_type, TokenType::Variable(_)))?;

            indices.push(elem_name);

            // Parse remaining elements.
            while self.peek_token().token_type != TokenType::RSquare {
                self.expect_token(TokenType::Comma)?;
                // let elem_token = self.expect_token(TokenType::Variable)?;
                let (_, elem_name) = self.expect_token_match(|token_type| {
                    matches!(token_type, TokenType::Variable(_))
                })?;
                indices.push(elem_name);
            }

            self.expect_token(TokenType::RSquare)?;
            Ok(LValue {
                position,
                name,
                node: LValueType::Array { indices },
            })
        } else {
            // Variable LValue
            // let token = self.expect_token(TokenType::Variable)?;
            let (position, name) =
                self.expect_token_match(|token_type| matches!(token_type, TokenType::Variable(_)))?;
            Ok(LValue {
                position,
                name,
                node: LValueType::Variable,
            })
        }
    }

    // ----------------------------------------------------------------------------------- Type Parsers ----------------------------------------------------------------------------------------------

    fn parse_type(&mut self) -> Result<Type<'a>, ParseError> {
        let mut typ = self.parse_type_terminal()?;
        while self.peek_token().token_type == TokenType::LSquare {
            typ = self.parse_array_type(typ)?;
        }
        Ok(typ)
    }

    fn parse_type_terminal(&mut self) -> Result<Type<'a>, ParseError> {
        match self.peek_token().token_type {
            TokenType::Int => self.parse_int_type(),
            TokenType::Float => self.parse_float_type(),
            TokenType::Bool => self.parse_bool_type(),
            TokenType::Void => self.parse_void_type(),
            TokenType::Variable(_) => self.parse_struct_type(),
            _ => Err(ParseError {
                message: format!("{} is not a valid type", self.peek_token().token_type),
                position: self.peek_token().position,
            }),
        }
    }

    fn parse_int_type(&mut self) -> Result<Type<'a>, ParseError> {
        let position = self.expect_token(TokenType::Int)?;
        Ok(Type {
            position,
            node: TypeType::Int,
        })
    }

    fn parse_float_type(&mut self) -> Result<Type<'a>, ParseError> {
        let position = self.expect_token(TokenType::Float)?;
        Ok(Type {
            position,
            node: TypeType::Float,
        })
    }

    fn parse_bool_type(&mut self) -> Result<Type<'a>, ParseError> {
        let position = self.expect_token(TokenType::Bool)?;
        Ok(Type {
            position,
            node: TypeType::Bool,
        })
    }

    fn parse_void_type(&mut self) -> Result<Type<'a>, ParseError> {
        let position = self.expect_token(TokenType::Void)?;
        Ok(Type {
            position,
            node: TypeType::Void,
        })
    }

    fn parse_struct_type(&mut self) -> Result<Type<'a>, ParseError> {
        let (position, name) =
            self.expect_token_match(|token_type| matches!(token_type, TokenType::Variable(_)))?;
        Ok(Type {
            position,
            node: TypeType::Struct {
                name,
                elements: Vec::new(),
            },
        })
    }

    fn parse_array_type(&mut self, base: Type<'a>) -> Result<Type<'a>, ParseError> {
        let position = self.expect_token(TokenType::LSquare)?;
        let mut rank = 1;
        while self.peek_token().token_type == TokenType::Comma {
            self.expect_token(TokenType::Comma)?;
            rank += 1;
        }
        self.expect_token(TokenType::RSquare)?;
        Ok(Type {
            position,
            node: TypeType::Array {
                element_type: Box::new(base),
                rank,
            },
        })
    }
}

pub fn parse(tokens: Vec<Token<'_>>) -> Result<Vec<Command<'_>>, ParseError> {
    let mut parser = Parser::new(tokens);
    parser.parse()
}
