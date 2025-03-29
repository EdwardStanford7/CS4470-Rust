use crate::ast::*;
use crate::utils::*;
use std::collections::HashMap;
use std::collections::VecDeque;
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

struct AssemblyGenerator<'a> {
    data_section: Vec<AssemblyValue>,
    functions: Vec<String>,
    constants: HashMap<AssemblyValue, String>,
    jump_counter: usize,
    // stack_size: usize,
    // padding: Vec<usize>,
    offsets: HashMap<String, ((usize, Type<'a>), usize)>,
    shadow_stack: VecDeque<(usize,bool,Option<Type<'a>>)>
}

impl<'a> Display for AssemblyGenerator<'a> {
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

impl<'a> AssemblyGenerator<'a> {
    pub fn new() -> Self {
        Self {
            data_section: Vec::new(),
            functions: Vec::new(),
            constants: HashMap::new(),
            offsets: HashMap::new(),
            jump_counter: 1,
            // stack_size: 0,
            // padding: Vec::new(),
            shadow_stack: VecDeque::new()
        }
    }

    pub fn generate_assembly(
        &mut self,
        commands: Vec<Command<'a>>,
        environment: TypeEnvironment<'a>,
    ) -> String {
        let mut main_function = String::new();
        // jpl_main prelude
        main_function.push_str("\n\njpl_main:\n_jpl_main:\n\tpush rbp\n\tmov rbp, rsp\n\tpush r12\n\tmov r12, rbp ;end of jpl_main prelude");
        self.shadow_stack.push_back((16,false,None));
        // self.stack_size += 16;

        for command in commands {
            self.generate_command(&mut main_function, &command, &environment);
            self.print_stack_size(&mut main_function);
            // assert!(self.stack_size() >= 16, 
            //     "\n failure due to \n{}\n\n{} < 16", command.to_string(), self.stack_size());
        }

        if self.stack_size() > 16 {
            main_function.push_str(&format!(
                "\n\tadd rsp, {};Deallocate local variables",
                self.stack_size() - 16
            ));
        }

        // jpl_main postlude
        main_function.push_str("\n\n\tpop r12 ;begin jpl_main postlude\n\tpop rbp\n\tret");

        format!(
            "global jpl_main\nglobal _jpl_main\nextern _fail_assertion\nextern _jpl_alloc\nextern _get_time\nextern _show\nextern _print\nextern _print_time\nextern _read_image\nextern _write_image\nextern _fmod\nextern _sqrt\nextern _exp\nextern _sin\nextern _cos\nextern _tan\nextern _asin\nextern _acos\nextern _atan\nextern _log\nextern _pow\nextern _atan2\nextern _to_int\nextern _to_float{}{}",
            self,
            main_function
        )
    }

    fn stack_size(&self) -> usize {
        let mut sum = 0;
        for (e,_,_) in self.shadow_stack.iter(){
            sum += e;
        }
        return sum;
    }

    fn generate_command(
        &mut self,
        function: &mut String,
        command: &Command<'a>,
        environment: &TypeEnvironment<'a>,
    ) {
        match command.node.as_ref() {
            CommandType::Show { expression } => {
                function.push_str("\n\t;SHOW start\t\t\t\t--- C");
                self.print_stack_size(function);
                self.check_add_alignment_with(function, &expression.resolved_type);
                self.print_stack_size(function);
                let expr_result = self.generate_expression(function, expression, environment);
                self.print_stack_size(function);
                let size = expr_result.0;
                let typ_str = expr_result.1.to_string();
                let const_name = self.get_constant(AssemblyValue::String(typ_str));
                function.push_str(&format!("\n\tlea rdi, [rel {}]", const_name));
                self.shadow_stack.push_back((8,false,None));
                function.push_str("\n\tlea rsi, [rsp]");
                self.print_stack_size(function);
                function.push_str("\n\tcall _show");
                self.print_stack_size(function);
                self.shadow_stack.pop_back();
                function.push_str(&format!("\n\tadd rsp, {}", size));
                self.print_stack_size(function);
                self.check_remove_alignment(function);
                self.print_stack_size(function);
                self.shadow_stack.pop_back();
                self.print_stack_size(function);
                function.push_str("\n\t;SHOW end\t\t\t\t--- C");
            }
            CommandType::Let { variable, rvalue} => {
                function.push_str("\n\t;LET start\t\t\t\t--- C");
                let res = self.generate_expression(function, rvalue, environment);
                self.offsets.insert(variable.name.to_string(), (res,self.stack_size()));
                function.push_str("\n\t;LET end\t\t\t\t--- C");
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
            //     function.push_str(&format!("\n\t;{} prelude", name));
            //     function.push_str("\n\tpush rbp");
            //     function.push_str("\n\tmov rbp, rsp");

            //     for statement in statements {
            //         // self.generate_statement(statement, environment);
            //     }

            //     // Function postlude
            //     function.push_str(&format!("\n\t;{} postlude", name));
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
    ) -> (usize, Type<'a>) {
        match expression.node.as_ref() {
            ExpressionType::Int { value } => {
                let s_height = self.shadow_stack.len();
                function.push_str("\n\t;int const start\t\t\t--- E");
                self.print_stack_size(function);
                let startstack = self.stack_size();
                let constant = self.get_constant(AssemblyValue::Number(value.to_string()));
                function.push_str(&format!("\n\tmov rax, [rel {}]", constant));
                function.push_str("\n\tpush rax");
                self.shadow_stack.push_back((8,false,Some(expression.resolved_type.clone())));
                assert!(startstack == self.stack_size() - 8);
                self.print_stack_size(function);
                function.push_str("\n\t;int const end\t\t\t\t--- E");
                assert!(s_height + 1 == self.shadow_stack.len());
                (8, Type::Int)
            }
            ExpressionType::Float { value } => {
                let s_height = self.shadow_stack.len();
                function.push_str("\n\t;float const start\t\t\t--- E");
                let startstack = self.stack_size();
                let constant = self.get_constant(AssemblyValue::Number(format!("{:?}", value)));
                function.push_str(&format!("\n\tmov rax, [rel {}]", constant));
                function.push_str("\n\tpush rax");
                self.shadow_stack.push_back((8,false,Some(expression.resolved_type.clone())));
                assert!(startstack == self.stack_size() - 8);
                function.push_str("\n\t;float const end\t\t\t--- E");
                assert!(s_height + 1 == self.shadow_stack.len());
                (8, Type::Float)
            }
            ExpressionType::True => {
                let s_height = self.shadow_stack.len();
                function.push_str("\n\t;true const start\t\t\t--- E");
                let constant = self.get_constant(AssemblyValue::Number("1".to_string()));
                function.push_str(&format!("\n\tmov rax, [rel {}]", constant));
                function.push_str("\n\tpush rax");
                self.shadow_stack.push_back((8,false,Some(expression.resolved_type.clone())));
                function.push_str("\n\t;true const end\t\t\t--- E");
                assert!(s_height + 1 == self.shadow_stack.len());
                (8, Type::Bool)
            }
            ExpressionType::False => {
                let s_height = self.shadow_stack.len();
                function.push_str("\n\t;false const start\t\t\t--- E");
                let constant = self.get_constant(AssemblyValue::Number("0".to_string()));
                function.push_str(&format!("\n\tmov rax, [rel {}]", constant));
                function.push_str("\n\tpush rax");
                self.shadow_stack.push_back((8,false,Some(expression.resolved_type.clone())));
                function.push_str("\n\t;int const end\t\t\t--- E");
                assert!(s_height + 1 == self.shadow_stack.len());
                (8, Type::Bool)
            }
            ExpressionType::Unop {
                operator: _,
                expression,
            } => {
                let s_height = self.shadow_stack.len();
                function.push_str("\n\t;unop start\t\t\t--- E");
                let startstack = self.stack_size();
                let (size, typ) = self.generate_expression(function, expression, environment);
                assert!(startstack == self.stack_size() - 8);

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
                assert!(startstack == self.stack_size() - 8);
                function.push_str("\n\t;unop end\t\t\t\t--- E");
                assert!(s_height + 1 == self.shadow_stack.len());
                (size, typ)
            }
            ExpressionType::Binop {
                operator,
                left,
                right,
            } => {
                let s_height = self.shadow_stack.len();
                function.push_str("\n\t;binop start\t\t\t\t--- E");
                if *operator == "%" && matches!(left.resolved_type, Type::Float) {
                    function.push_str("\n\t;fmod alignment add");
                    self.check_add_alignment(function);
                }
                let alignedstack = self.stack_size();

                assert!(alignedstack == self.stack_size());
                self.generate_expression(function, right, environment);
                assert!(alignedstack == self.stack_size() - 8, "\n{}", right.to_string());
                let (_, left_type) = self.generate_expression(function, left, environment);

                assert!(alignedstack == self.stack_size() - 16, "\n{}\n{}", right.to_string(), left.to_string());
                let result = match expression.resolved_type {
                    Type::Int => self.generate_int_op(function, operator),
                    Type::Float => self.generate_float_op(function, operator),
                    Type::Bool => match left_type {
                        Type::Int => self.generate_bool_op(function, operator, Type::Int),
                        Type::Float => self.generate_bool_op(function, operator, Type::Float),
                        Type::Bool => self.generate_bool_op(function, operator, Type::Bool),
                        _ => unreachable!(),
                    },
                    _ => unreachable!(),
                };
                function.push_str("\n\t;unop end\t\t\t\t--- E");
                assert!(s_height + 1 == self.shadow_stack.len());
                result
            }
            ExpressionType::ArrayLiteral { elements } => {
                let s_height = self.shadow_stack.len();
                function.push_str("\n\t;array literal start\t\t\t--- E");
                // Generate code for each element (in reverse order) and capture the element size.
                let mut element_size = 0;
                for element in elements.iter().rev() {
                    let (size, _) = self.generate_expression(function, element, environment);
                    element_size = size;
                    self.print_stack_size(function);
                }
                function.push_str("\n\t;array literal done iterating");
                function.push_str(&format!("\n\tmov rdi, {}", elements.len() * element_size));
                self.print_stack_size(function);
                self.check_add_alignment(function);
                self.print_stack_size(function);
                function.push_str("\n\tcall _jpl_alloc");
                self.print_stack_size(function);
                self.check_remove_alignment(function);
                self.print_stack_size(function);

                function.push_str(&format!(
                    "\n\t;Moving {} bytes from rsp to allocated memory",
                    elements.len() * element_size
                ));
                for i in (0..elements.len() * element_size / 8).rev() {
                    function.push_str(&format!(
                        "\n\tmov r10, [rsp + {}]\n\tmov [rax + {}], r10",
                        i * 8,
                        i * 8
                    ));
                }

                // Remove the element values from the stack and shadow stack.
                function.push_str(&format!("\n\tadd rsp, {}", elements.len() * element_size));
                for _ in 0..elements.len() {
                    self.shadow_stack.pop_back();
                }

                // Push the allocated array pointer and the dimension onto the stack.
                function.push_str("\n\tpush rax");
                function.push_str(&format!("\n\tmov rax, {}", elements.len()));
                function.push_str("\n\tpush rax");
                self.shadow_stack.push_back((16, false, None));

                function.push_str("\n\t;array literal end\t\t\t--- E");
                assert!(s_height + 1 == self.shadow_stack.len());
                (
                    16,
                    Type::Array {
                        element_type: Box::new(elements.first().unwrap().resolved_type.clone()),
                        rank: 1,
                    },
                )
            }
            ExpressionType::Variable { name } => {
                let s_height = self.shadow_stack.len();
                function.push_str("\n\t;var start\t\t\t--- E");
                self.print_stack_size(function);
                let (res,offset) = self.offsets.get(*name).unwrap();
                function.push_str(&format!("\n\tsub rsp, {} ;allocate for variable", res.0));
                function.push_str(&format!("\n\tmov r10, [rbp - {} + 0] ;get from offset",offset - res.0));//get
                function.push_str("\n\tmov [rsp + 0], r10 ;push into allocated location");//push
                self.shadow_stack.push_back((res.0,false,Some(expression.resolved_type.clone())));
                self.print_stack_size(function);
                function.push_str("\n\t;var end\t\t\t--- E");
                assert!(s_height + 1 == self.shadow_stack.len());
                res.clone()
            }
            _ => unimplemented!(),
        }
    }

    fn generate_int_op(&mut self, function: &mut String, operator: &str) -> (usize, Type<'a>) {
        function.push_str("\n\t;int op");
        match operator {
            "+" => {
                self.print_stack_size(function);
                self.shadow_stack.pop_back();
                self.shadow_stack.pop_back();
                self.shadow_stack.push_back((8,false,Some(Type::Int)));
                function.push_str("\n\tpop rax\n\tpop r10\n\tadd rax, r10\n\tpush rax");
                self.print_stack_size(function);
            },
            "-" => {
                self.shadow_stack.pop_back();
                self.shadow_stack.pop_back();
                self.shadow_stack.push_back((8,false,Some(Type::Int)));
                function.push_str("\n\tpop rax\n\tpop r10\n\tsub rax, r10\n\tpush rax");
            },
            "*" => {
                self.shadow_stack.pop_back();
                self.shadow_stack.pop_back();
                self.shadow_stack.push_back((8,false,Some(Type::Int)));
                function.push_str("\n\tpop rax\n\tpop r10\n\timul rax, r10\n\tpush rax");
            },
            "/" | "%" => {
                self.print_stack_size(function);
                self.shadow_stack.pop_back();
                self.shadow_stack.pop_back();
                function.push_str("\n\tpop rax\n\tpop r10\n\tcmp r10, 0");
                self.print_stack_size(function);
                let jump_label = format!(".jump{}", self.jump_counter);
                self.jump_counter += 1;
                function.push_str(&format!("\n\tjne {}", jump_label));
                self.check_add_alignment(function);
                function.push_str(&format!(
                    "\n\tlea rdi, [rel {}]",
                    self.get_constant(AssemblyValue::String(
                        if operator == "/" {
                            "divide by zero"
                        } else {
                            "mod by zero"
                        }
                        .to_string()
                    ))
                ));
                function.push_str("\n\tcall _fail_assertion");
                self.check_remove_alignment(function);

                function.push_str(&format!("\n{}:", jump_label));
                function.push_str("\n\tcqo\n\tidiv r10");

                if operator == "%" {
                    function.push_str("\n\tmov rax, rdx");
                }
                self.shadow_stack.push_back((8,false,Some(Type::Int)));
                function.push_str("\n\tpush rax");
            }
            _ => {
                unreachable!();
            }
        }
        self.print_stack_size(function);
        function.push_str("\n\t;int op");
        (8, Type::Int)
    }

    fn generate_float_op(&mut self, function: &mut String, operator: &str) -> (usize, Type<'a>) {
        function.push_str("\n\t;float binop start");
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
                    "\n\tmovsd xmm0, [rsp]\n\tadd rsp, 8\n\tmovsd xmm1, [rsp]\n\tadd rsp, 8\n\t{} xmm0, xmm1\n\tsub rsp, 8\n\tmovsd [rsp], xmm0",
                    op_instruction
                ));
            }
            "%" => {
                function.push_str("\n\tmovsd xmm0, [rsp]\n\tadd rsp, 8\n\tmovsd xmm1, [rsp]\n\tadd rsp, 8\n\tcall _fmod");

                function.push_str("\n\t;fmod alignment remove");
                self.check_remove_alignment(function);

                function.push_str("\n\tsub rsp, 8\n\tmovsd [rsp], xmm0");
            }
            _ => unreachable!(),
        }
        self.shadow_stack.pop_back();
        self.print_stack_size(function);
        function.push_str("\n\t;float binop end");

        (8, Type::Float)
    }

    fn generate_bool_op(
        &mut self,
        function: &mut String,
        operator: &str,
        left_type: Type<'a>,
    ) -> (usize, Type<'a>) {
        match left_type {
            Type::Int => self.generate_int_comparison(function, operator),
            Type::Float => self.generate_float_comparison(function, operator),
            Type::Bool => self.generate_bool_comparison(function, operator),
            _ => unreachable!(),
        }
        self.shadow_stack.pop_back();
        self.print_stack_size(function);
        (8, Type::Bool)
    }

    fn generate_int_comparison(&self, function: &mut String, operator: &str) {
        function.push_str("\n\t;int binop start");
        let set_instruction = match operator {
            "<" => "setl",
            "<=" => "setle",
            ">" => "setg",
            ">=" => "setge",
            "==" => "sete",
            "!=" => "setne",
            _ => unreachable!(),
        };

        function.push_str(&format!(
            "\n\tpop rax\n\tpop r10\n\tcmp rax, r10\n\t{} al\n\tand rax, 1\n\tpush rax",
            set_instruction
        ));
        self.print_stack_size(function);
        function.push_str("\n\t;int binop end");
    }

    fn generate_float_comparison(&self, function: &mut String, operator: &str) {
        function.push_str("\n\t;float start\n\tmovsd xmm0, [rsp]\n\tadd rsp, 8\n\tmovsd xmm1, [rsp]\n\tadd rsp, 8");

        let (cmp_instruction, swap_operands) = match operator {
            "<" => ("cmpltsd", false),
            "<=" => ("cmplesd", false),
            ">" => ("cmpltsd", true),
            ">=" => ("cmplesd", true),
            "==" => ("cmpeqsd", false),
            "!=" => ("cmpneqsd", false),
            _ => unreachable!(),
        };

        let cmp_code = if swap_operands {
            format!(
                "\n\t{} xmm1, xmm0\n\tmovq rax, xmm1\n\tand rax, 1\n\tpush rax\n\t;float end",
                cmp_instruction
            )
        } else {
            format!(
                "\n\t{} xmm0, xmm1\n\tmovq rax, xmm0\n\tand rax, 1\n\tpush rax\n\t;float end",
                cmp_instruction
            )
        };

        function.push_str(&cmp_code);
        self.print_stack_size(function);
    }

    fn generate_bool_comparison(&self, function: &mut String, operator: &str) {
        match operator {
            "&&" => function.push_str("\n\t;and start\n\tpop rax\n\tpop r10\n\tand rax, r10\n\tpush rax\n\t;and end"),
            "||" => function.push_str("\n\t;or start\n\tpop rax\n\tpop r10\n\tor rax, r10\n\tpush rax\n\t;or end"),
            "==" | "!=" => {
                function.push_str(
                    &"\n\t;bool eq or neq start\n\tpop rax\n\tpop r10\n\tcmp rax, r10\n\t\
                     {} al\n\tand rax, 1\n\tpush rax\n\t;bool eq or neq end"
                        .replace("{}", if operator == "==" { "sete" } else { "setne" }),
                );
            }
            _ => unreachable!(),
        }
        self.print_stack_size(function);
    }

    fn get_constant(&mut self, value: AssemblyValue) -> &str {
        let data_section_len = self.data_section.len();
        let entry = self.constants.entry(value.clone()).or_insert_with(|| {
            self.data_section.push(value.clone());
            format!("const{}", data_section_len)
        });
        entry
    }

    fn check_add_alignment_with(&mut self, function: &mut String, t: &Type<'a>) {
        let s = Self::get_type_stack_size(t);
        self.shadow_stack.push_back((s,false,None));
        self.check_add_alignment(function);
        let a = self.shadow_stack.pop_back().unwrap();
        self.shadow_stack.pop_back();
        self.shadow_stack.push_back(a);
    }

    fn get_type_stack_size(typ: &Type<'a>) -> usize {
        match typ {
            Type::Int | Type::Float | Type::Bool | Type::Void => 8,
            Type::Array {
                element_type: _,
                rank,
            } => 8 + 8 * rank,
            Type::Struct { name: _, elements } => {
                let mut size = 0;
                for (_, typ) in elements {
                    size += Self::get_type_stack_size(typ);
                }
                size
            }
            _ => unreachable!(),
        }
    }

    fn add_one_to_stack(&self, function: &mut String, expression: &Expression<'a>) {
        let size = Self::get_type_stack_size(&expression.resolved_type);
        function.push_str(&format!("\n\tsub rsp, {}", size));
    }

    fn add_all_to_stack(&self, function: &mut String, types: &Vec<&Expression<'a>>) {
        for e in types {
            self.add_one_to_stack(function, e);
        }
    }

    fn pre_call_padding(&mut self, function: &mut String, types: &Vec<&Expression<'a>>) -> (usize,bool,Option<Type<'a>>) {
        self.add_all_to_stack(function, types);
        self.check_add_alignment(function);
        let padding = self.shadow_stack.pop_back().unwrap().clone();
        for _ in 0..types.len() { self.shadow_stack.pop_back();}
        padding
    }

    fn stack_inspect(&self) -> String {
        let mut out = String::new();
        for (size,padding,typ) in self.shadow_stack.iter() {
            out.push_str(
                &format!("\n\tsize:{}\tpadding:{}\ttype{}",
                size,
                padding,
                typ.clone().unwrap_or(Type::Void)));
        }
        return out;
    }

    fn check_add_alignment(&mut self, function: &mut String) {
        function.push_str("\n\t;Check align");
        if self.stack_size() % 16 == 0 {
            function.push_str("\n\tsub rsp, 8 ;Add align");
            self.shadow_stack.push_back((8,true,None));
        } else {
            function.push_str("\n\t;no align");
            self.shadow_stack.push_back((0,true,None));
        }
        self.print_stack_size(function);
    }

fn check_remove_alignment(&mut self, function: &mut String) {
    function.push_str("\n\t;remove align marker");
    // Search for the most recent alignment marker in the shadow stack.
    let pos_opt = self.shadow_stack
        .iter()
        .enumerate()
        .rev()
        .find(|(_, entry)| entry.1) // entry.1 is the padding flag
        .map(|(i, _)| i);

    if let Some(pos) = pos_opt {
        let (size, _padding, _) = self.shadow_stack.remove(pos).unwrap();
        if pos != self.shadow_stack.len() - 1 {
            function.push_str("\n\t; mid stack align remove");
        }
        if size != 0 {
            function.push_str(&format!("\n\tadd rsp, {} ;remove align", size));
        }
    }
    self.print_stack_size(function);
}

    fn print_stack_size(&self, function: &mut String) {
        let mut stac = String::new();
        for (size,padding,typ) in self.shadow_stack.iter() {
            if *padding  {
                stac.push_str(&format!("[{}] ",size));
            } else {
                stac.push_str(
                    &format!("{}:{} ",typ.clone().unwrap_or(Type::Void), size));
            }
        }
       function.push_str(&format!("\n\t;Stack#{}# from: {}", self.stack_size(), stac));
    }
}


pub fn generate_assembly<'a>(
    commands: Vec<Command<'a>>,
    environment: TypeEnvironment<'a>,
) -> String {
    let mut generator = AssemblyGenerator::new();
    generator.generate_assembly(commands, environment)
}
