use crate::ast::*;
use crate::utils::*;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Display;

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

struct AssemblyGenerator {
    data_section: Vec<AssemblyValue>,
    functions: Vec<String>,
    constants: HashMap<AssemblyValue, String>,
    jump_counter: usize,
}

impl Display for AssemblyGenerator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut result = "\n\n\nsection .data".to_string();
        for (const_counter, value) in self.data_section.iter().enumerate() {
            result.push_str(&format!("\nconst{}: {}", const_counter, value));
        }

        result.push_str("\n\n\nsection .text");
        for function in &self.functions {
            result.push_str(&format!("{}\n\n", function));
        }

        write!(f, "{}", result)
    }
}

impl<'a> AssemblyGenerator {
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
        let mut main_function = String::new();
        // jpl_main prelude
        main_function.push_str("\n\njpl_main:\n_jpl_main:\n\t; jpl_main prelude\n\tpush rbp\n\tmov rbp, rsp\n\tpush r12\n\tmov r12, rbp");

        for command in commands {
            self.generate_command(&mut main_function, &command, &environment);
        }

        // jpl_main postlude
        main_function.push_str("\n\n\t; jpl_main postlude\n\tpop r12\n\tpop rbp\n\tret");

        format!(
            "global jpl_main\nglobal _jpl_main\nextern _fail_assertion\nextern _jpl_alloc\nextern _get_time\nextern _show\nextern _print\nextern _print_time\nextern _read_image\nextern _write_image\nextern _fmod\nextern _sqrt\nextern _exp\nextern _sin\nextern _cos\nextern _tan\nextern _asin\nextern _acos\nextern _atan\nextern _log\nextern _pow\nextern _atan2\nextern _to_int\nextern _to_float{}{}",
            self,
            main_function
        )
    }

    fn generate_command(
        &mut self,
        function: &mut String,
        command: &Command<'a>,
        environment: &TypeEnvironment<'a>,
    ) {
        match command.node.as_ref() {
            CommandType::Show { expression } => {
                function.push_str("\n\n\t; Show command");

                let expr_result = self.generate_expression(function, expression, environment);
                let size = expr_result.0;
                let typ_str = expr_result.1.to_string();
                let const_name = self.get_constant(AssemblyValue::String(typ_str));

                function.push_str(&format!("\n\tlea rdi, [rel {}]", const_name));
                function.push_str("\n\tlea rsi, [rsp]");
                function.push_str("\n\tcall _show");
                function.push_str(&format!("\n\tadd rsp, {}", size));
            }
            // CommandType::Function {
            //     name,
            //     parameters,
            //     return_type,
            //     statements,
            //     has_return,
            // } => {
            //     let mut function = String::new();
            //     function.push_str(&format!("\n{}:\n_{}:", name, name));

            //     // Function prelude
            //     function.push_str(&format!("\n\t; {} prelude", name));
            //     function.push_str("\n\tpush rbp");
            //     function.push_str("\n\tmov rbp, rsp");

            //     for statement in statements {
            //         // self.generate_statement(statement, environment);
            //     }

            //     // Function postlude
            //     function.push_str(&format!("\n\t; {} postlude", name));
            //     function.push_str("\n\tpop rbp");
            //     function.push_str("\n\tret");

            //     self.functions.push(function);
            // }
            _ => {}
        }
    }

    /// Generate assembly code for an expression
    /// Location of generated expression is always rax
    /// Returns size of expression in bytes
    fn generate_expression(
        &mut self,
        function: &mut String,
        expression: &Expression<'a>,
        environment: &TypeEnvironment<'a>,
    ) -> (usize, Type) {
        match expression.node.as_ref() {
            ExpressionType::Int { value } => {
                let constant = self.get_constant(AssemblyValue::Number(value.to_string()));
                function.push_str(&format!("\n\tmov rax, [rel {}]", constant));
                function.push_str("\n\tpush rax");
                (8, Type::Int)
            }
            ExpressionType::Float { value } => {
                // Format the float to ensure decimal point is always shown
                let formatted_value = if value.fract() == 0.0 {
                    format!("{:.1}", value) // Ensure at least one decimal place for whole numbers
                } else {
                    format!("{}", value) // Use default formatting for non-integer values
                };
                let constant = self.get_constant(AssemblyValue::Number(formatted_value));

                function.push_str(&format!("\n\tmov rax, [rel {}]", constant));
                function.push_str("\n\tpush rax");
                (8, Type::Float)
            }
            ExpressionType::True => {
                let constant = self.get_constant(AssemblyValue::Number("1".to_string()));
                function.push_str(&format!("\n\tmov rax, [rel {}]", constant));
                function.push_str("\n\tpush rax");
                (8, Type::Bool)
            }
            ExpressionType::False => {
                let constant = self.get_constant(AssemblyValue::Number("0".to_string()));
                function.push_str(&format!("\n\tmov rax, [rel {}]", constant));
                function.push_str("\n\tpush rax");
                (8, Type::Bool)
            }
            ExpressionType::Unop {
                operator: _,
                expression,
            } => {
                let (size, typ) = self.generate_expression(function, expression, environment);

                match typ {
                    Type::Int => {
                        function.push_str("\n\tpop rax");
                        function.push_str("\n\tneg rax");
                        function.push_str("\n\tpush rax");
                    }
                    Type::Float => {
                        function.push_str("\n\tmovsd xmm1, [rsp]");
                        function.push_str("\n\tadd rsp, 8");
                        function.push_str("\n\tpxor xmm0, xmm0");
                        function.push_str("\n\tsubsd xmm0, xmm1");
                        function.push_str("\n\tsub rsp, 8");
                        function.push_str("\n\tmovsd [rsp], xmm0");
                    }
                    Type::Bool => {
                        function.push_str("\n\tpop rax");
                        function.push_str("\n\txor rax, 1");
                        function.push_str("\n\tpush rax");
                    }
                    _ => unreachable!(),
                }

                (size, typ)
            }
            ExpressionType::Binop {
                operator,
                left,
                right,
            } => {
                let _ = self.generate_expression(function, right, environment);
                let (_, left_typ) = self.generate_expression(function, left, environment);

                match left_typ {
                    Type::Int => Self::generate_int_op(function, operator),
                    Type::Float => Self::generate_float_op(function, operator),
                    Type::Bool => Self::generate_bool_op(function, operator),
                    _ => unreachable!(),
                }
            }
            ExpressionType::ArrayLiteral { elements } => {
                unimplemented!();
            }
            _ => unimplemented!(),
        }
    }

    fn generate_int_op(function: &mut String, operator: &str) -> (usize, Type<'a>) {
        match operator {
            "+" => function.push_str("\n\tpop rax\n\tpop r10\n\tadd rax, r10\n\tpush rax"),
            "-" => function.push_str("\n\tpop rax\n\tpop r10\n\tsub rax, r10\n\tpush rax"),
            "*" => function.push_str("\n\tpop rax\n\tpop r10\n\timul rax, r10\n\tpush rax"),
            "/" | "%" => {
                // TODO: Add division by zero handling
            }
            _ => unreachable!(),
        }
        (8, Type::Int)
    }

    fn generate_float_op(function: &mut String, operator: &str) -> (usize, Type<'a>) {
        match operator {
            "+" | "-" | "*" | "/" => {
                let op_instruction = match operator {
                    "+" => "addsd",
                    "-" => "subsd",
                    "*" => "mulsd",
                    "/" => "divsd",
                    _ => unreachable!(),
                };

                function.push_str(&format!(
                    "\n\tmovsd xmm0, [rsp]\n\tadd rsp, 8\n\tmovsd xmm1, [rsp]\n\tadd rsp, 8\
                    \n\t{} xmm0, xmm1\
                    \n\tsub rsp, 8\
                    \n\tmovsd [rsp], xmm0",
                    op_instruction
                ));
            }
            "%" => {
                // TODO:  call _fmod
            }
            _ => unreachable!(),
        }
        (8, Type::Float)
    }

    fn generate_bool_op(function: &mut String, operator: &str) -> (usize, Type<'a>) {
        match operator {
            "&&" => function.push_str("\n\tpop rax\n\tpop r10\n\tand rax, r10\n\tpush rax"),
            "||" => function.push_str("\n\tpop rax\n\tpop r10\n\tor rax, r10\n\tpush rax"),
            _ => unreachable!(),
        }
        (8, Type::Bool)
    }

    fn get_constant(&mut self, value: AssemblyValue) -> &str {
        let data_section_len = self.data_section.len();
        let entry = self.constants.entry(value.clone()).or_insert_with(|| {
            self.data_section.push(value.clone());
            format!("const{}", data_section_len)
        });
        entry
    }
}

pub fn generate_assembly<'a>(
    commands: Vec<Command<'a>>,
    environment: TypeEnvironment<'a>,
) -> Result<String, ()> {
    let mut generator = AssemblyGenerator::new();
    Ok(generator.generate_assembly(commands, environment))
}
