use crate::ast::*;
use crate::lexer::Position;
use std::{
    cell::{Cell, RefCell},
    collections::{hash_map::Entry, HashMap},
    fmt::Display,
    rc::Rc,
};

#[derive(Debug)]
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

fn pos() -> Position {
    Position { line: 0, column: 0 }
}

pub struct TypeEnvironment<'a> {
    local_environment: HashMap<&'a str, TypeValue<'a>>,
    parent: Option<Rc<RefCell<TypeEnvironment<'a>>>>,
}

impl<'a> TypeEnvironment<'a> {
    pub fn new(parent: Option<Rc<RefCell<TypeEnvironment<'a>>>>) -> TypeEnvironment<'a> {
        TypeEnvironment {
            local_environment: HashMap::new(),
            parent,
        }
    }

    pub fn add_identifier(
        &mut self,
        name: &'a str,
        typ: TypeValue<'a>,
    ) -> Result<(), TypeCheckerError> {
        match self.local_environment.entry(name) {
            Entry::Occupied(_) => {
                return Err(TypeCheckerError {
                    message: format!("Cannot redefine the name {}", name),
                    position: typ.position.clone(),
                });
            }
            Entry::Vacant(entry) => {
                entry.insert(typ);
            }
        }
        Ok(())
    }

    pub fn get_identifier(
        &self,
        position: Position,
        name: &'a str,
    ) -> Result<TypeValue<'a>, TypeCheckerError> {
        match self.local_environment.get(name) {
            Some(typ) => Ok(typ.clone()),
            None => match &self.parent {
                Some(parent) => parent.borrow().get_identifier(position, name),
                None => Err(TypeCheckerError {
                    message: format!("Identifier {} is undefined", name),
                    position,
                }),
            },
        }
    }
}

struct TypeChecker<'a> {
    commands: &'a Vec<Command<'a>>,
}

impl<'a> TypeChecker<'a> {
    fn new(commands: &'a Vec<Command<'a>>) -> TypeChecker<'a> {
        TypeChecker { commands }
    }

    fn typecheck(&mut self) -> Result<Rc<RefCell<TypeEnvironment<'a>>>, TypeCheckerError> {
        let global_env = Rc::new(RefCell::new(TypeEnvironment::new(None)));

        self.populate_built_ins(&global_env);

        for command in self.commands {
            self.typecheck_command(command, &global_env)?;
        }

        Ok(global_env)
    }

    fn populate_built_ins(&mut self, global_env: &Rc<RefCell<TypeEnvironment<'a>>>) {
        // Create basic types
        let float_type = || TypeValue {
            position: pos(),
            node: TypeValueType::Float,
        };

        let int_type = || TypeValue {
            position: pos(),
            node: TypeValueType::Int,
        };

        // RGBA struct type
        let rgba = TypeValue {
            position: pos(),
            node: TypeValueType::Struct {
                name: "rgba",
                elements: vec![
                    ("r", float_type()),
                    ("g", float_type()),
                    ("b", float_type()),
                    ("a", float_type()),
                ],
            },
        };

        // Argnum and args
        let argnum = int_type();
        let args = TypeValue {
            position: pos(),
            node: TypeValueType::Array {
                element_type: Box::new(int_type()),
                rank: 1,
            },
        };

        // Function types
        let one_float_to_float = TypeValue {
            position: pos(),
            node: TypeValueType::Function {
                param_types: vec![float_type()],
                return_type: Box::new(float_type()),
            },
        };

        let two_floats_to_float = TypeValue {
            position: pos(),
            node: TypeValueType::Function {
                param_types: vec![float_type(), float_type()],
                return_type: Box::new(float_type()),
            },
        };

        let to_float = TypeValue {
            position: pos(),
            node: TypeValueType::Function {
                param_types: vec![int_type()],
                return_type: Box::new(float_type()),
            },
        };

        let to_int = TypeValue {
            position: pos(),
            node: TypeValueType::Function {
                param_types: vec![float_type()],
                return_type: Box::new(int_type()),
            },
        };

        // Add to environment
        let _ = global_env.borrow_mut().add_identifier("rgba", rgba);
        let _ = global_env.borrow_mut().add_identifier("argnum", argnum);
        let _ = global_env.borrow_mut().add_identifier("args", args);

        for name in &[
            "float", "sqrt", "exp", "sin", "cos", "tan", "asin", "acos", "atan", "log",
        ] {
            let _ = global_env
                .borrow_mut()
                .add_identifier(name, one_float_to_float.clone());
        }

        for name in &["pow", "atan2"] {
            let _ = global_env
                .borrow_mut()
                .add_identifier(name, two_floats_to_float.clone());
        }

        let _ = global_env.borrow_mut().add_identifier("to_float", to_float);
        let _ = global_env.borrow_mut().add_identifier("to_int", to_int);
    }

    fn bind_lvalue(
        &mut self,
        lvalue: &LValue<'a>,
        typ: TypeValue<'a>,
        environment: &Rc<RefCell<TypeEnvironment<'a>>>,
    ) -> Result<(), TypeCheckerError> {
        match &lvalue.node {
            LValueType::Array { indices } => {
                // Add the identifier with the type
                environment
                    .borrow_mut()
                    .add_identifier(lvalue.name, typ.clone())?;

                // Check if the type is an array with matching rank
                if let TypeValueType::Array {
                    element_type: _,
                    rank,
                } = &typ.node
                {
                    if *rank != indices.len() {
                        return Err(TypeCheckerError {
                            message: format!(
                                "Rank of array lvalue ({}) does not match right hand side ({})",
                                indices.len(),
                                rank
                            ),
                            position: lvalue.position.clone(),
                        });
                    }
                } else {
                    return Err(TypeCheckerError {
                        message: "Incorrect type for array lvalue".to_string(),
                        position: lvalue.position.clone(),
                    });
                }

                // Bind index variables as integers
                for &index in indices {
                    environment.borrow_mut().add_identifier(
                        index,
                        TypeValue {
                            position: pos(),
                            node: TypeValueType::Int,
                        },
                    )?;
                }

                Ok(())
            }
            LValueType::Variable => environment
                .borrow_mut()
                .add_identifier(lvalue.name, typ.clone()),
        }
    }

    fn typecheck_type(
        typ: &Type<'a>,
        env: &Rc<RefCell<TypeEnvironment<'a>>>,
    ) -> Result<TypeValue<'a>, TypeCheckerError> {
        match &typ.node {
            TypeType::Int => Ok(TypeValue {
                position: typ.position.clone(),
                node: TypeValueType::Int,
            }),
            TypeType::Float => Ok(TypeValue {
                position: typ.position.clone(),
                node: TypeValueType::Float,
            }),
            TypeType::Bool => Ok(TypeValue {
                position: typ.position.clone(),
                node: TypeValueType::Bool,
            }),
            TypeType::Void => Ok(TypeValue {
                position: typ.position.clone(),
                node: TypeValueType::Void,
            }),
            TypeType::Array { element_type, rank } => {
                let type_value = Self::typecheck_type(element_type, env)?;
                Ok(TypeValue {
                    position: typ.position.clone(),
                    node: TypeValueType::Array {
                        element_type: Box::new(type_value),
                        rank: *rank,
                    },
                })
            }
            TypeType::Struct { name, elements: _ } => {
                match env.borrow().get_identifier(typ.position.clone(), name) {
                    Ok(struct_type) => Ok(struct_type.clone()),
                    Err(err) => Err(err),
                }
            }
        }
    }

    fn types_equal(left: &TypeValue<'a>, right: &TypeValue<'a>) -> bool {
        match (&left.node, &right.node) {
            (TypeValueType::Int, TypeValueType::Int) => true,
            (TypeValueType::Float, TypeValueType::Float) => true,
            (TypeValueType::Bool, TypeValueType::Bool) => true,
            (TypeValueType::Void, TypeValueType::Void) => true,
            (
                TypeValueType::Array {
                    element_type: left_element,
                    rank: left_rank,
                },
                TypeValueType::Array {
                    element_type: right_element,
                    rank: right_rank,
                },
            ) => left_rank == right_rank && Self::types_equal(left_element, right_element),
            (
                TypeValueType::Struct {
                    name: left_name, ..
                },
                TypeValueType::Struct {
                    name: right_name, ..
                },
            ) => left_name == right_name,
            _ => false,
        }
    }

    // ----------------------------------------------------------------------------------- Command Typecheckers ----------------------------------------------------------------------------------------------

    fn typecheck_command(
        &mut self,
        command: &Command<'a>,
        global_env: &Rc<RefCell<TypeEnvironment<'a>>>,
    ) -> Result<(), TypeCheckerError> {
        match &command.node {
            CommandType::Read {
                source: _,
                destination,
            } => self.typecheck_read_command(command.position.clone(), destination, global_env),
            CommandType::Write {
                source,
                destination: _,
            } => self.typecheck_write_command(command.position.clone(), source, global_env),
            CommandType::Let { variable, rvalue } => {
                self.typecheck_let_command(variable, rvalue, global_env)
            }
            CommandType::Assert {
                condition,
                message: _,
            } => self.typecheck_assert_command(condition, global_env),
            CommandType::Print { message: _ } => Ok(()),
            CommandType::Show { expression } => {
                self.typecheck_expression(expression, global_env)?;
                Ok(())
            }
            CommandType::Time { command } => self.typecheck_command(command, global_env),
            CommandType::Struct { name, elements } => {
                self.typecheck_struct_command(command.position.clone(), name, elements, global_env)
            }
            CommandType::Function {
                name,
                parameters,
                return_type,
                statements,
                has_return,
                local_env,
            } => self.typecheck_function_command(
                command.position.clone(),
                name,
                parameters,
                return_type,
                statements,
                has_return,
                local_env,
                global_env,
            ),
        }
    }

    fn typecheck_read_command(
        &mut self,
        position: Position,
        destination: &LValue<'a>,
        global_env: &Rc<RefCell<TypeEnvironment<'a>>>,
    ) -> Result<(), TypeCheckerError> {
        let rgba = global_env
            .borrow()
            .get_identifier(position.clone(), "rgba")?;
        let rgba_array = TypeValue {
            position: pos(),
            node: TypeValueType::Array {
                element_type: Box::new(rgba.clone()),
                rank: 2,
            },
        };

        self.bind_lvalue(destination, rgba_array, global_env)?;

        if let LValueType::Array { indices } = &destination.node {
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
                let _ = global_env.borrow_mut().add_identifier(
                    index,
                    TypeValue {
                        position: pos(),
                        node: TypeValueType::Int,
                    },
                );
            }
        };

        Ok(())
    }

    fn typecheck_write_command(
        &mut self,
        position: Position,
        source: &Expression<'a>,
        global_env: &Rc<RefCell<TypeEnvironment<'a>>>,
    ) -> Result<(), TypeCheckerError> {
        let image_type = self.typecheck_expression(source, global_env)?;

        let is_rgba_array = matches!(
            &image_type.borrow().as_ref().unwrap().node,
            TypeValueType::Array {
                element_type,
                rank: 2,
            } if matches!(element_type.node, TypeValueType::Struct { name, .. } if name == "rgba")
        );

        if !is_rgba_array {
            Err(TypeCheckerError {
                message: "Expression for write needs to be rgba[,]".to_string(),
                position,
            })
        } else {
            Ok(())
        }
    }

    fn typecheck_assert_command(
        &mut self,
        condition: &Expression<'a>,
        global_env: &Rc<RefCell<TypeEnvironment<'a>>>,
    ) -> Result<(), TypeCheckerError> {
        let condition_type = self.typecheck_expression(condition, global_env)?;
        if !matches!(
            condition_type.borrow().as_ref().unwrap().node,
            TypeValueType::Bool
        ) {
            return Err(TypeCheckerError {
                message: "Asserted expression must be boolean".to_string(),
                position: condition.position.clone(),
            });
        }
        Ok(())
    }

    fn typecheck_let_command(
        &mut self,
        variable: &LValue<'a>,
        rvalue: &Expression<'a>,
        global_env: &Rc<RefCell<TypeEnvironment<'a>>>,
    ) -> Result<(), TypeCheckerError> {
        let rtype = self.typecheck_expression(rvalue, global_env)?;
        let cloned_rtype = rtype.borrow().clone().unwrap();
        self.bind_lvalue(variable, cloned_rtype, global_env)
    }

    fn typecheck_struct_command(
        &mut self,
        position: Position,
        name: &'a str,
        elements: &Vec<(&'a str, Type<'a>)>,
        global_env: &Rc<RefCell<TypeEnvironment<'a>>>,
    ) -> Result<(), TypeCheckerError> {
        let mut resolved_elements = Vec::new();

        for (element_name, element_type) in elements {
            // Check if duplicate name exists inside struct
            if resolved_elements
                .iter()
                .any(|(name, _)| name == element_name)
            {
                return Err(TypeCheckerError {
                    message: format!("Duplicate field name {} in struct", element_name),
                    position: position.clone(),
                });
            }

            let type_value = Self::typecheck_type(element_type, global_env)?;
            resolved_elements.push((*element_name, type_value));
        }

        // Add the struct to the environment
        global_env.borrow_mut().add_identifier(
            name,
            TypeValue {
                position: position.clone(),
                node: TypeValueType::Struct {
                    name,
                    elements: resolved_elements,
                },
            },
        )
    }

    fn typecheck_function_command(
        &mut self,
        position: Position,
        name: &'a str,
        parameters: &Vec<(LValue<'a>, Type<'a>)>,
        return_type: &Type<'a>,
        statements: &Vec<Statement<'a>>,
        has_return: &Cell<bool>,
        local_env: &Rc<RefCell<TypeEnvironment<'a>>>,
        global_env: &Rc<RefCell<TypeEnvironment<'a>>>,
    ) -> Result<(), TypeCheckerError> {
        // Check return type
        let fn_return_type = Self::typecheck_type(return_type, global_env)?;

        // Create parameter types list
        let mut param_types = Vec::new();

        // Create local scope
        *local_env.borrow_mut() = TypeEnvironment::new(Some(Rc::clone(global_env)));

        // Add parameters to local scope
        for (param, param_type) in parameters {
            let type_value = Self::typecheck_type(param_type, global_env)?;
            param_types.push(type_value.clone());
            self.bind_lvalue(param, type_value, local_env)?;
        }

        // Add function to global environment for recursive calls
        let function_type = TypeValue {
            position: position.clone(),
            node: TypeValueType::Function {
                param_types,
                return_type: Box::new(fn_return_type.clone()),
            },
        };

        if global_env
            .borrow_mut()
            .add_identifier(name, function_type)
            .is_err()
        {
            return Err(TypeCheckerError {
                message: format!("Cannot redefine function {}", name),
                position: position.clone(),
            });
        }

        // Check all statements in the function
        for statement in statements {
            match &statement.node {
                StatementType::Let { variable, rvalue } => {
                    self.typecheck_let_command(variable, rvalue, local_env)?;
                }
                StatementType::Assert {
                    condition,
                    message: _,
                } => {
                    let cond_type = self.typecheck_expression(condition, local_env)?;
                    if !matches!(
                        cond_type.borrow().as_ref().unwrap().node,
                        TypeValueType::Bool
                    ) {
                        return Err(TypeCheckerError {
                            message: "Asserted expression must be boolean".to_string(),
                            position: condition.position.clone(),
                        });
                    }
                }
                StatementType::Return { value } => {
                    let ret_type = self.typecheck_expression(value, local_env)?;
                    if !Self::types_equal(&fn_return_type, ret_type.borrow().as_ref().unwrap()) {
                        return Err(TypeCheckerError {
                            message: "Type of expression does not match return type of function"
                                .to_string(),
                            position: value.position.clone(),
                        });
                    }
                    has_return.replace(true);
                }
            }
        }

        // Check for implicit return
        if !(has_return.get()) && !matches!(fn_return_type.node, TypeValueType::Void) {
            return Err(TypeCheckerError {
                message: "Implicit return type (void) does not match return type of function"
                    .to_string(),
                position: position.clone(),
            });
        };
        Ok(())
    }

    // ----------------------------------------------------------------------------------- Expression Typecheckers ----------------------------------------------------------------------------------------------
    fn typecheck_expression(
        &mut self,
        expression: &Expression<'a>,
        environment: &Rc<RefCell<TypeEnvironment<'a>>>,
    ) -> Result<RefCell<Option<TypeValue<'a>>>, TypeCheckerError> {
        match &expression.node {
            ExpressionType::Int { value: _ } => self.typecheck_int_expression(expression),
            ExpressionType::Float { value: _ } => self.typecheck_float_expression(expression),
            ExpressionType::True => self.typecheck_bool_expression(expression),
            ExpressionType::False => self.typecheck_bool_expression(expression),
            ExpressionType::Void => self.typecheck_void_expression(expression),
            ExpressionType::Variable { name } => {
                let typ = self.typecheck_variable_expression(expression, name, environment)?;
                *expression.resolved_type.borrow_mut() = typ.borrow().clone();
                Ok(typ)
            }
            ExpressionType::ArrayLiteral { elements } => {
                let typ = self.typecheck_array_literal_expression(
                    expression.position.clone(),
                    elements,
                    environment,
                )?;
                *expression.resolved_type.borrow_mut() = typ.borrow().clone();
                Ok(typ)
            }
            ExpressionType::ArrayIndex { array, indices } => {
                let typ = self.typecheck_array_index_expression(
                    expression.position.clone(),
                    array,
                    indices,
                    environment,
                )?;
                *expression.resolved_type.borrow_mut() = typ.borrow().clone();
                Ok(typ)
            }
            ExpressionType::Dot {
                struct_variable,
                field,
            } => {
                let typ = self.typecheck_dot_expression(
                    expression.position.clone(),
                    struct_variable,
                    field,
                    environment,
                )?;
                *expression.resolved_type.borrow_mut() = typ.borrow().clone();
                Ok(typ)
            }
            ExpressionType::Call {
                function,
                arguments,
            } => {
                let typ = self.typecheck_call_expression(
                    expression.position.clone(),
                    function,
                    arguments,
                    environment,
                )?;
                *expression.resolved_type.borrow_mut() = typ.borrow().clone();
                Ok(typ)
            }
            ExpressionType::StructLiteral { name, fields } => {
                let typ = self.typecheck_struct_literal_expression(
                    expression.position.clone(),
                    name,
                    fields,
                    environment,
                )?;
                *expression.resolved_type.borrow_mut() = typ.borrow().clone();
                Ok(typ)
            }
            ExpressionType::Unop {
                operator: _,
                expression: inner,
            } => {
                let typ = self.typecheck_expression(inner, environment)?;
                *expression.resolved_type.borrow_mut() = typ.borrow().clone();
                Ok(typ)
            }
            ExpressionType::Binop {
                operator,
                left,
                right,
            } => {
                let typ = self.typecheck_binop_expression(
                    expression.position.clone(),
                    left,
                    right,
                    operator,
                    environment,
                )?;
                *expression.resolved_type.borrow_mut() = typ.borrow().clone();
                Ok(typ)
            }
            ExpressionType::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let typ = self.typecheck_if_expression(
                    expression.position.clone(),
                    condition,
                    then_branch,
                    else_branch,
                    environment,
                )?;
                *expression.resolved_type.borrow_mut() = typ.borrow().clone();
                Ok(typ)
            }
            ExpressionType::ArrayLoop { range, body } => {
                let typ = self.typecheck_array_loop_expression(
                    expression.position.clone(),
                    range,
                    body,
                    environment,
                )?;
                *expression.resolved_type.borrow_mut() = typ.borrow().clone();
                Ok(typ)
            }
            ExpressionType::SumLoop { range, body } => {
                let typ = self.typecheck_sum_loop_expression(
                    expression.position.clone(),
                    range,
                    body,
                    environment,
                )?;
                *expression.resolved_type.borrow_mut() = typ.borrow().clone();
                Ok(typ)
            }
        }
    }

    fn typecheck_int_expression(
        &mut self,
        expression: &Expression<'a>,
    ) -> Result<RefCell<Option<TypeValue<'a>>>, TypeCheckerError> {
        *expression.resolved_type.borrow_mut() = Some(TypeValue {
            position: expression.position.clone(),
            node: TypeValueType::Int,
        });
        Ok(expression.resolved_type.clone())
    }

    fn typecheck_float_expression(
        &mut self,
        expression: &Expression<'a>,
    ) -> Result<RefCell<Option<TypeValue<'a>>>, TypeCheckerError> {
        *expression.resolved_type.borrow_mut() = Some(TypeValue {
            position: expression.position.clone(),
            node: TypeValueType::Float,
        });
        Ok(expression.resolved_type.clone())
    }

    fn typecheck_bool_expression(
        &mut self,
        expression: &Expression<'a>,
    ) -> Result<RefCell<Option<TypeValue<'a>>>, TypeCheckerError> {
        *expression.resolved_type.borrow_mut() = Some(TypeValue {
            position: expression.position.clone(),
            node: TypeValueType::Bool,
        });
        Ok(expression.resolved_type.clone())
    }

    fn typecheck_void_expression(
        &mut self,
        expression: &Expression<'a>,
    ) -> Result<RefCell<Option<TypeValue<'a>>>, TypeCheckerError> {
        *expression.resolved_type.borrow_mut() = Some(TypeValue {
            position: expression.position.clone(),
            node: TypeValueType::Void,
        });
        Ok(expression.resolved_type.clone())
    }

    fn typecheck_variable_expression(
        &mut self,
        expression: &Expression<'a>,
        name: &'a str,
        environment: &Rc<RefCell<TypeEnvironment<'a>>>,
    ) -> Result<RefCell<Option<TypeValue<'a>>>, TypeCheckerError> {
        let identifier_type = environment
            .borrow_mut()
            .get_identifier(expression.position.clone(), name)?;

        // Check if it's a function type, which is not allowed for variable expressions
        if let TypeValueType::Function { .. } = identifier_type.node {
            return Err(TypeCheckerError {
                message: format!("Name {} is not defined as a variable", name),
                position: expression.position.clone(),
            });
        }

        // Set and return the resolved type
        *expression.resolved_type.borrow_mut() = Some(identifier_type.clone());
        Ok(expression.resolved_type.clone())
    }

    fn typecheck_binop_expression(
        &mut self,
        position: Position,
        left: &Expression<'a>,
        right: &Expression<'a>,
        operator: &Binop,
        environment: &Rc<RefCell<TypeEnvironment<'a>>>,
    ) -> Result<RefCell<Option<TypeValue<'a>>>, TypeCheckerError> {
        let left_type = self.typecheck_expression(left, environment)?;
        let right_type = self.typecheck_expression(right, environment)?;

        // Check if left and right have the same type
        if !Self::types_equal(
            left_type.borrow().as_ref().unwrap(),
            right_type.borrow().as_ref().unwrap(),
        ) {
            return Err(TypeCheckerError {
                message: "Left and right hand operators need to have the same type".to_string(),
                position: position.clone(),
            });
        }

        let left_type_ref = left_type.borrow();
        let left_type_inner = left_type_ref.as_ref().unwrap();

        match operator {
            &Binop::Add | Binop::Subtract | Binop::Multiply | Binop::Divide | Binop::Modulo => {
                // Numeric operators require int or float types
                if !matches!(
                    left_type_inner.node,
                    TypeValueType::Int | TypeValueType::Float
                ) {
                    return Err(TypeCheckerError {
                        message: format!(
                            "Left and right hand operators need to be int or float for {}",
                            operator
                        ),
                        position: position.clone(),
                    });
                }
                // Result type is the same as the operands
                Ok(left_type.clone())
            }
            Binop::Equals | Binop::NotEquals => {
                // Equality operators support int, float, or boolean
                if !matches!(
                    left_type_inner.node,
                    TypeValueType::Int | TypeValueType::Float | TypeValueType::Bool
                ) {
                    return Err(TypeCheckerError {
                        message: format!(
                            "Left and right hand operators need to be int, float or bool for {}",
                            operator
                        ),
                        position: position.clone(),
                    });
                }
                // Result is boolean
                let bool_type = RefCell::new(Some(TypeValue {
                    position: position.clone(),
                    node: TypeValueType::Bool,
                }));
                Ok(bool_type)
            }
            Binop::Less | Binop::Greater | Binop::LessEquals | Binop::GreaterEquals => {
                // Comparison operators require int or float types
                if !matches!(
                    left_type_inner.node,
                    TypeValueType::Int | TypeValueType::Float
                ) {
                    return Err(TypeCheckerError {
                        message: format!(
                            "Left and right hand operators need to be int or float for {}",
                            operator
                        ),
                        position: position.clone(),
                    });
                }
                // Result is boolean
                let bool_type = RefCell::new(Some(TypeValue {
                    position: position.clone(),
                    node: TypeValueType::Bool,
                }));
                Ok(bool_type)
            }
            Binop::And | Binop::Or => {
                // Logical operators require bool types
                if !matches!(left_type_inner.node, TypeValueType::Bool) {
                    return Err(TypeCheckerError {
                        message: format!(
                            "Left and right hand operators need to be bool for {}",
                            operator
                        ),
                        position: position.clone(),
                    });
                }
                // Result is boolean (same as operands)
                Ok(left_type.clone())
            }
        }
    }

    fn typecheck_struct_literal_expression(
        &mut self,
        position: Position,
        name: &'a str,
        fields: &[Expression<'a>],
        environment: &Rc<RefCell<TypeEnvironment<'a>>>,
    ) -> Result<RefCell<Option<TypeValue<'a>>>, TypeCheckerError> {
        // Get the struct type from environment
        let type_value = environment
            .borrow_mut()
            .get_identifier(position.clone(), name)?;

        if let TypeValueType::Struct { name: _, elements } = type_value.node.clone() {
            // Check number of fields
            if elements.len() != fields.len() {
                return Err(TypeCheckerError {
                    message: format!(
                        "Expected {} indices for array access got {}",
                        elements.len(),
                        fields.len()
                    ),
                    position: position.clone(),
                });
            }

            // Check each field type
            for (i, field_expr) in fields.iter().enumerate() {
                let field_type = self.typecheck_expression(field_expr, environment)?;
                let struct_field = &elements[i];

                if !Self::types_equal(field_type.borrow().as_ref().unwrap(), &struct_field.1) {
                    return Err(TypeCheckerError {
                        message: format!("Invalid type of element {} for type {}", i, name),
                        position: position.clone(),
                    });
                }
            }

            Ok(RefCell::new(Some(type_value)))
        } else {
            Err(TypeCheckerError {
                message: format!("{} is not a struct type", name),
                position: position.clone(),
            })
        }
    }

    fn typecheck_array_literal_expression(
        &mut self,
        position: Position,
        elements: &[Expression<'a>],
        environment: &Rc<RefCell<TypeEnvironment<'a>>>,
    ) -> Result<RefCell<Option<TypeValue<'a>>>, TypeCheckerError> {
        if elements.is_empty() {
            return Err(TypeCheckerError {
                message: "Array literal cannot be empty".to_string(),
                position: position.clone(),
            });
        }

        // Check the type of the first element
        let first_type = self.typecheck_expression(&elements[0], environment)?;

        // Check all other elements have the same type
        for (i, element) in elements.iter().enumerate().skip(1) {
            let element_type = self.typecheck_expression(element, environment)?;

            if !Self::types_equal(
                first_type.borrow().as_ref().unwrap(),
                element_type.borrow().as_ref().unwrap(),
            ) {
                return Err(TypeCheckerError {
                    message: format!(
                        "All elements of array literal must have the same type, element {} differs",
                        i
                    ),
                    position: position.clone(),
                });
            }
        }

        // Create and return array type
        let array_type = RefCell::new(Some(TypeValue {
            position: position.clone(),
            node: TypeValueType::Array {
                element_type: Box::new(first_type.borrow().clone().unwrap()),
                rank: 1,
            },
        }));

        Ok(array_type)
    }

    fn typecheck_if_expression(
        &mut self,
        position: Position,
        condition: &Expression<'a>,
        then_branch: &Expression<'a>,
        else_branch: &Expression<'a>,
        environment: &Rc<RefCell<TypeEnvironment<'a>>>,
    ) -> Result<RefCell<Option<TypeValue<'a>>>, TypeCheckerError> {
        // Check that condition is boolean
        let condition_type = self.typecheck_expression(condition, environment)?;
        if !matches!(
            condition_type.borrow().as_ref().unwrap().node,
            TypeValueType::Bool
        ) {
            return Err(TypeCheckerError {
                message: "Type of condition for if expression must be boolean".to_string(),
                position: position.clone(),
            });
        }

        // Check that both branches have the same type
        let then_type = self.typecheck_expression(then_branch, environment)?;
        let else_type = self.typecheck_expression(else_branch, environment)?;

        if !Self::types_equal(
            then_type.borrow().as_ref().unwrap(),
            else_type.borrow().as_ref().unwrap(),
        ) {
            return Err(TypeCheckerError {
                message: "Type of both branches in if expression must match".to_string(),
                position: position.clone(),
            });
        }

        Ok(then_type)
    }

    fn typecheck_dot_expression(
        &mut self,
        position: Position,
        struct_variable: &Expression<'a>,
        field: &'a str,
        environment: &Rc<RefCell<TypeEnvironment<'a>>>,
    ) -> Result<RefCell<Option<TypeValue<'a>>>, TypeCheckerError> {
        // Get struct type
        let variable_type = self.typecheck_expression(struct_variable, environment)?;

        if let Some(TypeValue {
            node: TypeValueType::Struct { name: _, elements },
            ..
        }) = &*variable_type.clone().borrow()
        {
            // Find the field
            for (elem_name, elem_type) in elements {
                if *elem_name == field {
                    return Ok(RefCell::new(Some(elem_type.clone())));
                }
            }

            Err(TypeCheckerError {
                message: format!("Struct has no field named {}", field),
                position: position.clone(),
            })
        } else {
            Err(TypeCheckerError {
                message: "Expression is not a struct and cannot be indexed into".to_string(),
                position: position.clone(),
            })
        }
    }

    fn typecheck_array_index_expression(
        &mut self,
        position: Position,
        array: &Expression<'a>,
        indices: &Vec<Expression<'a>>,
        environment: &Rc<RefCell<TypeEnvironment<'a>>>,
    ) -> Result<RefCell<Option<TypeValue<'a>>>, TypeCheckerError> {
        let array_type = self.typecheck_expression(array, environment)?;

        if let Some(TypeValue {
            node: TypeValueType::Array { element_type, rank },
            ..
        }) = &*array_type.clone().borrow()
        {
            // Check number of indices
            if *rank != indices.len() {
                return Err(TypeCheckerError {
                    message: format!(
                        "Expected {} indices for array access got {}",
                        rank,
                        indices.len()
                    ),
                    position: position.clone(),
                });
            }

            // Check that all indices are integers
            for index in indices {
                let index_type = self.typecheck_expression(index, environment)?;
                if !matches!(
                    index_type.borrow().as_ref().unwrap().node,
                    TypeValueType::Int
                ) {
                    return Err(TypeCheckerError {
                        message: "Array indices must be integers".to_string(),
                        position: index.position.clone(),
                    });
                }
            }

            // Return element type
            Ok(RefCell::new(Some(*element_type.clone())))
        } else {
            Err(TypeCheckerError {
                message: "Expression is not an array and cannot be indexed".to_string(),
                position: position.clone(),
            })
        }
    }

    fn typecheck_call_expression(
        &mut self,
        position: Position,
        function: &'a str,
        arguments: &[Expression<'a>],
        environment: &Rc<RefCell<TypeEnvironment<'a>>>,
    ) -> Result<RefCell<Option<TypeValue<'a>>>, TypeCheckerError> {
        // Get function type
        let func_type = environment
            .borrow()
            .get_identifier(position.clone(), function)?;

        if let TypeValue {
            node:
                TypeValueType::Function {
                    param_types,
                    return_type,
                },
            ..
        } = func_type
        {
            // Check number of arguments
            if param_types.len() != arguments.len() {
                return Err(TypeCheckerError {
                    message: format!("Incorrect number of parameters for function {}", function),
                    position: position.clone(),
                });
            }

            // Check each argument type
            for (i, (arg, param_type)) in arguments.iter().zip(param_types.iter()).enumerate() {
                let arg_type = self.typecheck_expression(arg, environment)?;
                if !Self::types_equal(arg_type.borrow().as_ref().unwrap(), param_type) {
                    return Err(TypeCheckerError {
                        message: format!(
                            "Incorrect type of parameter {} for function {}",
                            i, function
                        ),
                        position: arg.position.clone(),
                    });
                }
            }

            // Return function's return type
            Ok(RefCell::new(Some(*return_type.clone())))
        } else {
            Err(TypeCheckerError {
                message: format!("{} is not a function", function),
                position: position.clone(),
            })
        }
    }

    fn typecheck_array_loop_expression(
        &mut self,
        position: Position,
        range: &Vec<(&'a str, Expression<'a>)>,
        body: &Expression<'a>,
        environment: &Rc<RefCell<TypeEnvironment<'a>>>,
    ) -> Result<RefCell<Option<TypeValue<'a>>>, TypeCheckerError> {
        if range.is_empty() {
            return Err(TypeCheckerError {
                message: "Cannot make array of rank 0".to_string(),
                position: position.clone(),
            });
        }

        let local_env = Rc::new(RefCell::new(TypeEnvironment::new(Some(
            environment.clone(),
        ))));

        // Add loop variables to scope
        for (var_name, limit_expr) in range {
            let limit_type = self.typecheck_expression(limit_expr, environment)?;

            if !matches!(
                limit_type.borrow().as_ref().unwrap().node,
                TypeValueType::Int
            ) {
                return Err(TypeCheckerError {
                    message: "Cannot iterate to non-integer value".to_string(),
                    position: limit_expr.position.clone(),
                });
            }

            local_env
                .borrow_mut()
                .add_identifier(
                    var_name,
                    TypeValue {
                        position: position.clone(),
                        node: TypeValueType::Int,
                    },
                )
                .map_err(|_| TypeCheckerError {
                    message: format!("Cannot redefine variable {}", var_name),
                    position: position.clone(),
                })?;
        }

        // Type check the body
        let element_type = self.typecheck_expression(body, &local_env)?;

        // Create array type
        let array_type = RefCell::new(Some(TypeValue {
            position: position.clone(),
            node: TypeValueType::Array {
                element_type: Box::new(element_type.borrow().clone().unwrap()),
                rank: range.len(),
            },
        }));

        Ok(array_type)
    }

    fn typecheck_sum_loop_expression(
        &mut self,
        position: Position,
        range: &Vec<(&'a str, Expression<'a>)>,
        body: &Expression<'a>,
        environment: &Rc<RefCell<TypeEnvironment<'a>>>,
    ) -> Result<RefCell<Option<TypeValue<'a>>>, TypeCheckerError> {
        if range.is_empty() {
            return Err(TypeCheckerError {
                message: "Cannot sum over array of rank 0".to_string(),
                position: position.clone(),
            });
        }

        let local_env = Rc::new(RefCell::new(TypeEnvironment::new(Some(
            environment.clone(),
        ))));

        // Add loop variables to scope
        for (var_name, limit_expr) in range {
            let limit_type = self.typecheck_expression(limit_expr, environment)?;

            if !matches!(
                limit_type.borrow().as_ref().unwrap().node,
                TypeValueType::Int
            ) {
                return Err(TypeCheckerError {
                    message: "Cannot iterate to non-integer value".to_string(),
                    position: limit_expr.position.clone(),
                });
            }

            local_env
                .borrow_mut()
                .add_identifier(
                    var_name,
                    TypeValue {
                        position: position.clone(),
                        node: TypeValueType::Int,
                    },
                )
                .map_err(|_| TypeCheckerError {
                    message: format!("Cannot redefine variable {}", var_name),
                    position: position.clone(),
                })?;
        }

        // Type check the body
        let element_type = self.typecheck_expression(body, &local_env)?;

        // Check if body type is numeric (int or float)
        if !matches!(
            element_type.borrow().as_ref().unwrap().node,
            TypeValueType::Int | TypeValueType::Float
        ) {
            return Err(TypeCheckerError {
                message: "Cannot sum over elements that are not int or float".to_string(),
                position: body.position.clone(),
            });
        }

        // Return element type (int or float)
        Ok(element_type)
    }
}

pub fn typecheck<'a>(
    commands: &'a Vec<Command<'a>>,
) -> Result<Rc<RefCell<TypeEnvironment<'a>>>, TypeCheckerError> {
    let mut typechecker = TypeChecker::new(commands);
    typechecker.typecheck()
}
