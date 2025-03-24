use crate::ast::*;
use crate::utils::*;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Display;

struct AssemblyFunction<'a> {
    name: &'a str,
    code: Vec<String>,
}

impl Display for AssemblyFunction<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut result = format!("{}:\n", self.name);
        result.push_str(&format!("_{}:\n", self.name)); // Double print function name for macos reasons

        for line in &self.code {
            result.push_str(&format!("{}\n", line));
        }

        write!(f, "{}", result)
    }
}

impl<'a> AssemblyFunction<'a> {
    fn new(name: &'a str) -> Self {
        Self {
            name,
            code: Vec::new(),
        }
    }

    fn add_line(&mut self, line: String) {
        self.code.push(line);
    }
}

/// Very much not sure about this
#[derive(Eq, Hash, PartialEq, Clone)]
enum AssemblyValue {
    Number(String),
    String(String),
}

impl Display for AssemblyValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssemblyValue::Number(value) => write!(f, "dq {}", value),
            AssemblyValue::String(value) => write!(f, "db `{}`, 0", value),
        }
    }
}

struct AssemblyGenerator<'a> {
    data_section: Vec<AssemblyValue>,
    functions: Vec<AssemblyFunction<'a>>,
    constants: HashMap<AssemblyValue, String>,
    jump_counter: usize,
}

impl Display for AssemblyGenerator<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut result = "section .data\n".to_string();
        for (const_counter, value) in self.data_section.iter().enumerate() {
            result.push_str(&format!("const{}: {}\n", const_counter, value));
        }

        result.push_str("\nsection .text\n");
        for function in &self.functions {
            result.push_str(&format!("{}\n\n", function));
        }

        write!(f, "{}", result)
    }
}

impl<'a> AssemblyGenerator<'a> {
    pub fn new() -> Self {
        Self {
            data_section: Vec::new(),
            functions: Vec::new(),
            constants: HashMap::new(),
            jump_counter: 0,
        }
    }

    pub fn generate_assembly(
        &mut self,
        commands: Vec<Command<'a>>,
        environment: TypeEnvironment<'a>,
    ) -> String {
        let mut main_function = AssemblyFunction::new("jpl_main");

        // jpl_main prelude
        main_function.add_line("\t; jpl_main prelude".to_string());
        main_function.add_line("\tpush rbp".to_string());
        main_function.add_line("\tmov rbp, rsp".to_string());
        main_function.add_line("\tpush r12".to_string());
        main_function.add_line("\tmov r12, rbp\n".to_string());

        for command in commands {
            self.generate_command(&mut main_function, &command, &environment);
        }

        // jpl_main postlude
        main_function
            .code
            .push("\n\t; jpl_main postlude".to_string());
        main_function.add_line("\tpop r12".to_string());
        main_function.add_line("\tpop rbp".to_string());
        main_function.add_line("\tret".to_string());

        format!(
            "global jpl_main\nglobal _jpl_main\nextern _fail_assertion\nextern _jpl_alloc\nextern _get_time\nextern _show\nextern _print\nextern _print_time\nextern _read_image\nextern _write_image\nextern _fmod\nextern _sqrt\nextern _exp\nextern _sin\nextern _cos\nextern _tan\nextern _asin\nextern _acos\nextern _atan\nextern _log\nextern _pow\nextern _atan2\nextern _to_int\nextern _to_float\n\n{}{}",
            self,
            main_function
        )
    }

    fn generate_command(
        &mut self,
        function: &mut AssemblyFunction<'a>,
        command: &Command<'a>,
        environment: &TypeEnvironment<'a>,
    ) {
        match command.node.as_ref() {
            CommandType::Show { expression } => {
                let (size, typ_str) = self.generate_expression(function, expression, environment);
                let const_name = self.get_constant(AssemblyValue::String(typ_str));

                function.add_line(format!("\tlea rdi, [rel {}]", const_name));
                function.add_line("\tlea rsi, [rsp]".to_string());
                function.add_line("\tcall _show".to_string());
                function.add_line(format!("\tadd rsp, {}", size));
            }
            CommandType::Function {
                name,
                parameters,
                return_type,
                statements,
                has_return,
            } => {
                let mut function = AssemblyFunction::new(name);

                // Function prelude
                function.add_line(format!("\t; {} prelude", name));
                function.add_line("\tpush rbp".to_string());
                function.add_line("\tmov rbp, rsp".to_string());

                for statement in statements {
                    // self.generate_statement(statement, environment);
                }

                // Function postlude
                function.add_line(format!("\n\t; {} postlude", name));
                function.add_line("\tpop rbp".to_string());
                function.add_line("\tret".to_string());

                self.functions.push(function);
            }
            _ => {}
        }
    }

    /// Generate assembly code for an expression
    /// Location of generated expression is always rax
    /// Returns size of expression in bytes
    fn generate_expression(
        &mut self,
        function: &mut AssemblyFunction<'a>,
        expression: &Expression<'a>,
        environment: &TypeEnvironment<'a>,
    ) -> (usize, String) {
        match expression.node.as_ref() {
            ExpressionType::Int { value } => {
                let constant = self.get_constant(AssemblyValue::Number(value.to_string()));
                function.add_line(format!("\tmov rax, [rel {}]", constant));
                function.add_line("\tpush rax".to_string());
                (8, "(IntType)".to_string())
            }
            ExpressionType::Float { value } => {
                // Format the float to ensure decimal point is always shown
                let formatted_value = if value.fract() == 0.0 {
                    format!("{:.1}", value) // Ensure at least one decimal place for whole numbers
                } else {
                    format!("{}", value) // Use default formatting for non-integer values
                };
                let constant = self.get_constant(AssemblyValue::Number(formatted_value));

                function.add_line(format!("\tmov rax, [rel {}]", constant));
                function.add_line("\tpush rax".to_string());
                (8, "(FloatType)".to_string())
            }
            ExpressionType::True => {
                let constant = self.get_constant(AssemblyValue::Number("1".to_string()));
                function.add_line(format!("\tmov rax, [rel {}]", constant));
                function.add_line("\tpush rax".to_string());
                (8, "(BoolType)".to_string())
            }
            ExpressionType::False => {
                let constant = self.get_constant(AssemblyValue::Number("0".to_string()));
                function.add_line(format!("\tmov rax, [rel {}]", constant));
                function.add_line("\tpush rax".to_string());
                (8, "(BoolType)".to_string())
            }
            ExpressionType::Unop {
                operator,
                expression,
            } => {
                let (size, typ_str) = self.generate_expression(function, expression, environment);

                match typ_str.as_str() {
                    "(IntType)" => {
                        function.add_line("\tpop rax".to_string());
                        function.add_line("\tneg rax".to_string());
                        function.add_line("\tpush rax".to_string());
                    }
                    "(FloatType)" => {
                        function.add_line("\tmovsd xmm1, [rsp]".to_string());
                        function.add_line("\tadd rsp, 8".to_string());
                        function.add_line("\tpxor xmm0, xmm0".to_string());
                        function.add_line("\tsubsd xmm0, xmm1".to_string());
                        function.add_line("\tsub rsp, 8".to_string());
                        function.add_line("\tmovsd [rsp], xmm0".to_string());
                    }
                    "(BoolType)" => {
                        function.add_line("\tpop rax".to_string());
                        function.add_line("\txor rax, 1".to_string());
                        function.add_line("\tpush rax".to_string());
                    }
                    _ => panic!("Unknown unary operator {}", operator),
                }

                (size, typ_str)
            }
            _ => panic!("Matched on unknown expression type {}", expression),
        }
    }

    // TODO entry api
    fn get_constant(&mut self, value: AssemblyValue) -> String {
        if !self.constants.contains_key(&value) {
            self.constants
                .insert(value.clone(), format!("const{}", self.data_section.len()));
            self.data_section.push(value.clone());
        }
        self.constants[&value].clone()
    }
}

pub fn generate_assembly<'a>(
    commands: Vec<Command<'a>>,
    environment: TypeEnvironment<'a>,
) -> Result<String, ()> {
    let mut generator = AssemblyGenerator::new();
    Ok(generator.generate_assembly(commands, environment))
}
