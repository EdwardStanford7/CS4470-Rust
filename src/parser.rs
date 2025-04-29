use crate::ast::*;
use crate::utils::*;
use std::cell::Cell;

struct Parser<'a> {
    tokens: Vec<Token<'a>>,
    index: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: Vec<Token<'a>>) -> Self {
        Self { tokens, index: 0 }
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

    fn expect_token(&mut self, expected: TokenType) -> Result<Position, ParseError> {
        let token = &self.tokens[self.index];
        self.index += 1;

        if token.token_type == expected {
            Ok(token.position)
        } else {
            Err(ParseError::new(
                format!("Expected token {}, got {}", expected, token.token_type),
                token.position,
            ))
        }
    }

    fn expect_token_match(
        &mut self,
        does_match: impl FnOnce(&TokenType<'a>) -> bool,
    ) -> Result<(Position, &'a str), ParseError> {
        let token = &self.tokens[self.index];
        self.index += 1;

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

        Err(ParseError::new(
            format!("Expected value token, got {}", token.token_type),
            token.position,
        ))
    }

    fn peek_token(&self) -> &Token<'a> {
        self.tokens.get(self.index).unwrap()
    }

    fn peek_next_token(&self) -> &Token<'a> {
        self.tokens.get(self.index + 1).unwrap()
    }

    // ----------------------------------------------------------------------------------- Command Parsers ----------------------------------------------------------------------------------------------
    fn parse_command(&mut self) -> Result<Command<'a>, ParseError> {
        match self.peek_token().token_type {
            TokenType::Read => {
                let position = self.expect_token(TokenType::Read)?;
                self.expect_token(TokenType::Image)?;
                let (_, source) = self
                    .expect_token_match(|token_type| matches!(token_type, TokenType::String(_)))?;
                self.expect_token(TokenType::To)?;
                let destination = self.parse_lvalue()?;

                Ok(Command {
                    position,
                    node: Box::new(CommandType::Read {
                        source,
                        destination,
                    }),
                })
            }
            TokenType::Write => {
                let position = self.expect_token(TokenType::Write)?;
                self.expect_token(TokenType::Image)?;
                let source = self.parse_precedence1_expr()?;
                self.expect_token(TokenType::To)?;
                let (_, destination) = self
                    .expect_token_match(|token_type| matches!(token_type, TokenType::String(_)))?;

                Ok(Command {
                    position,
                    node: Box::new(CommandType::Write {
                        source,
                        destination,
                    }),
                })
            }
            TokenType::Let => {
                let position = self.expect_token(TokenType::Let)?;
                let variable = self.parse_lvalue()?;
                self.expect_token(TokenType::Equals)?;
                let rvalue = self.parse_precedence1_expr()?;

                Ok(Command {
                    position,
                    node: Box::new(CommandType::Let { variable, rvalue }),
                })
            }
            TokenType::Assert => {
                let position = self.expect_token(TokenType::Assert)?;
                let condition = self.parse_precedence1_expr()?;
                self.expect_token(TokenType::Comma)?;
                let (_, message) = self
                    .expect_token_match(|token_type| matches!(token_type, TokenType::String(_)))?;
                Ok(Command {
                    position,
                    node: Box::new(CommandType::Assert { condition, message }),
                })
            }
            TokenType::Print => {
                let position = self.expect_token(TokenType::Print)?;
                let (_, message) = self
                    .expect_token_match(|token_type| matches!(token_type, TokenType::String(_)))?;
                Ok(Command {
                    position,
                    node: Box::new(CommandType::Print { message }),
                })
            }
            TokenType::Show => {
                let position = self.expect_token(TokenType::Show)?;
                let expression = self.parse_precedence1_expr()?;
                Ok(Command {
                    position,
                    node: Box::new(CommandType::Show { expression }),
                })
            }
            TokenType::Time => {
                let position = self.expect_token(TokenType::Time)?;
                let command = self.parse_command()?;
                Ok(Command {
                    position,
                    node: Box::new(CommandType::Time { command }),
                })
            }
            TokenType::Struct => {
                let start_position = self.expect_token(TokenType::Struct)?;
                let (_, name) = self.expect_token_match(|token_type| {
                    matches!(token_type, TokenType::Variable(_))
                })?;
                self.expect_token(TokenType::LCurly)?;
                self.expect_token(TokenType::Newline)?;

                let mut elements = Vec::new();
                while self.peek_token().token_type != TokenType::RCurly {
                    let (_, field_name) = self.expect_token_match(|token_type| {
                        matches!(token_type, TokenType::Variable(_))
                    })?;
                    self.expect_token(TokenType::Colon)?;
                    let field_type = self.parse_type()?;
                    self.expect_token(TokenType::Newline)?;
                    elements.push((field_name, field_type));
                }
                self.expect_token(TokenType::RCurly)?;
                Ok(Command {
                    position: start_position,
                    node: Box::new(CommandType::Struct { name, elements }),
                })
            }
            TokenType::Fn => {
                let start_position = self.expect_token(TokenType::Fn)?;
                let (_, name) = self.expect_token_match(|token_type| {
                    matches!(token_type, TokenType::Variable(_))
                })?;
                self.expect_token(TokenType::LParen)?;

                let mut parameters = Vec::new();
                if self.peek_token().token_type != TokenType::RParen {
                    let param_lvalue = self.parse_lvalue()?;
                    self.expect_token(TokenType::Colon)?;
                    let param_type = self.parse_type()?;
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
                let return_type = self.parse_type()?;
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
                    node: Box::new(CommandType::Function {
                        name,
                        parameters,
                        return_type,
                        statements,
                        has_return: Cell::new(false),
                    }),
                })
            }
            _ => Err(ParseError::new(
                format!("Expected command, got {}", self.peek_token()),
                self.peek_token().position,
            )),
        }
    }

    // ----------------------------------------------------------------------------------- Expression Parsers ----------------------------------------------------------------------------------------------
    fn parse_comma_separated_exprs(
        &mut self,
        end_token: TokenType,
    ) -> Result<Vec<Expression<'a>>, ParseError> {
        let mut elements = Vec::new();
        // First element.
        elements.push(self.parse_precedence1_expr()?);
        while self.peek_token().token_type != end_token {
            self.expect_token(TokenType::Comma)?;
            elements.push(self.parse_precedence1_expr()?);
        }
        Ok(elements)
    }

    fn parse_dot_expr(
        &mut self,
        struct_variable: Expression<'a>,
    ) -> Result<Expression<'a>, ParseError> {
        let position = self.expect_token(TokenType::Dot)?;
        let (_, field) =
            self.expect_token_match(|token_type| matches!(token_type, TokenType::Variable(_)))?;

        Ok(Expression {
            position,
            node: Box::new(ExpressionType::Dot {
                struct_variable,
                field,
            }),
            resolved_type: Type::Unresolved,
        })
    }

    fn parse_array_index_expr(
        &mut self,
        array: Expression<'a>,
    ) -> Result<Expression<'a>, ParseError> {
        let position = self.expect_token(TokenType::LSquare)?;

        let indices = if self.peek_token().token_type == TokenType::RSquare {
            Vec::new()
        } else {
            self.parse_comma_separated_exprs(TokenType::RSquare)?
        };

        self.expect_token(TokenType::RSquare)?;

        Ok(Expression {
            position,
            node: Box::new(ExpressionType::ArrayIndex { array, indices }),
            resolved_type: Type::Unresolved,
        })
    }

    fn parse_range_variable(&mut self) -> Result<(&'a str, Expression<'a>), ParseError> {
        let (_, variable) =
            self.expect_token_match(|token_type| matches!(token_type, TokenType::Variable(_)))?;
        self.expect_token(TokenType::Colon)?;
        let expr = self.parse_precedence1_expr()?;
        Ok((variable, expr))
    }

    fn parse_loop_expr(&mut self, loop_type: TokenType) -> Result<Expression<'a>, ParseError> {
        let position = self.expect_token(loop_type.clone())?;
        self.expect_token(TokenType::LSquare)?;
        let mut range = Vec::new();

        // Parse variable ranges
        if self.peek_token().token_type != TokenType::RSquare {
            range.push(self.parse_range_variable()?);

            while self.peek_token().token_type != TokenType::RSquare {
                self.expect_token(TokenType::Comma)?;
                range.push(self.parse_range_variable()?);
            }
        }

        self.expect_token(TokenType::RSquare)?;
        let body = self.parse_precedence1_expr()?;

        // Create the appropriate expression type
        let node = match loop_type {
            TokenType::Array => Box::new(ExpressionType::ArrayLoop { range, body }),
            TokenType::Sum => Box::new(ExpressionType::SumLoop { range, body }),
            _ => unreachable!(), // This should never happen given the function's intended use
        };

        Ok(Expression {
            position,
            node,
            resolved_type: Type::Unresolved,
        })
    }

    fn parse_if_expr(&mut self) -> Result<Expression<'a>, ParseError> {
        let position = self.expect_token(TokenType::If)?;
        let condition = self.parse_precedence1_expr()?;
        self.expect_token(TokenType::Then)?;
        let then_branch = self.parse_precedence1_expr()?;
        self.expect_token(TokenType::Else)?;
        let else_branch = self.parse_precedence1_expr()?;
        Ok(Expression {
            position,
            node: Box::new(ExpressionType::If {
                condition,
                then_branch,
                else_branch,
            }),
            resolved_type: Type::Unresolved,
        })
    }

    fn parse_precedence7_expr(&mut self) -> Result<Expression<'a>, ParseError> {
        match self.peek_token().token_type {
            TokenType::IntVal(_) => {
                // Inline parse_int_expr
                let (position, value_str) = self
                    .expect_token_match(|token_type| matches!(token_type, TokenType::IntVal(_)))?;
                let value = value_str.parse::<i64>().map_err(|_| {
                    ParseError::new(
                        format!("Integer constant {} is too large", value_str),
                        position,
                    )
                })?;
                Ok(Expression {
                    position,
                    node: Box::new(ExpressionType::Int { value }),
                    resolved_type: Type::Unresolved,
                })
            }
            TokenType::FloatVal(_) => {
                // Inline parse_float_expr
                let (position, value_str) = self.expect_token_match(|token_type| {
                    matches!(token_type, TokenType::FloatVal(_))
                })?;
                let value = value_str.parse::<f64>().map_err(|_| {
                    ParseError::new(
                        format!("Float constant {} is not valid", value_str),
                        position,
                    )
                })?;
                if value.is_infinite() {
                    return Err(ParseError::new(
                        format!("Float constant {} is too large", value_str),
                        position,
                    ));
                }
                Ok(Expression {
                    position,
                    node: Box::new(ExpressionType::Float { value }),
                    resolved_type: Type::Unresolved,
                })
            }
            TokenType::True => {
                // Inline parse_true_expr
                let position = self.expect_token(TokenType::True)?;
                Ok(Expression {
                    position,
                    node: Box::new(ExpressionType::True),
                    resolved_type: Type::Unresolved,
                })
            }
            TokenType::False => {
                // Inline parse_false_expr
                let position = self.expect_token(TokenType::False)?;
                Ok(Expression {
                    position,
                    node: Box::new(ExpressionType::False),
                    resolved_type: Type::Unresolved,
                })
            }
            TokenType::Variable(_) => {
                // Peek ahead to decide between struct literal, call, or plain variable.
                match self.peek_next_token().token_type {
                    TokenType::LCurly => {
                        // Inline parse_struct_literal_expr
                        let (start_position, name) = self.expect_token_match(|token_type| {
                            matches!(token_type, TokenType::Variable(_))
                        })?;

                        self.expect_token(TokenType::LCurly)?;

                        let fields = if self.peek_token().token_type != TokenType::RCurly {
                            self.parse_comma_separated_exprs(TokenType::RCurly)?
                        } else {
                            Vec::new()
                        };

                        self.expect_token(TokenType::RCurly)?;

                        Ok(Expression {
                            position: start_position,
                            node: Box::new(ExpressionType::StructLiteral { name, fields }),
                            resolved_type: Type::Unresolved,
                        })
                    }
                    TokenType::LParen => {
                        // Inline parse_call_expr
                        let (start_position, function) = self.expect_token_match(|token_type| {
                            matches!(token_type, TokenType::Variable(_))
                        })?;

                        self.expect_token(TokenType::LParen)?;

                        let arguments = if self.peek_token().token_type != TokenType::RParen {
                            self.parse_comma_separated_exprs(TokenType::RParen)?
                        } else {
                            Vec::new()
                        };

                        self.expect_token(TokenType::RParen)?;

                        Ok(Expression {
                            position: start_position,
                            node: Box::new(ExpressionType::Call {
                                function,
                                arguments,
                            }),
                            resolved_type: Type::Unresolved,
                        })
                    }
                    _ => {
                        // Inline parse_variable_expr
                        let (position, name) = self.expect_token_match(|token_type| {
                            matches!(token_type, TokenType::Variable(_))
                        })?;
                        Ok(Expression {
                            position,
                            node: Box::new(ExpressionType::Variable { name }),
                            resolved_type: Type::Unresolved,
                        })
                    }
                }
            }
            TokenType::LSquare => {
                // Inline parse_array_literal_expr
                let position = self.expect_token(TokenType::LSquare)?;
                // Empty array literal.
                if self.peek_token().token_type == TokenType::RSquare {
                    self.expect_token(TokenType::RSquare)?;
                    return Ok(Expression {
                        position,
                        node: Box::new(ExpressionType::ArrayLiteral {
                            elements: Vec::new(),
                        }),
                        resolved_type: Type::Unresolved,
                    });
                }

                let elements = self.parse_comma_separated_exprs(TokenType::RSquare)?;
                self.expect_token(TokenType::RSquare)?;

                Ok(Expression {
                    position,
                    node: Box::new(ExpressionType::ArrayLiteral { elements }),
                    resolved_type: Type::Unresolved,
                })
            }
            TokenType::LParen => {
                // Inline parse_precedent_expr
                self.expect_token(TokenType::LParen)?;
                let expr = self.parse_precedence1_expr()?;
                self.expect_token(TokenType::RParen)?;
                Ok(expr)
            }
            TokenType::Void => {
                // Inline parse_void_expr
                let position = self.expect_token(TokenType::Void)?;
                Ok(Expression {
                    position,
                    node: Box::new(ExpressionType::Void),
                    resolved_type: Type::Unresolved,
                })
            }
            _ => Err(ParseError::new(
                format!("{} is not a valid expression", self.peek_token().token_type),
                self.peek_token().position,
            )),
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
                    self.index += 1;
                    let expression = self.parse_precedence5_expr()?;
                    return Ok(Expression {
                        position,
                        node: Box::new(ExpressionType::Unop {
                            operator,
                            expression,
                        }),
                        resolved_type: Type::Unresolved,
                    });
                }
            }
            TokenType::Array => {
                // Directly call parse_loop_expr instead of using parse_array_loop_expr
                return self.parse_loop_expr(TokenType::Array);
            }
            TokenType::Sum => {
                // Directly call parse_loop_expr instead of using parse_sum_loop_expr
                return self.parse_loop_expr(TokenType::Sum);
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

    fn parse_binary_expr(
        &mut self,
        parse_higher_precedence: fn(&mut Self) -> Result<Expression<'a>, ParseError>,
        operators: &[&str],
    ) -> Result<Expression<'a>, ParseError> {
        let mut left = parse_higher_precedence(self)?;

        loop {
            // Check if the next token is an operator we're looking for
            let is_target_op = matches!(&self.peek_token().token_type, TokenType::Op(op) if operators.contains(op));

            if is_target_op {
                // Use expect_token_match to safely advance the token and get the operator
                let (_, op) = self.expect_token_match(|token_type| {
                    if let TokenType::Op(op) = token_type {
                        operators.contains(op)
                    } else {
                        false
                    }
                })?;

                let right = parse_higher_precedence(self)?;
                left = Expression {
                    position: left.position,
                    node: Box::new(ExpressionType::Binop {
                        operator: op,
                        left,
                        right,
                    }),
                    resolved_type: Type::Unresolved,
                };
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_precedence4_expr(&mut self) -> Result<Expression<'a>, ParseError> {
        self.parse_binary_expr(Self::parse_precedence5_expr, &["*", "/", "%"])
    }

    fn parse_precedence3_expr(&mut self) -> Result<Expression<'a>, ParseError> {
        self.parse_binary_expr(Self::parse_precedence4_expr, &["+", "-"])
    }

    fn parse_precedence2_expr(&mut self) -> Result<Expression<'a>, ParseError> {
        self.parse_binary_expr(
            Self::parse_precedence3_expr,
            &["<", "<=", ">", ">=", "==", "!="],
        )
    }

    fn parse_precedence1_expr(&mut self) -> Result<Expression<'a>, ParseError> {
        self.parse_binary_expr(Self::parse_precedence2_expr, &["&&", "||"])
    }

    // ----------------------------------------------------------------------------------- Statement Parsers ----------------------------------------------------------------------------------------------

    fn parse_statement(&mut self) -> Result<Statement<'a>, ParseError> {
        match self.peek_token().token_type {
            TokenType::Let => self.parse_let_statement(),
            TokenType::Assert => self.parse_assert_statement(),
            TokenType::Return => self.parse_return_statement(),
            _ => Err(ParseError::new(
                format!("Expected statement, got {}", self.peek_token().token_type),
                self.peek_token().position,
            )),
        }
    }

    fn parse_let_statement(&mut self) -> Result<Statement<'a>, ParseError> {
        self.expect_token(TokenType::Let)?;
        let variable = self.parse_lvalue()?;
        self.expect_token(TokenType::Equals)?;
        let rvalue = self.parse_precedence1_expr()?;
        Ok(Statement {
            node: StatementType::Let { variable, rvalue },
        })
    }

    fn parse_assert_statement(&mut self) -> Result<Statement<'a>, ParseError> {
        self.expect_token(TokenType::Assert)?;
        let condition = self.parse_precedence1_expr()?;
        self.expect_token(TokenType::Comma)?;
        let (_, message) =
            self.expect_token_match(|token_type| matches!(token_type, TokenType::String(_)))?;
        Ok(Statement {
            node: StatementType::Assert { condition, message },
        })
    }

    fn parse_return_statement(&mut self) -> Result<Statement<'a>, ParseError> {
        self.expect_token(TokenType::Return)?;
        let value = self.parse_precedence1_expr()?;
        Ok(Statement {
            node: StatementType::Return { value },
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
            _ => Err(ParseError::new(
                format!("{} is not a valid type", self.peek_token().token_type),
                self.peek_token().position,
            )),
        }
    }

    fn parse_int_type(&mut self) -> Result<Type<'a>, ParseError> {
        self.expect_token(TokenType::Int)?;
        Ok(Type::Int{ value: None })
    }

    fn parse_float_type(&mut self) -> Result<Type<'a>, ParseError> {
        self.expect_token(TokenType::Float)?;
        Ok(Type::Float)
    }

    fn parse_bool_type(&mut self) -> Result<Type<'a>, ParseError> {
        self.expect_token(TokenType::Bool)?;
        Ok(Type::Bool{ value: None})
    }

    fn parse_void_type(&mut self) -> Result<Type<'a>, ParseError> {
        self.expect_token(TokenType::Void)?;
        Ok(Type::Void)
    }

    fn parse_struct_type(&mut self) -> Result<Type<'a>, ParseError> {
        let (_, name) =
            self.expect_token_match(|token_type| matches!(token_type, TokenType::Variable(_)))?;
        Ok(Type::Struct {
            name,
            elements: Vec::new(),
        })
    }

    fn parse_array_type(&mut self, element_type: Type<'a>) -> Result<Type<'a>, ParseError> {
        self.expect_token(TokenType::LSquare)?;
        let mut rank = 1;
        while self.peek_token().token_type == TokenType::Comma {
            self.expect_token(TokenType::Comma)?;
            rank += 1;
        }
        self.expect_token(TokenType::RSquare)?;
        Ok(Type::Array {
            element_type: Box::new(element_type),
            rank,
        })
    }
}

pub fn parse(tokens: Vec<Token<'_>>) -> Result<Vec<Command<'_>>, ParseError> {
    let mut parser = Parser::new(tokens);
    parser.parse()
}
