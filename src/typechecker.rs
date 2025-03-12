use crate::ast::*;
use std::{collections::HashMap, fmt::Display};

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
    global_env: HashMap<&'a str, Type<'a>>,
}

impl<'a> TypeChecker<'a> {
    fn new(commands: &'a Vec<Command<'a>>) -> TypeChecker<'a> {
        TypeChecker {
            commands,
            global_env: HashMap::new(),
        }
    }

    fn typecheck(&self) -> Result<(), TypeCheckerError> {
        for command in self.commands {
            self.typecheck_command(command)?;
        }

        Ok(())
    }

    // ----------------------------------------------------------------------------------- Command Typecheckers ----------------------------------------------------------------------------------------------

    fn typecheck_command(&self, command: &Command<'a>) -> Result<(), TypeCheckerError> {
        match command.node {
            CommandType::Read {
                source,
                destination,
            } => self.typecheck_read_command(source, destination),
            // Command::Read {
            //     source,
            //     destination,
            // } => self.typecheck_read_command(source, destination),
            // Command::Write(source) => self.typecheck_write_command(source),
            // Command::Let(lvalue, expr) => self.typecheck_let_command(lvalue, expr),
            // Command::Print(expr) => self.typecheck_print_command(expr),
            // Command::Assert(expr) => self.typecheck_assert_command(expr),
            // Command::Time(cmd) => self.typecheck_command(cmd),
            // Command::Struct(name, elements) => {
            //     self.typecheck_struct_command(name, elements, command.position())
            // }
            // Command::Function(name, params, return_type, body) => {
            //     self.typecheck_function_command(name, params, return_type, body, command.position())
            // }
            // _ => Err(TypeCheckerError {
            //     message: "Unsupported command type".to_string(),
            //     position: command.position(),
            // }),
        }
    }

    // Placeholder implementations for other commands
    fn typecheck_read_command(
        &self,
        source: &'a str,
        dest: Box<LValue<'a>>,
    ) -> Result<(), TypeCheckerError> {
        // Create rgba array type (2D array of rgba values)
        let rgba_type = Type::Primitive("rgba");
        let rgba_array_type = Type::Array(Box::new(rgba_type), 2);
        
        // Try to add the destination to the environment with the rgba array type
        if let LValueType::Variable { name } = &dest.node {
            if self.global_env.contains_key(name) {
                return Err(TypeCheckerError {
                    message: format!("Cannot redefine the name {} as rgba[,]", name),
                    position: dest.position,
                });
            }
            self.global_env.insert(name, rgba_array_type);
        } else if let LValueType::Array { elements } = &dest.node {
            // Check array has exactly 2 dimensions
            if elements.len() != 2 {
                return Err(TypeCheckerError {
                    message: format!("Cannot bind rgba[,] to array of dimension {}", elements.len()),
                    position: dest.position,
                });
            }
            
            // Add integer type for each element variable
            for element in elements {
                if let LValueType::Variable { name } = &element.node {
                    self.global_env.insert(name, Type::Int);
                }
            }
        }
        
        Ok(())
    }

  
}

pub fn typecheck<'a>(commands: &'a Vec<Command<'a>>) -> Result<(), TypeCheckerError> {
    let typechecker = TypeChecker::new(commands);
    typechecker.typecheck()
}
