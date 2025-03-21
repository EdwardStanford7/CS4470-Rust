use crate::ast::*;
use crate::utils::*;

struct AssemblyGenerator<'a> {
    commands: Vec<Command<'a>>,
    environment: TypeEnvironment<'a>,
}

impl<'a> AssemblyGenerator<'a> {
    pub fn new(commands: Vec<Command<'a>>, environment: TypeEnvironment<'a>) -> Self {
        Self {
            commands,
            environment,
        }
    }

    pub fn generate_assembly(&self) -> String {
        let mut assembly = String::new();

        for command in &self.commands {
            match command {
                _ => {}
            }
        }

        assembly
    }
}

pub fn generate_assembly<'a>(
    commands: Vec<Command<'a>>,
    environment: TypeEnvironment<'a>,
) -> Result<String, ()> {
    let generator = AssemblyGenerator::new(commands, environment);
    Ok(generator.generate_assembly())
}
