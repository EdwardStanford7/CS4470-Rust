use crate::{ast::*, typechecker::TypeEnvironment};
use std::{cell::RefCell, collections::HashMap, fmt::Display, rc::Rc};

struct CFunction<'a> {
    name: &'a str,
    code: String,
    name_counter: usize,
    jpl_to_c: HashMap<&'a str, &'a str>,
}

impl Display for CFunction<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.code)
    }
}

impl<'a> CFunction<'a> {
    fn new(name: &'a str) -> CFunction<'a> {
        CFunction {
            name,
            code: String::new(),
            name_counter: 0,
            jpl_to_c: HashMap::new(),
        }
    }

    fn add_line(&mut self, line: &str) {
        self.code.push_str(line);
        self.code.push('\n');
    }

    fn gen_sym(&mut self) -> String {
        let sym = format!("{}{}", self.name, self.name_counter);
        self.name_counter += 1;
        sym
    }

    fn add_mapping(&mut self, jpl_name: &'a str, c_name: &'a str) {
        self.jpl_to_c.insert(jpl_name, c_name);
    }

    fn get_mapping(&self, jpl_name: &'a str) -> Option<&'a str> {
        self.jpl_to_c.get(jpl_name).copied()
    }
}

struct CGenerator<'a> {
    global_env: &'a Rc<RefCell<TypeEnvironment<'a>>>,
    commands: &'a Vec<Command<'a>>,
    functions: Vec<CFunction<'a>>,
}

impl<'a> CGenerator<'a> {
    fn new(
        global_env: &'a Rc<RefCell<TypeEnvironment<'a>>>,
        commands: &'a Vec<Command<'a>>,
    ) -> CGenerator<'a> {
        CGenerator {
            global_env,
            commands,
            functions: Vec::new(),
        }
    }

    fn generate_code(&self) -> String {
        let mut code = String::new();

        for command in self.commands {
            code.push_str(&self.generate_command(command));
        }

        code
    }

    fn generate_command(&self, command: &Command<'a>) -> String {
        match command {
            _ => todo!(),
        }
    }
}

pub fn generate_code<'a>(
    global_env: &'a Rc<RefCell<TypeEnvironment<'a>>>,
    commands: &'a Vec<Command<'a>>,
) -> String {
    let generator = CGenerator::new(global_env, commands);
    generator.generate_code()
}
