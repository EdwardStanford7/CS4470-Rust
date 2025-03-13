use crate::ast::*;
use std::{cell::RefCell, collections::HashMap, fmt::Display, rc::Rc};

use crate::lexer::Position;

pub struct TypeCheckerError {
    pub message: String,
    pub position: Position,
}

impl Display for TypeCheckerError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Type error: {}:{}: {}",
            self.position.line, self.position.column, self.message
        )
    }
}

struct TypeChecker<'a> {
    commands: &'a Vec<Command<'a>>,
    global_env: HashMap<&'a str, Rc<RefCell<Option<Type<'a>>>>>,
}

impl<'a> TypeChecker<'a> {
    fn new(commands: &'a Vec<Command<'a>>) -> TypeChecker<'a> {
        TypeChecker {
            commands,
            global_env: HashMap::new(),
        }
    }

    fn typecheck(
        &mut self,
    ) -> Result<HashMap<&'a str, Rc<RefCell<Option<Type<'a>>>>>, TypeCheckerError> {
        self.populate_built_ins();

        for command in self.commands {
            self.typecheck_command(command)?;
        }

        Ok(std::mem::take(&mut self.global_env))
    }

    fn add_identifier(
        &mut self,
        name: &'a str,
        typ: Rc<RefCell<Option<Type<'a>>>>,
    ) -> Result<(), ()> {
        if self.global_env.contains_key(name) {
            return Err(());
        }
        self.global_env.insert(name, typ);
        Ok(())
    }

    fn get_identifier(&self, name: &'a str) -> Result<Rc<RefCell<Option<Type<'a>>>>, ()> {
        match self.global_env.get(name) {
            Some(typ) => Ok(typ.clone()),
            None => Err(()),
        }
    }

    fn populate_built_ins(&mut self) {
        let pos = || Position { line: 0, column: 0 };

        // Create basic types
        let float_type = || Type {
            position: pos(),
            node: TypeValue::Float,
        };

        let int_type = || Type {
            position: pos(),
            node: TypeValue::Int,
        };

        // RGBA struct type
        let rgba = Rc::new(RefCell::new(Some(Type {
            position: pos(),
            node: TypeValue::Struct {
                name: "rgba",
                elements: Some(vec![
                    ("r", float_type()),
                    ("g", float_type()),
                    ("b", float_type()),
                    ("a", float_type()),
                ]),
            },
        })));

        // Argnum and args
        let argnum = Rc::new(RefCell::new(Some(int_type())));
        let args = Rc::new(RefCell::new(Some(Type {
            position: pos(),
            node: TypeValue::Array {
                element_type: Box::new(int_type()),
                rank: 1,
            },
        })));

        // Function types
        let one_float_to_float = Rc::new(RefCell::new(Some(Type {
            position: pos(),
            node: TypeValue::Function {
                param_types: vec![float_type()],
                return_type: Box::new(float_type()),
            },
        })));

        let two_floats_to_float = Rc::new(RefCell::new(Some(Type {
            position: pos(),
            node: TypeValue::Function {
                param_types: vec![float_type(), float_type()],
                return_type: Box::new(float_type()),
            },
        })));

        let to_float = Rc::new(RefCell::new(Some(Type {
            position: pos(),
            node: TypeValue::Function {
                param_types: vec![int_type()],
                return_type: Box::new(float_type()),
            },
        })));

        let to_int = Rc::new(RefCell::new(Some(Type {
            position: pos(),
            node: TypeValue::Function {
                param_types: vec![float_type()],
                return_type: Box::new(int_type()),
            },
        })));

        // Add to environment
        self.global_env.insert("rgba", rgba);
        self.global_env.insert("argnum", argnum);
        self.global_env.insert("args", args);

        for name in &[
            "float", "sqrt", "exp", "sin", "cos", "tan", "asin", "acos", "atan", "log",
        ] {
            self.global_env.insert(name, one_float_to_float.clone());
        }

        for name in &["pow", "atan2"] {
            self.global_env.insert(name, two_floats_to_float.clone());
        }

        self.global_env.insert("to_float", to_float);
        self.global_env.insert("to_int", to_int);
    }

    // ----------------------------------------------------------------------------------- Command Typecheckers ----------------------------------------------------------------------------------------------

    fn typecheck_command(&mut self, command: &Command<'a>) -> Result<(), TypeCheckerError> {
        match &command.node {
            CommandType::Read {
                source,
                destination,
            } => self.typecheck_read_command(source, destination),
            CommandType::Write {
                source,
                destination,
            } => todo!(),
            CommandType::Let { variable, rvalue } => todo!(),
            CommandType::Assert { condition, message } => todo!(),
            CommandType::Print { message } => todo!(),
            CommandType::Show { expression } => {
                self.typecheck_expression(expression);
                Ok(())
            }
            CommandType::Time { command } => todo!(),
            CommandType::Function {
                name,
                parameters,
                return_type,
                statements,
                has_return,
                local_scope,
            } => todo!(),
            CommandType::Struct { name, elements } => todo!(),
        }
    }

    fn typecheck_read_command(
        &mut self,
        source: &'a str,
        destination: &LValue<'a>,
    ) -> Result<(), TypeCheckerError> {
        if self
            .add_identifier(source, self.get_identifier("rgba").unwrap())
            .is_err()
        {
            return Err(TypeCheckerError {
                message: format!("Cannot redefine the name {}", source),
                position: destination.position.clone(),
            });
        };

        if let LValueType::Array { name: _, indices } = &destination.node {
            if indices.len() != 2 {
                return Err(TypeCheckerError {
                    message: format!(
                        "Cannot bind rgba[,] to array of dimension  {}",
                        indices.len()
                    ),
                    position: destination.position.clone(),
                });
            }
            for index in indices {
                let _ = self.add_identifier(
                    index,
                    Rc::new(RefCell::new(Some(Type {
                        position: Position { line: 0, column: 0 },
                        node: TypeValue::Int,
                    }))),
                );
            }
        };

        Ok(())
    }

    // ----------------------------------------------------------------------------------- Expression Typecheckers ----------------------------------------------------------------------------------------------
    fn typecheck_expression(
        &mut self,
        expression: &Expression<'a>,
    ) -> Result<Rc<RefCell<Option<Type<'a>>>>, TypeCheckerError> {
        match &expression.node {
            ExpressionType::Int { value: _ } => {
                *expression.resolved_type.borrow_mut() = Some(Type {
                    position: expression.position.clone(),
                    node: TypeValue::Int,
                });
                Ok(expression.resolved_type.clone())
            }
            ExpressionType::Float { value: _ } => {
                *expression.resolved_type.borrow_mut() = Some(Type {
                    position: expression.position.clone(),
                    node: TypeValue::Float,
                });
                Ok(expression.resolved_type.clone())
            }
            ExpressionType::True => {
                *expression.resolved_type.borrow_mut() = Some(Type {
                    position: expression.position.clone(),
                    node: TypeValue::Bool,
                });
                Ok(expression.resolved_type.clone())
            }
            ExpressionType::False => {
                *expression.resolved_type.borrow_mut() = Some(Type {
                    position: expression.position.clone(),
                    node: TypeValue::Bool,
                });
                Ok(expression.resolved_type.clone())
            }
            ExpressionType::Void => {
                *expression.resolved_type.borrow_mut() = Some(Type {
                    position: expression.position.clone(),
                    node: TypeValue::Void,
                });
                Ok(expression.resolved_type.clone())
            }
            ExpressionType::Variable { name } => todo!(),
            ExpressionType::ArrayLiteral { elements } => todo!(),
            ExpressionType::ArrayIndex { array, indices } => todo!(),
            ExpressionType::Dot {
                struct_variable,
                field,
            } => todo!(),
            ExpressionType::Call {
                function,
                arguments,
            } => todo!(),
            ExpressionType::StructLiteral { name, fields } => todo!(),
            ExpressionType::Unop {
                operator,
                expression,
            } => todo!(),
            ExpressionType::Binop {
                operator,
                left,
                right,
            } => todo!(),
            ExpressionType::If {
                condition,
                then_branch,
                else_branch,
            } => todo!(),
            ExpressionType::ArrayLoop { range, body } => todo!(),
            ExpressionType::SumLoop { range, body } => todo!(),
        }
    }
}

pub fn typecheck<'a>(
    commands: &'a Vec<Command<'a>>,
) -> Result<HashMap<&'a str, Rc<RefCell<Option<Type<'a>>>>>, TypeCheckerError> {
    let mut typechecker = TypeChecker::new(commands);
    typechecker.typecheck()
}
