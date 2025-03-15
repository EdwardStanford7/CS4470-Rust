use crate::ast::*;
use crate::lexer::Position;
use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    fmt::Display,
    rc::Rc,
};

pub struct TypeError {
    pub message: String,
    pub position: Position,
}

impl Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Type error: {}:{}: {}",
            self.position.line, self.position.column, self.message
        )
    }
}

pub struct TypeEnvironment<'a> {
    local_environment: HashMap<&'a str, Type<'a>>,
    parent: Option<Rc<RefCell<TypeEnvironment<'a>>>>,
}

impl<'a> TypeEnvironment<'a> {
    pub fn new(parent: Option<Rc<RefCell<TypeEnvironment<'a>>>>) -> TypeEnvironment<'a> {
        TypeEnvironment {
            local_environment: HashMap::new(),
            parent,
        }
    }

    pub fn add_identifier(&mut self, name: &'a str, typ: Type<'a>) -> Result<(), TypeError> {
        match self.local_environment.entry(name) {
            Entry::Occupied(_) => {
                return Err(TypeError {
                    message: format!("Cannot redefine the name {}", name),
                    position: typ.position,
                });
            }
            Entry::Vacant(entry) => {
                entry.insert(typ);
            }
        }
        Ok(())
    }

    pub fn get_identifier(&self, position: Position, name: &'a str) -> Result<Type<'a>, TypeError> {
        match self.local_environment.get(name) {
            Some(typ) => Ok(typ.clone()),
            None => match &self.parent {
                Some(parent) => parent.borrow().get_identifier(position, name),
                None => Err(TypeError {
                    message: format!("Identifier {} is undefined", name),
                    position,
                }),
            },
        }
    }
}

pub fn typecheck(
    mut commands: Vec<Command<'_>>,
) -> Result<(Vec<Command<'_>>, Rc<RefCell<TypeEnvironment<'_>>>), TypeError> {
    let global_env = Rc::new(RefCell::new(TypeEnvironment::new(None)));

    populate_built_ins(&global_env);

    for command in commands.iter_mut() {
        typecheck_command(command, &global_env)?;
    }

    Ok((commands, global_env))
}

fn pos() -> Position {
    Position { line: 0, column: 0 }
}

fn populate_built_ins(global_env: &Rc<RefCell<TypeEnvironment<'_>>>) {
    // Create basic types
    let float_type = || Type {
        position: pos(),
        node: TypeType::Float,
    };

    let int_type = || Type {
        position: pos(),
        node: TypeType::Int,
    };

    // RGBA struct type
    let rgba = Type {
        position: pos(),
        node: TypeType::Struct {
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
    let args = Type {
        position: pos(),
        node: TypeType::Array {
            element_type: Box::new(int_type()),
            rank: 1,
        },
    };

    // Function types
    let one_float_to_float = Type {
        position: pos(),
        node: TypeType::Function {
            param_types: vec![float_type()],
            return_type: Box::new(float_type()),
        },
    };

    let two_floats_to_float = Type {
        position: pos(),
        node: TypeType::Function {
            param_types: vec![float_type(), float_type()],
            return_type: Box::new(float_type()),
        },
    };

    let to_float = Type {
        position: pos(),
        node: TypeType::Function {
            param_types: vec![int_type()],
            return_type: Box::new(float_type()),
        },
    };

    let to_int = Type {
        position: pos(),
        node: TypeType::Function {
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

fn bind_lvalue<'a>(
    lvalue: &LValue<'a>,
    typ: Type<'a>,
    environment: &Rc<RefCell<TypeEnvironment<'a>>>,
) -> Result<(), TypeError> {
    match &lvalue.node {
        LValueType::Array { indices } => {
            // Add the identifier with the type
            environment
                .borrow_mut()
                .add_identifier(lvalue.name, typ.clone())?;

            // Check if the type is an array with matching rank
            if let TypeType::Array {
                element_type: _,
                rank,
            } = &typ.node
            {
                if *rank != indices.len() {
                    return Err(TypeError {
                        message: format!(
                            "Rank of array lvalue ({}) does not match right hand side ({})",
                            indices.len(),
                            rank
                        ),
                        position: lvalue.position,
                    });
                }
            } else {
                return Err(TypeError {
                    message: "Incorrect type for array lvalue".to_string(),
                    position: lvalue.position,
                });
            }

            // Bind index variables as integers
            for &index in indices {
                environment.borrow_mut().add_identifier(
                    index,
                    Type {
                        position: pos(),
                        node: TypeType::Int,
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

fn typecheck_type<'a>(
    typ: &Type<'a>,
    env: &Rc<RefCell<TypeEnvironment<'a>>>,
) -> Result<Type<'a>, TypeError> {
    match &typ.node {
        TypeType::Int => Ok(Type {
            position: typ.position,
            node: TypeType::Int,
        }),
        TypeType::Float => Ok(Type {
            position: typ.position,
            node: TypeType::Float,
        }),
        TypeType::Bool => Ok(Type {
            position: typ.position,
            node: TypeType::Bool,
        }),
        TypeType::Void => Ok(Type {
            position: typ.position,
            node: TypeType::Void,
        }),
        TypeType::Array { element_type, rank } => {
            let type_value = typecheck_type(element_type, env)?;
            Ok(Type {
                position: typ.position,
                node: TypeType::Array {
                    element_type: Box::new(type_value),
                    rank: *rank,
                },
            })
        }
        TypeType::Struct { name, elements: _ } => {
            match env.borrow().get_identifier(typ.position, name) {
                Ok(struct_type) => Ok(struct_type.clone()),
                Err(err) => Err(err),
            }
        }
        TypeType::Function {
            param_types,
            return_type,
        } => {
            let mut resolved_param_types = Vec::new();
            for param_type in param_types {
                resolved_param_types.push(typecheck_type(param_type, env)?);
            }
            let resolved_return_type = typecheck_type(return_type, env)?;
            Ok(Type {
                position: typ.position,
                node: TypeType::Function {
                    param_types: resolved_param_types,
                    return_type: Box::new(resolved_return_type),
                },
            })
        }
    }
}

fn types_equal<'a>(left: &Type<'a>, right: &Type<'a>) -> bool {
    match (&left.node, &right.node) {
        (TypeType::Int, TypeType::Int) => true,
        (TypeType::Float, TypeType::Float) => true,
        (TypeType::Bool, TypeType::Bool) => true,
        (TypeType::Void, TypeType::Void) => true,
        (
            TypeType::Array {
                element_type: left_element,
                rank: left_rank,
            },
            TypeType::Array {
                element_type: right_element,
                rank: right_rank,
            },
        ) => left_rank == right_rank && types_equal(left_element, right_element),
        (
            TypeType::Struct {
                name: left_name, ..
            },
            TypeType::Struct {
                name: right_name, ..
            },
        ) => left_name == right_name,
        _ => false,
    }
}

// ----------------------------------------------------------------------------------- Command Typecheckers ----------------------------------------------------------------------------------------------

fn typecheck_command<'a>(
    command: &Command<'a>,
    global_env: &Rc<RefCell<TypeEnvironment<'a>>>,
) -> Result<(), TypeError> {
    match &command.node {
        CommandType::Read {
            source: _,
            destination,
        } => typecheck_read_command(command.position, destination, global_env),
        CommandType::Write {
            source,
            destination: _,
        } => typecheck_write_command(command.position, source, global_env),
        CommandType::Let { variable, rvalue } => {
            typecheck_let_command(variable, rvalue, global_env)
        }
        CommandType::Assert {
            condition,
            message: _,
        } => typecheck_assert_command(condition, global_env),
        CommandType::Print { message: _ } => Ok(()),
        CommandType::Show { expression } => {
            typecheck_expression(expression, global_env)?;
            Ok(())
        }
        CommandType::Time {
            command: inner_command,
        } => typecheck_command(inner_command, global_env),
        CommandType::Struct { name, elements } => {
            typecheck_struct_command(command.position, name, elements, global_env)
        }
        CommandType::Function {
            name,
            parameters,
            return_type,
            statements,
            mut has_return,
            local_env,
        } => {
            // Check return type
            let fn_return_type = typecheck_type(return_type, global_env)?;

            // Create parameter types list
            let mut param_types = Vec::new();

            // Create local scope
            *local_env.borrow_mut() = TypeEnvironment::new(Some(Rc::clone(global_env)));

            // Add parameters to local scope
            for (param, param_type) in parameters {
                let type_value = typecheck_type(param_type, global_env)?;
                param_types.push(type_value.clone());
                bind_lvalue(param, type_value, local_env)?;
            }

            let position = command.position;

            // Add function to global environment for recursive calls
            let function_type = Type {
                position,
                node: TypeType::Function {
                    param_types,
                    return_type: Box::new(fn_return_type.clone()),
                },
            };

            if global_env
                .borrow_mut()
                .add_identifier(name, function_type)
                .is_err()
            {
                return Err(TypeError {
                    message: format!("Cannot redefine function {}", name),
                    position,
                });
            }

            // Check all statements in the function
            for statement in statements {
                match &statement.node {
                    StatementType::Let { variable, rvalue } => {
                        typecheck_let_command(variable, rvalue, local_env)?;
                    }
                    StatementType::Assert {
                        condition,
                        message: _,
                    } => {
                        let cond_type = typecheck_expression(condition, local_env)?;
                        if !matches!(cond_type.borrow().as_ref().unwrap().node, TypeType::Bool) {
                            return Err(TypeError {
                                message: "Asserted expression must be boolean".to_string(),
                                position: condition.position,
                            });
                        }
                    }
                    StatementType::Return { value } => {
                        let ret_type = typecheck_expression(value, local_env)?;
                        if !types_equal(&fn_return_type, ret_type.borrow().as_ref().unwrap()) {
                            return Err(TypeError {
                                message:
                                    "Type of expression does not match return type of function"
                                        .to_string(),
                                position: value.position,
                            });
                        }
                        has_return = true
                    }
                }
            }

            // Check for implicit return
            if !has_return && !matches!(fn_return_type.node, TypeType::Void) {
                return Err(TypeError {
                    message: "Implicit return type (void) does not match return type of function"
                        .to_string(),
                    position,
                });
            };
            Ok(())
        }
    }
}

fn typecheck_read_command<'a>(
    position: Position,
    destination: &LValue<'a>,
    global_env: &Rc<RefCell<TypeEnvironment<'a>>>,
) -> Result<(), TypeError> {
    let rgba = global_env.borrow().get_identifier(position, "rgba")?;
    let rgba_array = Type {
        position: pos(),
        node: TypeType::Array {
            element_type: Box::new(rgba.clone()),
            rank: 2,
        },
    };

    bind_lvalue(destination, rgba_array, global_env)?;

    if let LValueType::Array { indices } = &destination.node {
        if indices.len() != 2 {
            return Err(TypeError {
                message: format!(
                    "Cannot bind rgba[,] to array of dimension  {}",
                    indices.len()
                ),
                position: destination.position,
            });
        }
        for index in indices {
            let _ = global_env.borrow_mut().add_identifier(
                index,
                Type {
                    position: pos(),
                    node: TypeType::Int,
                },
            );
        }
    };

    Ok(())
}

fn typecheck_write_command<'a>(
    position: Position,
    source: &Expression<'a>,
    global_env: &Rc<RefCell<TypeEnvironment<'a>>>,
) -> Result<(), TypeError> {
    let image_type = typecheck_expression(source, global_env)?;

    let is_rgba_array = matches!(
        &image_type.borrow().as_ref().unwrap().node,
        TypeType::Array {
            element_type,
            rank: 2,
        } if matches!(element_type.node, TypeType::Struct { name, .. } if name == "rgba")
    );

    if !is_rgba_array {
        Err(TypeError {
            message: "Expression for write needs to be rgba[,]".to_string(),
            position,
        })
    } else {
        Ok(())
    }
}

fn typecheck_assert_command<'a>(
    condition: &Expression<'a>,
    global_env: &Rc<RefCell<TypeEnvironment<'a>>>,
) -> Result<(), TypeError> {
    let condition_type = typecheck_expression(condition, global_env)?;
    if !matches!(
        condition_type.borrow().as_ref().unwrap().node,
        TypeType::Bool
    ) {
        return Err(TypeError {
            message: "Asserted expression must be boolean".to_string(),
            position: condition.position,
        });
    }
    Ok(())
}

fn typecheck_let_command<'a>(
    variable: &LValue<'a>,
    rvalue: &Expression<'a>,
    global_env: &Rc<RefCell<TypeEnvironment<'a>>>,
) -> Result<(), TypeError> {
    let rtype = typecheck_expression(rvalue, global_env)?;
    let cloned_rtype = rtype.borrow().clone().unwrap();
    bind_lvalue(variable, cloned_rtype, global_env)
}

fn typecheck_struct_command<'a>(
    position: Position,
    name: &'a str,
    elements: &Vec<(&'a str, Type<'a>)>,
    global_env: &Rc<RefCell<TypeEnvironment<'a>>>,
) -> Result<(), TypeError> {
    let mut resolved_elements = Vec::new();

    for (element_name, element_type) in elements {
        // Check if duplicate name exists inside struct
        if resolved_elements
            .iter()
            .any(|(name, _)| name == element_name)
        {
            return Err(TypeError {
                message: format!("Duplicate field name {} in struct", element_name),
                position,
            });
        }

        let type_value = typecheck_type(element_type, global_env)?;
        resolved_elements.push((*element_name, type_value));
    }

    // Add the struct to the environment
    global_env.borrow_mut().add_identifier(
        name,
        Type {
            position,
            node: TypeType::Struct {
                name,
                elements: resolved_elements,
            },
        },
    )
}

// ----------------------------------------------------------------------------------- Expression Typecheckers ----------------------------------------------------------------------------------------------
fn typecheck_expression<'a>(
    expression: &Expression<'a>,
    environment: &Rc<RefCell<TypeEnvironment<'a>>>,
) -> Result<RefCell<Option<Type<'a>>>, TypeError> {
    match &expression.node {
        ExpressionType::Int { value: _ } => typecheck_int_expression(expression),
        ExpressionType::Float { value: _ } => typecheck_float_expression(expression),
        ExpressionType::True => typecheck_bool_expression(expression),
        ExpressionType::False => typecheck_bool_expression(expression),
        ExpressionType::Void => typecheck_void_expression(expression),
        ExpressionType::Variable { name } => {
            let typ = typecheck_variable_expression(expression, name, environment)?;
            *expression.resolved_type.borrow_mut() = typ.borrow().clone();
            Ok(typ)
        }
        ExpressionType::ArrayLiteral { elements } => {
            let typ =
                typecheck_array_literal_expression(expression.position, elements, environment)?;
            *expression.resolved_type.borrow_mut() = typ.borrow().clone();
            Ok(typ)
        }
        ExpressionType::ArrayIndex { array, indices } => {
            let typ =
                typecheck_array_index_expression(expression.position, array, indices, environment)?;
            *expression.resolved_type.borrow_mut() = typ.borrow().clone();
            Ok(typ)
        }
        ExpressionType::Dot {
            struct_variable,
            field,
        } => {
            let typ =
                typecheck_dot_expression(expression.position, struct_variable, field, environment)?;
            *expression.resolved_type.borrow_mut() = typ.borrow().clone();
            Ok(typ)
        }
        ExpressionType::Call {
            function,
            arguments,
        } => {
            let typ =
                typecheck_call_expression(expression.position, function, arguments, environment)?;
            *expression.resolved_type.borrow_mut() = typ.borrow().clone();
            Ok(typ)
        }
        ExpressionType::StructLiteral { name, fields } => {
            let typ = typecheck_struct_literal_expression(
                expression.position,
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
            let typ = typecheck_expression(inner, environment)?;
            *expression.resolved_type.borrow_mut() = typ.borrow().clone();
            Ok(typ)
        }
        ExpressionType::Binop {
            operator,
            left,
            right,
        } => {
            let typ = typecheck_binop_expression(
                expression.position,
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
            let typ = typecheck_if_expression(
                expression.position,
                condition,
                then_branch,
                else_branch,
                environment,
            )?;
            *expression.resolved_type.borrow_mut() = typ.borrow().clone();
            Ok(typ)
        }
        ExpressionType::ArrayLoop { range, body } => {
            let typ =
                typecheck_array_loop_expression(expression.position, range, body, environment)?;
            *expression.resolved_type.borrow_mut() = typ.borrow().clone();
            Ok(typ)
        }
        ExpressionType::SumLoop { range, body } => {
            let typ = typecheck_sum_loop_expression(expression.position, range, body, environment)?;
            *expression.resolved_type.borrow_mut() = typ.borrow().clone();
            Ok(typ)
        }
    }
}

fn typecheck_int_expression<'a>(
    expression: &Expression<'a>,
) -> Result<RefCell<Option<Type<'a>>>, TypeError> {
    *expression.resolved_type.borrow_mut() = Some(Type {
        position: expression.position,
        node: TypeType::Int,
    });
    Ok(expression.resolved_type.clone())
}

fn typecheck_float_expression<'a>(
    expression: &Expression<'a>,
) -> Result<RefCell<Option<Type<'a>>>, TypeError> {
    *expression.resolved_type.borrow_mut() = Some(Type {
        position: expression.position,
        node: TypeType::Float,
    });
    Ok(expression.resolved_type.clone())
}

fn typecheck_bool_expression<'a>(
    expression: &Expression<'a>,
) -> Result<RefCell<Option<Type<'a>>>, TypeError> {
    *expression.resolved_type.borrow_mut() = Some(Type {
        position: expression.position,
        node: TypeType::Bool,
    });
    Ok(expression.resolved_type.clone())
}

fn typecheck_void_expression<'a>(
    expression: &Expression<'a>,
) -> Result<RefCell<Option<Type<'a>>>, TypeError> {
    *expression.resolved_type.borrow_mut() = Some(Type {
        position: expression.position,
        node: TypeType::Void,
    });
    Ok(expression.resolved_type.clone())
}

fn typecheck_variable_expression<'a>(
    expression: &Expression<'a>,
    name: &'a str,
    environment: &Rc<RefCell<TypeEnvironment<'a>>>,
) -> Result<RefCell<Option<Type<'a>>>, TypeError> {
    let identifier_type = environment
        .borrow_mut()
        .get_identifier(expression.position, name)?;

    // Check if it's a function type, which is not allowed for variable expressions
    if let TypeType::Function { .. } = identifier_type.node {
        return Err(TypeError {
            message: format!("Name {} is not defined as a variable", name),
            position: expression.position,
        });
    }

    // Set and return the resolved type
    *expression.resolved_type.borrow_mut() = Some(identifier_type.clone());
    Ok(expression.resolved_type.clone())
}

fn typecheck_binop_expression<'a>(
    position: Position,
    left: &Expression<'a>,
    right: &Expression<'a>,
    operator: &Binop,
    environment: &Rc<RefCell<TypeEnvironment<'a>>>,
) -> Result<RefCell<Option<Type<'a>>>, TypeError> {
    let left_type = typecheck_expression(left, environment)?;
    let right_type = typecheck_expression(right, environment)?;

    // Check if left and right have the same type
    if !types_equal(
        left_type.borrow().as_ref().unwrap(),
        right_type.borrow().as_ref().unwrap(),
    ) {
        return Err(TypeError {
            message: "Left and right hand operators need to have the same type".to_string(),
            position,
        });
    }

    let left_type_ref = left_type.borrow();
    let left_type_inner = left_type_ref.as_ref().unwrap();

    match operator {
        &Binop::Add | Binop::Subtract | Binop::Multiply | Binop::Divide | Binop::Modulo => {
            // Numeric operators require int or float types
            if !matches!(left_type_inner.node, TypeType::Int | TypeType::Float) {
                return Err(TypeError {
                    message: format!(
                        "Left and right hand operators need to be int or float for {}",
                        operator
                    ),
                    position,
                });
            }
            // Result type is the same as the operands
            Ok(left_type.clone())
        }
        Binop::Equals | Binop::NotEquals => {
            // Equality operators support int, float, or boolean
            if !matches!(
                left_type_inner.node,
                TypeType::Int | TypeType::Float | TypeType::Bool
            ) {
                return Err(TypeError {
                    message: format!(
                        "Left and right hand operators need to be int, float or bool for {}",
                        operator
                    ),
                    position,
                });
            }
            // Result is boolean
            let bool_type = RefCell::new(Some(Type {
                position,
                node: TypeType::Bool,
            }));
            Ok(bool_type)
        }
        Binop::Less | Binop::Greater | Binop::LessEquals | Binop::GreaterEquals => {
            // Comparison operators require int or float types
            if !matches!(left_type_inner.node, TypeType::Int | TypeType::Float) {
                return Err(TypeError {
                    message: format!(
                        "Left and right hand operators need to be int or float for {}",
                        operator
                    ),
                    position,
                });
            }
            // Result is boolean
            let bool_type = RefCell::new(Some(Type {
                position,
                node: TypeType::Bool,
            }));
            Ok(bool_type)
        }
        Binop::And | Binop::Or => {
            // Logical operators require bool types
            if !matches!(left_type_inner.node, TypeType::Bool) {
                return Err(TypeError {
                    message: format!(
                        "Left and right hand operators need to be bool for {}",
                        operator
                    ),
                    position,
                });
            }
            // Result is boolean (same as operands)
            Ok(left_type.clone())
        }
    }
}

fn typecheck_struct_literal_expression<'a>(
    position: Position,
    name: &'a str,
    fields: &[Expression<'a>],
    environment: &Rc<RefCell<TypeEnvironment<'a>>>,
) -> Result<RefCell<Option<Type<'a>>>, TypeError> {
    // Get the struct type from environment
    let type_value = environment.borrow_mut().get_identifier(position, name)?;

    if let TypeType::Struct { name: _, elements } = type_value.node.clone() {
        // Check number of fields
        if elements.len() != fields.len() {
            return Err(TypeError {
                message: format!(
                    "Expected {} indices for array access got {}",
                    elements.len(),
                    fields.len()
                ),
                position,
            });
        }

        // Check each field type
        for (i, field_expr) in fields.iter().enumerate() {
            let field_type = typecheck_expression(field_expr, environment)?;
            let struct_field = &elements[i];

            if !types_equal(field_type.borrow().as_ref().unwrap(), &struct_field.1) {
                return Err(TypeError {
                    message: format!("Invalid type of element {} for type {}", i, name),
                    position,
                });
            }
        }

        Ok(RefCell::new(Some(type_value)))
    } else {
        Err(TypeError {
            message: format!("{} is not a struct type", name),
            position,
        })
    }
}

fn typecheck_array_literal_expression<'a>(
    position: Position,
    elements: &[Expression<'a>],
    environment: &Rc<RefCell<TypeEnvironment<'a>>>,
) -> Result<RefCell<Option<Type<'a>>>, TypeError> {
    if elements.is_empty() {
        return Err(TypeError {
            message: "Array literal cannot be empty".to_string(),
            position,
        });
    }

    // Check the type of the first element
    let first_type = typecheck_expression(&elements[0], environment)?;

    // Check all other elements have the same type
    for (i, element) in elements.iter().enumerate().skip(1) {
        let element_type = typecheck_expression(element, environment)?;

        if !types_equal(
            first_type.borrow().as_ref().unwrap(),
            element_type.borrow().as_ref().unwrap(),
        ) {
            return Err(TypeError {
                message: format!(
                    "All elements of array literal must have the same type, element {} differs",
                    i
                ),
                position,
            });
        }
    }

    // Create and return array type
    let array_type = RefCell::new(Some(Type {
        position,
        node: TypeType::Array {
            element_type: Box::new(first_type.borrow().clone().unwrap()),
            rank: 1,
        },
    }));

    Ok(array_type)
}

fn typecheck_if_expression<'a>(
    position: Position,
    condition: &Expression<'a>,
    then_branch: &Expression<'a>,
    else_branch: &Expression<'a>,
    environment: &Rc<RefCell<TypeEnvironment<'a>>>,
) -> Result<RefCell<Option<Type<'a>>>, TypeError> {
    // Check that condition is boolean
    let condition_type = typecheck_expression(condition, environment)?;
    if !matches!(
        condition_type.borrow().as_ref().unwrap().node,
        TypeType::Bool
    ) {
        return Err(TypeError {
            message: "Type of condition for if expression must be boolean".to_string(),
            position,
        });
    }

    // Check that both branches have the same type
    let then_type = typecheck_expression(then_branch, environment)?;
    let else_type = typecheck_expression(else_branch, environment)?;

    if !types_equal(
        then_type.borrow().as_ref().unwrap(),
        else_type.borrow().as_ref().unwrap(),
    ) {
        return Err(TypeError {
            message: "Type of both branches in if expression must match".to_string(),
            position,
        });
    }

    Ok(then_type)
}

fn typecheck_dot_expression<'a>(
    position: Position,
    struct_variable: &Expression<'a>,
    field: &'a str,
    environment: &Rc<RefCell<TypeEnvironment<'a>>>,
) -> Result<RefCell<Option<Type<'a>>>, TypeError> {
    // Get struct type
    let variable_type = typecheck_expression(struct_variable, environment)?;

    if let Some(Type {
        node: TypeType::Struct { name: _, elements },
        ..
    }) = &*variable_type.clone().borrow()
    {
        // Find the field
        for (elem_name, elem_type) in elements {
            if *elem_name == field {
                return Ok(RefCell::new(Some(elem_type.clone())));
            }
        }

        Err(TypeError {
            message: format!("Struct has no field named {}", field),
            position,
        })
    } else {
        Err(TypeError {
            message: "Expression is not a struct and cannot be indexed into".to_string(),
            position,
        })
    }
}

fn typecheck_array_index_expression<'a>(
    position: Position,
    array: &Expression<'a>,
    indices: &Vec<Expression<'a>>,
    environment: &Rc<RefCell<TypeEnvironment<'a>>>,
) -> Result<RefCell<Option<Type<'a>>>, TypeError> {
    let array_type = typecheck_expression(array, environment)?;

    if let Some(Type {
        node: TypeType::Array { element_type, rank },
        ..
    }) = &*array_type.clone().borrow()
    {
        // Check number of indices
        if *rank != indices.len() {
            return Err(TypeError {
                message: format!(
                    "Expected {} indices for array access got {}",
                    rank,
                    indices.len()
                ),
                position,
            });
        }

        // Check that all indices are integers
        for index in indices {
            let index_type = typecheck_expression(index, environment)?;
            if !matches!(index_type.borrow().as_ref().unwrap().node, TypeType::Int) {
                return Err(TypeError {
                    message: "Array indices must be integers".to_string(),
                    position: index.position,
                });
            }
        }

        // Return element type
        Ok(RefCell::new(Some(*element_type.clone())))
    } else {
        Err(TypeError {
            message: "Expression is not an array and cannot be indexed".to_string(),
            position,
        })
    }
}

fn typecheck_call_expression<'a>(
    position: Position,
    function: &'a str,
    arguments: &[Expression<'a>],
    environment: &Rc<RefCell<TypeEnvironment<'a>>>,
) -> Result<RefCell<Option<Type<'a>>>, TypeError> {
    // Get function type
    let func_type = environment.borrow().get_identifier(position, function)?;

    if let Type {
        node: TypeType::Function {
            param_types,
            return_type,
        },
        ..
    } = func_type
    {
        // Check number of arguments
        if param_types.len() != arguments.len() {
            return Err(TypeError {
                message: format!("Incorrect number of parameters for function {}", function),
                position,
            });
        }

        // Check each argument type
        for (i, (arg, param_type)) in arguments.iter().zip(param_types.iter()).enumerate() {
            let arg_type = typecheck_expression(arg, environment)?;
            if !types_equal(arg_type.borrow().as_ref().unwrap(), param_type) {
                return Err(TypeError {
                    message: format!(
                        "Incorrect type of parameter {} for function {}",
                        i, function
                    ),
                    position: arg.position,
                });
            }
        }

        // Return function's return type
        Ok(RefCell::new(Some(*return_type.clone())))
    } else {
        Err(TypeError {
            message: format!("{} is not a function", function),
            position,
        })
    }
}

fn typecheck_array_loop_expression<'a>(
    position: Position,
    range: &Vec<(&'a str, Expression<'a>)>,
    body: &Expression<'a>,
    environment: &Rc<RefCell<TypeEnvironment<'a>>>,
) -> Result<RefCell<Option<Type<'a>>>, TypeError> {
    if range.is_empty() {
        return Err(TypeError {
            message: "Cannot make array of rank 0".to_string(),
            position,
        });
    }

    let local_env = Rc::new(RefCell::new(TypeEnvironment::new(Some(
        environment.clone(),
    ))));

    // Add loop variables to scope
    for (var_name, limit_expr) in range {
        let limit_type = typecheck_expression(limit_expr, environment)?;

        if !matches!(limit_type.borrow().as_ref().unwrap().node, TypeType::Int) {
            return Err(TypeError {
                message: "Cannot iterate to non-integer value".to_string(),
                position: limit_expr.position,
            });
        }

        local_env
            .borrow_mut()
            .add_identifier(
                var_name,
                Type {
                    position,
                    node: TypeType::Int,
                },
            )
            .map_err(|_| TypeError {
                message: format!("Cannot redefine variable {}", var_name),
                position,
            })?;
    }

    // Type check the body
    let element_type = typecheck_expression(body, &local_env)?;

    // Create array type
    let array_type = RefCell::new(Some(Type {
        position,
        node: TypeType::Array {
            element_type: Box::new(element_type.borrow().clone().unwrap()),
            rank: range.len(),
        },
    }));

    Ok(array_type)
}

fn typecheck_sum_loop_expression<'a>(
    position: Position,
    range: &Vec<(&'a str, Expression<'a>)>,
    body: &Expression<'a>,
    environment: &Rc<RefCell<TypeEnvironment<'a>>>,
) -> Result<RefCell<Option<Type<'a>>>, TypeError> {
    if range.is_empty() {
        return Err(TypeError {
            message: "Cannot sum over array of rank 0".to_string(),
            position,
        });
    }

    let local_env = Rc::new(RefCell::new(TypeEnvironment::new(Some(
        environment.clone(),
    ))));

    // Add loop variables to scope
    for (var_name, limit_expr) in range {
        let limit_type = typecheck_expression(limit_expr, environment)?;

        if !matches!(limit_type.borrow().as_ref().unwrap().node, TypeType::Int) {
            return Err(TypeError {
                message: "Cannot iterate to non-integer value".to_string(),
                position: limit_expr.position,
            });
        }

        local_env
            .borrow_mut()
            .add_identifier(
                var_name,
                Type {
                    position,
                    node: TypeType::Int,
                },
            )
            .map_err(|_| TypeError {
                message: format!("Cannot redefine variable {}", var_name),
                position,
            })?;
    }

    // Type check the body
    let element_type = typecheck_expression(body, &local_env)?;

    // Check if body type is numeric (int or float)
    if !matches!(
        element_type.borrow().as_ref().unwrap().node,
        TypeType::Int | TypeType::Float
    ) {
        return Err(TypeError {
            message: "Cannot sum over elements that are not int or float".to_string(),
            position: body.position,
        });
    }

    // Return element type (int or float)
    Ok(element_type)
}
