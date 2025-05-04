use crate::ast::*;
use crate::utils::*;

pub fn typecheck(mut commands: Vec<Command<'_>>) -> Result<Vec<Command<'_>>, TypeError> {
    let mut environment = TypeEnvironment::new();

    populate_built_ins(&mut environment);

    for command in commands.iter_mut() {
        typecheck_command(command, &mut environment)?;
    }

    Ok(commands)
}

fn populate_built_ins(environment: &mut TypeEnvironment<'_>) {
    environment.add_scope("global", "");

    // RGBA struct type
    let rgba = Type::Struct {
        name: "rgba",
        elements: vec![
            ("r", Type::Float),
            ("g", Type::Float),
            ("b", Type::Float),
            ("a", Type::Float),
        ],
    };

    // Argnum and args
    let argnum = Type::Int;
    let args = Type::Array {
        element_type: Box::new(Type::Int),
        rank: 1,
    };

    // Function types
    let one_float_to_float = Type::Function {
        param_types: vec![Type::Float],
        return_type: Box::new(Type::Float),
    };

    let two_floats_to_float = Type::Function {
        param_types: vec![Type::Float, Type::Float],
        return_type: Box::new(Type::Float),
    };

    let to_float = Type::Function {
        param_types: vec![Type::Int],
        return_type: Box::new(Type::Float),
    };

    let to_int = Type::Function {
        param_types: vec![Type::Float],
        return_type: Box::new(Type::Int),
    };

    // Add to environment
    let _ = environment.add_identifier("global", "rgba", rgba, Position::new(0, 0));
    let _ = environment.add_identifier("global", "argnum", argnum, Position::new(0, 0));
    let _ = environment.add_identifier("global", "args", args, Position::new(0, 0));

    for name in &[
        "float", "sqrt", "exp", "sin", "cos", "tan", "asin", "acos", "atan", "log",
    ] {
        let _ = environment.add_identifier(
            "global",
            name,
            one_float_to_float.clone(),
            Position::new(0, 0),
        );
    }

    for name in &["pow", "atan2"] {
        let _ = environment.add_identifier(
            "global",
            name,
            two_floats_to_float.clone(),
            Position::new(0, 0),
        );
    }

    let _ = environment.add_identifier("global", "to_float", to_float, Position::new(0, 0));
    let _ = environment.add_identifier("global", "to_int", to_int, Position::new(0, 0));
}

fn bind_lvalue<'a>(
    lvalue: &LValue<'a>,
    typ: Type<'a>,
    scope: &'a str,
    environment: &mut TypeEnvironment<'a>,
) -> Result<(), TypeError> {
    match &lvalue.node {
        LValueType::Array { indices } => {
            // Add the identifier with the type
            environment.add_identifier(scope, lvalue.name, typ.clone(), lvalue.position)?;

            // Check if the type is an array with matching rank
            if let Type::Array {
                element_type: _,
                rank,
            } = &typ
            {
                if *rank != indices.len() {
                    return Err(TypeError::new(
                        format!(
                            "Rank of array lvalue ({}) does not match right hand side ({})",
                            indices.len(),
                            rank
                        ),
                        lvalue.position,
                    ));
                }
            } else {
                return Err(TypeError::new(
                    "Incorrect type for array lvalue".to_string(),
                    lvalue.position,
                ));
            }

            // Bind index variables as integers
            for &index in indices {
                environment.add_identifier(scope, index, Type::Int, lvalue.position)?;
            }

            Ok(())
        }
        LValueType::Variable => {
            environment.add_identifier(scope, lvalue.name, typ, lvalue.position)
        }
    }
}

fn typecheck_type<'a>(
    typ: Type<'a>,
    scope: &'a str,
    environment: &mut TypeEnvironment<'a>,
    position: Position,
) -> Result<Type<'a>, TypeError> {
    match &typ {
        Type::Array { element_type, rank } => {
            return Ok(Type::Array {
                element_type: Box::new(typecheck_type(
                    element_type.as_ref().clone(),
                    scope,
                    environment,
                    position,
                )?),
                rank: *rank,
            });
        }
        Type::Struct { name, elements: _ } => {
            match environment.get_identifier(scope, position, name) {
                Ok(struct_type) => {
                    return Ok(struct_type);
                }
                Err(err) => return Err(err),
            }
        }
        Type::Function {
            param_types,
            return_type,
        } => {
            let mut resolved_param_types = Vec::new();
            for param_type in param_types {
                resolved_param_types.push(typecheck_type(
                    param_type.clone(),
                    scope,
                    environment,
                    position,
                )?);
            }
            typecheck_type(return_type.as_ref().clone(), scope, environment, position)?;
        }
        _ => {}
    }

    Ok(typ)
}

fn types_equal(t1: &Type<'_>, t2: &Type<'_>) -> bool {
    match (t1, t2) {
        (Type::Int, Type::Int)
        | (Type::Float, Type::Float)
        | (Type::Bool, Type::Bool)
        | (Type::Void, Type::Void) => true,
        (
            Type::Array {
                element_type: e1,
                rank: r1,
            },
            Type::Array {
                element_type: e2,
                rank: r2,
            },
        ) => r1 == r2 && types_equal(e1, e2),
        (
            Type::Struct {
                name: n1,
                elements: _,
            },
            Type::Struct {
                name: n2,
                elements: _,
            },
        ) => n1 == n2,
        (
            Type::Function {
                param_types: p1,
                return_type: r1,
            },
            Type::Function {
                param_types: p2,
                return_type: r2,
            },
        ) => {
            p1.len() == p2.len()
                && p1.iter().zip(p2.iter()).all(|(t1, t2)| types_equal(t1, t2))
                && types_equal(r1, r2)
        }
        _ => false,
    }
}

fn typecheck_command<'a>(
    command: &mut Command<'a>,
    environment: &mut TypeEnvironment<'a>,
) -> Result<(), TypeError> {
    match command.node.as_mut() {
        CommandType::Read {
            source: _,
            destination,
        } => {
            let rgba = environment.get_identifier("global", command.position, "rgba")?;
            let rgba_array = Type::Array {
                element_type: Box::new(rgba.clone()),
                rank: 2,
            };

            bind_lvalue(destination, rgba_array, "global", environment)?;

            if let LValueType::Array { indices } = &destination.node {
                if indices.len() != 2 {
                    return Err(TypeError::new(
                        format!(
                            "Cannot bind rgba[,] to array of dimension  {}",
                            indices.len()
                        ),
                        destination.position,
                    ));
                }
                for index in indices {
                    let _ =
                        environment.add_identifier("global", index, Type::Int, command.position);
                }
            };

            Ok(())
        }
        CommandType::Write {
            source,
            destination: _,
        } => {
            let image_type = typecheck_expression(source, "global", environment)?;

            let is_rgba_array = matches!(
                image_type,
                Type::Array {
                    element_type,
                    rank: 2,
                } if matches!(element_type.as_ref(), Type::Struct { name, .. } if *name == "rgba")
            );

            if !is_rgba_array {
                Err(TypeError::new(
                    "Expression for write needs to be rgba[,]".to_string(),
                    command.position,
                ))
            } else {
                Ok(())
            }
        }
        CommandType::Let { variable, rvalue } => {
            let rtype = typecheck_expression(rvalue, "global", environment)?;
            bind_lvalue(variable, rtype, "global", environment)
        }
        CommandType::Assert {
            condition,
            message: _,
        } => {
            let condition_type = typecheck_expression(condition, "global", environment)?;
            if !matches!(condition_type, Type::Bool) {
                return Err(TypeError::new(
                    "Asserted expression must be boolean".to_string(),
                    condition.position,
                ));
            }
            Ok(())
        }
        CommandType::Print { message: _ } => Ok(()),
        CommandType::Show { expression } => {
            typecheck_expression(expression, "global", environment)?;
            Ok(())
        }
        CommandType::Time {
            command: inner_command,
        } => typecheck_command(inner_command, environment),
        CommandType::Struct { name, elements } => {
            let mut resolved_elements = Vec::new();

            for (element_name, element_type) in elements {
                // Check if duplicate name exists inside struct
                if resolved_elements
                    .iter()
                    .any(|(name, _)| name == element_name)
                {
                    return Err(TypeError::new(
                        format!("Duplicate field name {} in struct", element_name),
                        command.position,
                    ));
                }

                let type_value = typecheck_type(
                    element_type.clone(),
                    "global",
                    environment,
                    command.position,
                )?;
                resolved_elements.push((*element_name, type_value));
            }

            // Add the struct to the environment
            environment.add_identifier(
                "global",
                name,
                Type::Struct {
                    name,
                    elements: resolved_elements,
                },
                command.position,
            )
        }
        CommandType::Function {
            name,
            parameters,
            return_type,
            statements,
            has_return,
        } => {
            // Check return type
            *return_type =
                typecheck_type(return_type.clone(), "global", environment, command.position)?;

            // Create parameter types list
            let mut param_types = Vec::new();

            environment.add_scope(name, "global");

            // Add parameters to local scope
            for (param, param_type) in parameters.iter_mut() {
                let type_value =
                    typecheck_type(param_type.clone(), "global", environment, param.position)?;
                *param_type = type_value.clone();
                param_types.push(type_value.clone());
                bind_lvalue(param, type_value, name, environment)?;
            }

            // Add function to global environment for recursive calls
            let function_type = Type::Function {
                param_types,
                return_type: Box::new(return_type.clone()),
            };

            environment.add_identifier("global", name, function_type, command.position)?;

            // Check all statements in the function
            for statement in statements.iter_mut() {
                match &mut statement.node {
                    StatementType::Let { variable, rvalue } => {
                        let rtype = typecheck_expression(rvalue, name, environment)?;
                        bind_lvalue(variable, rtype, name, environment)?;
                    }
                    StatementType::Assert {
                        condition,
                        message: _,
                    } => {
                        let cond_type = typecheck_expression(condition, name, environment)?;
                        if !types_equal(&cond_type, &Type::Bool) {
                            return Err(TypeError::new(
                                "Asserted expression must be boolean".to_string(),
                                condition.position,
                            ));
                        }
                    }
                    StatementType::Return { value } => {
                        let ret_type = typecheck_expression(value, name, environment)?;
                        if !types_equal(return_type, &ret_type) {
                            return Err(TypeError::new(
                                "Type of expression does not match return type of function"
                                    .to_string(),
                                value.position,
                            ));
                        }
                        has_return.replace(true);
                    }
                }
            }

            // Check for implicit return
            if !has_return.get() && !types_equal(return_type, &Type::Void) {
                return Err(TypeError::new(
                    "Implicit return type (void) does not match return type of function"
                        .to_string(),
                    command.position,
                ));
            };

            Ok(())
        }
    }
}

fn typecheck_expression<'a>(
    expression: &mut Expression<'a>,
    scope: &'a str,
    environment: &mut TypeEnvironment<'a>,
) -> Result<Type<'a>, TypeError> {
    match expression.node.as_mut() {
        ExpressionType::Int { value: _ } => {
            expression.resolved_type = Type::Int;
            Ok(Type::Int)
        }
        ExpressionType::Float { value: _ } => {
            expression.resolved_type = Type::Float;
            Ok(Type::Float)
        }
        ExpressionType::True => {
            expression.resolved_type = Type::Bool;
            Ok(Type::Bool)
        }
        ExpressionType::False => {
            expression.resolved_type = Type::Bool;
            Ok(Type::Bool)
        }
        ExpressionType::Void => {
            expression.resolved_type = Type::Void;
            Ok(Type::Void)
        }
        ExpressionType::Variable { name } => {
            let identifier_type = environment.get_identifier(scope, expression.position, name)?;

            // Check if it's a function type, which is not allowed for variable expressions
            if let Type::Function { .. } = identifier_type {
                return Err(TypeError::new(
                    format!("Name {} is not defined as a variable", name),
                    expression.position,
                ));
            }

            // Set and return the resolved type
            expression.resolved_type = identifier_type.clone();
            Ok(identifier_type)
        }
        ExpressionType::ArrayLiteral { elements } => {
            if elements.is_empty() {
                return Err(TypeError::new(
                    "Array literal cannot be empty".to_string(),
                    expression.position,
                ));
            }

            // Check the type of the first element
            let first_type =
                typecheck_expression(elements.get_mut(0).unwrap(), scope, environment)?;

            // Check all other elements have the same type
            for (i, element) in elements.iter_mut().enumerate().skip(1) {
                let element_type = typecheck_expression(element, scope, environment)?;

                if !types_equal(&first_type, &element_type) {
                    return Err(TypeError::new(
                         format!(
                            "All elements of array literal must have the same type, element {} differs",
                            i
                        ),
                        expression.position));
                }
            }

            // Create and return array type
            let array_type = Type::Array {
                element_type: Box::new(first_type),
                rank: 1,
            };
            expression.resolved_type = array_type.clone();
            Ok(array_type)
        }
        ExpressionType::ArrayIndex { array, indices } => {
            let array_type = typecheck_expression(array, scope, environment)?;

            if let Type::Array { element_type, rank } = array_type {
                // Check number of indices
                if rank != indices.len() {
                    return Err(TypeError::new(
                        format!(
                            "Expected {} indices for array access got {}",
                            rank,
                            indices.len()
                        ),
                        expression.position,
                    ));
                }

                // Check that all indices are integers
                for index in indices.iter_mut() {
                    let index_type = typecheck_expression(index, scope, environment)?;
                    if !types_equal(&index_type, &Type::Int) {
                        return Err(TypeError::new(
                            "Array indices must be integers".to_string(),
                            index.position,
                        ));
                    }
                }

                // Return element type
                let element_type = *element_type;
                expression.resolved_type = element_type.clone();
                Ok(element_type)
            } else {
                Err(TypeError::new(
                    "Expression is not an array and cannot be indexed".to_string(),
                    expression.position,
                ))
            }
        }
        ExpressionType::Dot {
            struct_variable,
            field,
        } => {
            let variable_type = typecheck_expression(struct_variable, scope, environment)?;
            if let Ok(Type::Struct { name: _, elements }) =
                typecheck_type(variable_type, scope, environment, struct_variable.position)
            {
                // Find the field
                for (elem_name, elem_type) in elements {
                    if elem_name == *field {
                        expression.resolved_type = elem_type.clone();
                        return Ok(elem_type);
                    }
                }

                Err(TypeError::new(
                    format!("Struct has no field named {}", field),
                    expression.position,
                ))
            } else {
                Err(TypeError::new(
                    "Expression is not a struct and cannot be indexed into".to_string(),
                    expression.position,
                ))
            }
        }
        ExpressionType::Call {
            function,
            arguments,
        } => {
            let func_type = environment.get_identifier(scope, expression.position, function)?;

            if let Type::Function {
                param_types,
                return_type,
            } = func_type
            {
                // Check number of arguments
                if param_types.len() != arguments.len() {
                    return Err(TypeError::new(
                        format!("Incorrect number of parameters for function {}", function),
                        expression.position,
                    ));
                }

                // Check each argument type
                for (i, (arg, param_type)) in
                    arguments.iter_mut().zip(param_types.iter()).enumerate()
                {
                    let arg_type = typecheck_expression(arg, scope, environment)?;
                    if !types_equal(&arg_type, param_type) {
                        return Err(TypeError::new(
                            format!(
                                "Incorrect type of parameter {} for function {}",
                                i, function
                            ),
                            arg.position,
                        ));
                    }
                }

                // Return function's return type
                let return_type = *return_type;
                expression.resolved_type = return_type.clone();
                Ok(return_type)
            } else {
                Err(TypeError::new(
                    format!("{} is not a function", function),
                    expression.position,
                ))
            }
        }
        ExpressionType::StructLiteral { name, fields } => {
            let type_value = environment.get_identifier(scope, expression.position, name)?;

            if let Type::Struct { name: _, elements } = type_value.clone() {
                // Check number of fields
                if elements.len() != fields.len() {
                    return Err(TypeError::new(
                        format!(
                            "Expected {} fields for struct literal got {}",
                            elements.len(),
                            fields.len()
                        ),
                        expression.position,
                    ));
                }

                // Check each field type
                for (i, field_expr) in fields.iter_mut().enumerate() {
                    let expr_type = typecheck_expression(field_expr, scope, environment)?;

                    let struct_field = &elements[i];
                    let field_type = typecheck_type(
                        struct_field.1.clone(),
                        scope,
                        environment,
                        field_expr.position,
                    )?;

                    if !types_equal(&expr_type, &field_type) {
                        return Err(TypeError::new(
                            format!(
                                "Invalid type of element {} for type {}",
                                struct_field.0, name
                            ),
                            expression.position,
                        ));
                    }
                }

                expression.resolved_type = type_value.clone();
                Ok(type_value)
            } else {
                Err(TypeError::new(
                    format!("{} is not a struct type", name),
                    expression.position,
                ))
            }
        }
        ExpressionType::Unop {
            operator: _,
            expression: expr,
        } => {
            // Simply check the expression and return its type
            let typ = typecheck_expression(expr, scope, environment)?;
            expression.resolved_type = typ.clone();
            Ok(typ)
        }
        ExpressionType::Binop {
            operator,
            left,
            right,
        } => {
            let left_type = typecheck_expression(left, scope, environment)?;
            let right_type = typecheck_expression(right, scope, environment)?;

            // Check if left and right have the same type
            if !types_equal(&left_type, &right_type) {
                return Err(TypeError::new(
                    "Left and right hand operators need to have the same type".to_string(),
                    expression.position,
                ));
            }

            // Check required input types
            let requires_numeric = matches!(
                *operator,
                "+" | "-" | "*" | "/" | "%" | "<" | ">" | "<=" | ">="
            );
            let requires_boolean = matches!(*operator, "&&" | "||");
            let requires_comparable = matches!(*operator, "==" | "!=");

            if requires_numeric && !matches!(left_type, Type::Int | Type::Float) {
                return Err(TypeError::new(
                    "Left and right hand operators need to be int or float".to_string(),
                    expression.position,
                ));
            } else if requires_boolean && !matches!(left_type, Type::Bool) {
                return Err(TypeError::new(
                    "Left and right hand operators need to be bool".to_string(),
                    expression.position,
                ));
            } else if requires_comparable
                && !matches!(left_type, Type::Int | Type::Float | Type::Bool)
            {
                return Err(TypeError::new(
                    "Left and right hand operators need to be int, float or bool".to_string(),
                    expression.position,
                ));
            }

            // Set the result type
            let returns_bool = matches!(
                *operator,
                "==" | "!=" | "<" | ">" | "<=" | ">=" | "&&" | "||"
            );

            let result_type = if returns_bool { Type::Bool } else { left_type };

            expression.resolved_type = result_type.clone();
            Ok(result_type)
        }
        ExpressionType::If {
            condition,
            then_branch,
            else_branch,
        } => {
            // Check that condition is boolean
            let condition_type = typecheck_expression(condition, scope, environment)?;
            if !types_equal(&condition_type, &Type::Bool) {
                return Err(TypeError::new(
                    "Type of condition for if expression must be boolean".to_string(),
                    expression.position,
                ));
            }

            // Check that both branches have the same type
            let then_type = typecheck_expression(then_branch, scope, environment)?;
            let else_type = typecheck_expression(else_branch, scope, environment)?;

            if !types_equal(&then_type, &else_type) {
                return Err(TypeError::new(
                    "Type of both branches in if expression must match".to_string(),
                    expression.position,
                ));
            }

            expression.resolved_type = then_type.clone();
            Ok(then_type)
        }
        ExpressionType::ArrayLoop { range, body } => {
            let rank = range.len();
            if rank == 0 {
                return Err(TypeError::new(
                    "Cannot make array of rank 0".to_string(),
                    expression.position,
                ));
            }

            let loop_scope = range.first().unwrap().0;
            let mut loop_vars = Vec::new();

            // Add loop variables to scope
            for (variable, bound) in range {
                let limit_type = typecheck_expression(bound, scope, environment)?;

                if !types_equal(&limit_type, &Type::Int) {
                    return Err(TypeError::new(
                        "Cannot iterate to non-integer value".to_string(),
                        bound.position,
                    ));
                }

                loop_vars.push((variable, bound.position));
            }

            // Create a new environment for loop variables
            environment.add_scope(loop_scope, scope);

            for var in loop_vars {
                environment.add_identifier(loop_scope, var.0, Type::Int, var.1)?;
            }

            // Type check the body
            let body_type = typecheck_expression(body, loop_scope, environment)?;

            // Remove the loop variables from the environment
            environment.remove_scope(loop_scope);

            // Create array type
            let array_type = Type::Array {
                element_type: Box::new(body_type),
                rank,
            };

            expression.resolved_type = array_type.clone();
            Ok(array_type)
        }
        ExpressionType::SumLoop { range, body } => {
            if range.is_empty() {
                return Err(TypeError::new(
                    "Cannot sum over array of rank 0".to_string(),
                    expression.position,
                ));
            }

            // Create a new environment for loop variables
            let loop_scope = range.first().unwrap().0;
            let mut loop_vars = Vec::new();

            // Add loop variables to scope
            for (variable, bound) in range {
                let limit_type = typecheck_expression(bound, scope, environment)?;

                if !types_equal(&limit_type, &Type::Int) {
                    return Err(TypeError::new(
                        "Cannot iterate to non-integer value".to_string(),
                        bound.position,
                    ));
                }

                loop_vars.push((variable, bound.position));
            }

            // Create a new environment for loop variables
            environment.add_scope(loop_scope, scope);

            for var in loop_vars {
                environment.add_identifier(loop_scope, var.0, Type::Int, var.1)?;
            }

            // Type check the body
            let body_type = typecheck_expression(body, loop_scope, environment)?;

            // Remove the loop variables from the environment
            environment.remove_scope(loop_scope);

            // Check if body type is numeric (int or float)
            if !matches!(body_type, Type::Int | Type::Float) {
                return Err(TypeError::new(
                    "Cannot sum over elements that are not int or float".to_string(),
                    body.position,
                ));
            }

            // Return element type (int or float)
            expression.resolved_type = body_type.clone();
            Ok(body_type)
        }
    }
}
