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

        for command in commands {
            let s_height = self.shadow_stack.len();
            self.generate_command(&mut main_function, &command, &environment);
            assert!(s_height == self.shadow_stack.len() || s_height == self.shadow_stack.len() - 1);
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

    fn generate_statement(
        &mut self,
        function_string: &mut String,
        statement: &Statement<'a>,
        environment: &TypeEnvironment<'a>,
    ) {
        match &statement.node {
            StatementType::Return { value } => {
                // generate expression
                self.generate_expression(function_string, value, environment);
                self.shadow_stack.pop_back();
                self.shadow_stack.pop_back();
                self.print_stack_size(function_string);
                function_string.push_str("\n\t;RETURN end\t\t\t\t--- C");
            }
            StatementType::Let { variable, rvalue } => {
                self.handle_let(function_string, environment, variable, rvalue);
            }
            _ => {
                println!("\n\nfailure because of statement {}", statement.to_string());
                unimplemented!()
            }
        }
    }

    fn generate_command(
        &mut self,
        function_string: &mut String,
        command: &Command<'a>,
        environment: &TypeEnvironment<'a>,
    ) {
        match command.node.as_ref() {
            CommandType::Show { expression } => {
                let s_height = self.shadow_stack.len();
                function_string.push_str("\n\t;SHOW start\t\t\t\t--- C");
                self.print_stack_size(function_string);
                self.check_add_alignment_with(function_string, &expression.resolved_type);
                self.print_stack_size(function_string);
                let expr_result = self.generate_expression(function_string, expression, environment);
                self.print_stack_size(function_string);
                let size = expr_result.0;
                let typ_str = expr_result.1.to_string();
                let const_name = self.get_constant(AssemblyValue::String(typ_str));
                function_string.push_str(&format!("\n\tlea rdi, [rel {}]", const_name));
                self.shadow_stack.push_back((8,false,None));
                function_string.push_str("\n\tlea rsi, [rsp]");
                self.print_stack_size(function_string);
                function_string.push_str("\n\tcall _show");
                self.print_stack_size(function_string);
                self.shadow_stack.pop_back();
                function_string.push_str(&format!("\n\tadd rsp, {}", size));
                self.print_stack_size(function_string);
                self.check_remove_alignment(function_string);
                self.print_stack_size(function_string);
                self.shadow_stack.pop_back();
                self.print_stack_size(function_string);
                function_string.push_str("\n\t;SHOW end\t\t\t\t--- C");
                assert!(s_height == self.shadow_stack.len());
            }
            CommandType::Let { variable, rvalue} => {
                self.handle_let(function_string, environment, variable, rvalue);
            }
            CommandType::Function {
                name,
                parameters,
                return_type,
                statements,
                has_return,
            } => {
                let main_shadow_stack = self.shadow_stack.clone();
                self.shadow_stack = VecDeque::new();
                let mut function = String::new();
                function.push_str("\n\t;FUNC start\t\t\t\t--- FFFF");
                function.push_str(&format!("\n{}:\n_{}:", name, name));
                // Function prelude
                function.push_str(&format!("\n\t;{} prelude", name));
                function.push_str("\n\tpush rbp");
                function.push_str("\n\tmov rbp, rsp");

                for statement in statements {
                    self.generate_statement(&mut function, &statement, environment);
                }

                function.push_str(&format!("\n\t; {} postlude", name));
                match return_type {
                    Type::Int | Type::Bool  => {
                        function.push_str("\n\tpop rax ; put top of stack in rax");
                    }
                    Type::Float => {
                        function.push_str("\n\tmovsd xmm0, [rsp]");
                        function.push_str("\n\tadd rsp, 8");
                    }
                    _ => {
                        println!("\n\nfailure for function return type {}", return_type.to_string());
                        unimplemented!()
                    }
                }
                function.push_str(&format!("\n\tadd rsp, {} ; Local variables",self.stack_size()));
                function.push_str("\n\tpop rbp");
                function.push_str("\n\tret");
                function.push_str("\n\t;FUNC end\t\t\t\t--- FFFF");
                self.functions.push(function);
                self.shadow_stack = main_shadow_stack;
            }
            _ => {
                println!("\n\nfailure for command type {}", command.to_string());
                unimplemented!()
            }
        }
    }

    fn handle_let(
        &mut self, function_string: &mut String,
        environment: &TypeEnvironment<'a>,
        variable: &LValue<'_>,
        rvalue: &Expression<'a>,
    ) {
        match &variable.node {
            LValueType::Variable => {
                let s_height = self.shadow_stack.len();
                function_string.push_str("\n\t;LET start\t\t\t\t--- C");
                let res = self.generate_expression(function_string, rvalue, environment);
                self.offsets.insert(variable.name.to_string(), (res,self.stack_size()));
                function_string.push_str("\n\t;LET end\t\t\t\t--- C");
                assert!(s_height + 1 == self.shadow_stack.len());
            }
            LValueType::Array { indices } => {
                let s_height = self.shadow_stack.len();
                function_string.push_str("\n\t;LET start\t\t\t\t--- C");
                let res = self.generate_expression(function_string, rvalue, environment);
                self.offsets.insert(variable.name.to_string(), (res.clone(),self.stack_size()));
                for (i,b_name) in indices.iter().enumerate() {
                    let stack_location = i * 8 + self.stack_size() - res.0 + 16; //this is a little suspicious
                    self.offsets.insert(b_name.to_string(), ((8, Type::Int),stack_location));
                }
                function_string.push_str("\n\t;LET end\t\t\t\t--- C");
                assert!(s_height + 1 == self.shadow_stack.len());
            }
        }
    }
    
    /// Generate assembly code for an expression
    /// Location of generated expression is always rax
    /// Returns size of expression in bytes
    fn generate_expression(
        &mut self,
        function_string: &mut String,
        expression: &Expression<'a>,
        environment: &TypeEnvironment<'a>,
    ) -> (usize, Type<'a>) {
        match expression.node.as_ref() {
            ExpressionType::Int { value } => {
                let s_height = self.shadow_stack.len();
                function_string.push_str("\n\t;int const start\t\t\t--- E");
                self.print_stack_size(function_string);
                let startstack = self.stack_size();
                let constant = self.get_constant(AssemblyValue::Number(value.to_string()));
                function_string.push_str(&format!("\n\tmov rax, [rel {}]", constant));
                function_string.push_str("\n\tpush rax");
                self.shadow_stack.push_back((8,false,Some(expression.resolved_type.clone())));
                assert!(startstack == self.stack_size() - 8);
                self.print_stack_size(function_string);
                function_string.push_str("\n\t;int const end\t\t\t\t--- E");
                assert!(s_height + 1 == self.shadow_stack.len());
                (8, Type::Int)
            }
            ExpressionType::Float { value } => {
                let s_height = self.shadow_stack.len();
                function_string.push_str("\n\t;float const start\t\t\t--- E");
                let startstack = self.stack_size();
                let constant = self.get_constant(AssemblyValue::Number(format!("{:?}", value)));
                function_string.push_str(&format!("\n\tmov rax, [rel {}]", constant));
                function_string.push_str("\n\tpush rax");
                self.shadow_stack.push_back((8,false,Some(expression.resolved_type.clone())));
                assert!(startstack == self.stack_size() - 8);
                function_string.push_str("\n\t;float const end\t\t\t--- E");
                assert!(s_height + 1 == self.shadow_stack.len());
                (8, Type::Float)
            }
            ExpressionType::True => {
                let s_height = self.shadow_stack.len();
                function_string.push_str("\n\t;true const start\t\t\t--- E");
                let constant = self.get_constant(AssemblyValue::Number("1".to_string()));
                function_string.push_str(&format!("\n\tmov rax, [rel {}]", constant));
                function_string.push_str("\n\tpush rax");
                self.shadow_stack.push_back((8,false,Some(expression.resolved_type.clone())));
                function_string.push_str("\n\t;true const end\t\t\t--- E");
                assert!(s_height + 1 == self.shadow_stack.len());
                (8, Type::Bool)
            }
            ExpressionType::False => {
                let s_height = self.shadow_stack.len();
                function_string.push_str("\n\t;false const start\t\t\t--- E");
                let constant = self.get_constant(AssemblyValue::Number("0".to_string()));
                function_string.push_str(&format!("\n\tmov rax, [rel {}]", constant));
                function_string.push_str("\n\tpush rax");
                self.shadow_stack.push_back((8,false,Some(expression.resolved_type.clone())));
                function_string.push_str("\n\t;int const end\t\t\t--- E");
                assert!(s_height + 1 == self.shadow_stack.len());
                (8, Type::Bool)
            }
            ExpressionType::Unop {
                operator: _,
                expression,
            } => {
                let s_height = self.shadow_stack.len();
                function_string.push_str("\n\t;unop start\t\t\t--- E");
                let startstack = self.stack_size();
                let (size, typ) = self.generate_expression(function_string, expression, environment);
                assert!(startstack == self.stack_size() - 8);

                match typ {
                    Type::Int => {
                        function_string.push_str("\n\tpop rax");
                        function_string.push_str("\n\tneg rax");
                        function_string.push_str("\n\tpush rax");
                    }
                    Type::Float => {
                        function_string.push_str("\n\tmovsd xmm1, [rsp]");
                        function_string.push_str("\n\tadd rsp, 8");
                        function_string.push_str("\n\tpxor xmm0, xmm0");
                        function_string.push_str("\n\tsubsd xmm0, xmm1");
                        function_string.push_str("\n\tsub rsp, 8");
                        function_string.push_str("\n\tmovsd [rsp], xmm0");
                    }
                    Type::Bool => {
                        function_string.push_str("\n\tpop rax");
                        function_string.push_str("\n\txor rax, 1");
                        function_string.push_str("\n\tpush rax");
                    }
                    _ => unreachable!(),
                }
                assert!(startstack == self.stack_size() - 8);
                function_string.push_str("\n\t;unop end\t\t\t\t--- E");
                assert!(s_height + 1 == self.shadow_stack.len());
                (size, typ)
            }
            ExpressionType::Binop {
                operator,
                left,
                right,
            } => {
                let s_height = self.shadow_stack.len();
                function_string.push_str("\n\t;binop start\t\t\t\t--- E");
                if *operator == "%" && matches!(left.resolved_type, Type::Float) {
                    function_string.push_str("\n\t;fmod alignment add");
                    self.check_add_alignment(function_string);
                }
                let alignedstack = self.stack_size();

                assert!(alignedstack == self.stack_size());
                self.generate_expression(function_string, right, environment);
                assert!(alignedstack == self.stack_size() - 8, "\n{}", right.to_string());
                let (_, left_type) = self.generate_expression(function_string, left, environment);

                assert!(alignedstack == self.stack_size() - 16, "\n{}\n{}", right.to_string(), left.to_string());
                let result = match expression.resolved_type {
                    Type::Int => self.generate_int_op(function_string, operator),
                    Type::Float => self.generate_float_op(function_string, operator),
                    Type::Bool => match left_type {
                        Type::Int => self.generate_bool_op(function_string, operator, Type::Int),
                        Type::Float => self.generate_bool_op(function_string, operator, Type::Float),
                        Type::Bool => self.generate_bool_op(function_string, operator, Type::Bool),
                        _ => unreachable!(),
                    },
                    _ => unreachable!(),
                };
                function_string.push_str("\n\t;unop end\t\t\t\t--- E");
                assert!(s_height + 1 == self.shadow_stack.len());
                result
            }
            ExpressionType::ArrayLiteral { elements } => {
                let s_height = self.shadow_stack.len();
                function_string.push_str("\n\t;array literal start\t\t\t--- E");
                // Generate code for each element (in reverse order) and capture the element size.
                let mut element_size = 0;
                for element in elements.iter().rev() {
                    let (size, _) = self.generate_expression(function_string, element, environment);
                    element_size = size;
                    self.print_stack_size(function_string);
                }
                function_string.push_str("\n\t;array literal done iterating");
                function_string.push_str(&format!("\n\tmov rdi, {}", elements.len() * element_size));
                self.print_stack_size(function_string);
                self.check_add_alignment(function_string);
                self.print_stack_size(function_string);
                function_string.push_str("\n\tcall _jpl_alloc");
                self.print_stack_size(function_string);
                self.check_remove_alignment(function_string);
                self.print_stack_size(function_string);

                function_string.push_str(&format!(
                    "\n\t;Moving {} bytes from rsp to allocated memory",
                    elements.len() * element_size
                ));
                for i in (0..elements.len() * element_size / 8).rev() {
                    function_string.push_str(&format!(
                        "\n\tmov r10, [rsp + {}]\n\tmov [rax + {}], r10",
                        i * 8,
                        i * 8
                    ));
                }

                // Remove the element values from the stack and shadow stack.
                function_string.push_str(&format!("\n\tadd rsp, {}", elements.len() * element_size));
                for _ in 0..elements.len() {
                    self.shadow_stack.pop_back();
                }

                // Push the allocated array pointer and the dimension onto the stack.
                function_string.push_str("\n\tpush rax");
                function_string.push_str(&format!("\n\tmov rax, {}", elements.len()));
                function_string.push_str("\n\tpush rax");
                self.shadow_stack.push_back((16, false, Some(expression.resolved_type.clone())));

                function_string.push_str("\n\t;array literal end\t\t\t--- E");
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
                let ret = self.handle_variable(function_string, name, expression);
                assert!(s_height + 1 == self.shadow_stack.len());
                ret
            }
            ExpressionType::Call { function, arguments } => {
                let s_height = self.shadow_stack.len();
                self.check_add_alignment(function_string);
                if arguments.len() != 0 {
                    println!("\n\nfailure because function has arguments");
                    unimplemented!()
                }
                function_string.push_str(&format!("\n\tcall _{}", function.to_string()));
                self.check_remove_alignment(function_string);
                let ret_type = expression.resolved_type.clone();
                let size = Self::get_type_stack_size(&ret_type);
                self.shadow_stack.push_back((size,false,Some(ret_type.clone())));
                match &expression.resolved_type {
                    Type::Int | Type::Bool => {
                        function_string.push_str("\n\tpush rax");
                    }
                    Type::Float => {
                        function_string.push_str("\n\tsub rsp, 8");
                        function_string.push_str("\n\tmovsd [rsp], xmm0");
                    }
                    _ => {
                        println!("\n\nfailure for call return type {}", ret_type.to_string());
                    }
                }
                assert!(s_height + 1 == self.shadow_stack.len());
                (size, ret_type)
            }
            _ => {
                println!("failure because for expression: {}", expression.to_string());
                unimplemented!();
            }
        }
    }

    fn handle_variable(
        &mut self, function_string: &mut String,
        name: &str, expression: &Expression<'a>,
    ) -> (usize, Type<'a>) {
        match expression.resolved_type {
            Type::Int | Type::Bool | Type::Float => {
                //TODO: handle array bounds?
                let s_height = self.shadow_stack.len();
                function_string.push_str("\n\t;var start\t\t\t--- E");
                self.print_stack_size(function_string);
                let (res,offset) = self.offsets.get(name).unwrap();
                function_string.push_str(&format!("\n\tsub rsp, {} ;allocate for variable", res.0));
                function_string.push_str(&format!("\n\tmov r10, [rbp - {} + 0] ;get from offset",offset - 8));//get
                function_string.push_str("\n\tmov [rsp + 0], r10 ;push into allocated location");//push
                self.shadow_stack.push_back((res.0,false,Some(expression.resolved_type.clone())));
                self.print_stack_size(function_string);
                function_string.push_str("\n\t;var end\t\t\t--- E");
                assert!(s_height + 1 == self.shadow_stack.len());
                res.clone()
            }
            Type::Array { element_type: _, rank } => {
                let s_height = self.shadow_stack.len();
                let (res,offset) = self.offsets.get(name).unwrap();
                function_string.push_str("\n\t;array var start\t\t\t--- E");
                self.print_stack_size(function_string);
                self.shadow_stack.push_back((res.0,false,Some(expression.resolved_type.clone())));
                function_string.push_str(&format!("\n\tsub rsp, {}", res.0));
                self.print_stack_size(function_string);
                for i in (0..rank+1).rev() {
                    function_string.push_str(&format!("\n\tmov r10, [rbp - {} + {}]", offset - 8, i * 8));
                    function_string.push_str(&format!("\n\tmov [rsp + {}], r10",i * 8));
                }
                function_string.push_str("\n\t;array var end\t\t\t--- E");
                assert!(s_height + 1 == self.shadow_stack.len());
                res.clone()
            }
            _ => unreachable!(),
        }
    }

    fn generate_int_op(
        &mut self, function_string: &mut String,
        operator: &str,
    ) -> (usize, Type<'a>) {
        function_string.push_str("\n\t;int op");
        match operator {
            "+" => {
                self.print_stack_size(function_string);
                self.shadow_stack.pop_back();
                self.shadow_stack.pop_back();
                self.shadow_stack.push_back((8,false,Some(Type::Int)));
                function_string.push_str("\n\tpop rax\n\tpop r10\n\tadd rax, r10\n\tpush rax");
                self.print_stack_size(function_string);
            },
            "-" => {
                self.shadow_stack.pop_back();
                self.shadow_stack.pop_back();
                self.shadow_stack.push_back((8,false,Some(Type::Int)));
                function_string.push_str("\n\tpop rax\n\tpop r10\n\tsub rax, r10\n\tpush rax");
            },
            "*" => {
                self.shadow_stack.pop_back();
                self.shadow_stack.pop_back();
                self.shadow_stack.push_back((8,false,Some(Type::Int)));
                function_string.push_str("\n\tpop rax\n\tpop r10\n\timul rax, r10\n\tpush rax");
            },
            "/" | "%" => {
                self.print_stack_size(function_string);
                self.shadow_stack.pop_back();
                self.shadow_stack.pop_back();
                function_string.push_str("\n\tpop rax\n\tpop r10\n\tcmp r10, 0");
                self.print_stack_size(function_string);
                let jump_label = format!(".jump{}", self.jump_counter);
                self.jump_counter += 1;
                function_string.push_str(&format!("\n\tjne {}", jump_label));
                self.check_add_alignment(function_string);
                function_string.push_str(&format!(
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
                function_string.push_str("\n\tcall _fail_assertion");
                self.check_remove_alignment(function_string);

                function_string.push_str(&format!("\n{}:", jump_label));
                function_string.push_str("\n\tcqo\n\tidiv r10");

                if operator == "%" {
                    function_string.push_str("\n\tmov rax, rdx");
                }
                self.shadow_stack.push_back((8,false,Some(Type::Int)));
                function_string.push_str("\n\tpush rax");
            }
            _ => {
                unreachable!();
            }
        }
        self.print_stack_size(function_string);
        function_string.push_str("\n\t;int op");
        (8, Type::Int)
    }

    fn generate_float_op(
        &mut self, function_string: &mut String,
        operator: &str,
    ) -> (usize, Type<'a>) {
        function_string.push_str("\n\t;float binop start");
        match operator {
            "+" | "-" | "*" | "/" => {
                let op_instruction = match operator {
                    "+" => "addsd",
                    "-" => "subsd",
                    "*" => "mulsd",
                    "/" => "divsd",
                    _ => unreachable!(),
                };

                function_string.push_str(&format!(
                    "\n\tmovsd xmm0, [rsp]\n\tadd rsp, 8\n\tmovsd xmm1, [rsp]\n\tadd rsp, 8\n\t{} xmm0, xmm1\n\tsub rsp, 8\n\tmovsd [rsp], xmm0",
                    op_instruction
                ));
            }
            "%" => {
                function_string.push_str("\n\tmovsd xmm0, [rsp]\n\tadd rsp, 8\n\tmovsd xmm1, [rsp]\n\tadd rsp, 8\n\tcall _fmod");

                function_string.push_str("\n\t;fmod alignment remove");
                self.check_remove_alignment(function_string);

                function_string.push_str("\n\tsub rsp, 8\n\tmovsd [rsp], xmm0");
            }
            _ => unreachable!(),
        }
        self.shadow_stack.pop_back();
        self.print_stack_size(function_string);
        function_string.push_str("\n\t;float binop end");

        (8, Type::Float)
    }

    fn generate_bool_op(
        &mut self,
        function_string: &mut String,
        operator: &str,
        left_type: Type<'a>,
    ) -> (usize, Type<'a>) {
        match left_type {
            Type::Int => self.generate_int_comparison(function_string, operator),
            Type::Float => self.generate_float_comparison(function_string, operator),
            Type::Bool => self.generate_bool_comparison(function_string, operator),
            _ => unreachable!(),
        }
        self.shadow_stack.pop_back();
        self.print_stack_size(function_string);
        (8, Type::Bool)
    }

    fn generate_int_comparison(
        &self, function_string: &mut String,
        operator: &str,
    ) {
        function_string.push_str("\n\t;int binop start");
        let set_instruction = match operator {
            "<" => "setl",
            "<=" => "setle",
            ">" => "setg",
            ">=" => "setge",
            "==" => "sete",
            "!=" => "setne",
            _ => unreachable!(),
        };

        function_string.push_str(&format!(
            "\n\tpop rax\n\tpop r10\n\tcmp rax, r10\n\t{} al\n\tand rax, 1\n\tpush rax",
            set_instruction
        ));
        self.print_stack_size(function_string);
        function_string.push_str("\n\t;int binop end");
    }

    fn generate_float_comparison(
        &self, function_string: &mut String,
        operator: &str,
    ) {
        function_string.push_str("\n\t;float start\n\tmovsd xmm0, [rsp]\n\tadd rsp, 8\n\tmovsd xmm1, [rsp]\n\tadd rsp, 8");

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

        function_string.push_str(&cmp_code);
        self.print_stack_size(function_string);
    }

    fn generate_bool_comparison(
        &self, function_string: &mut String,
        operator: &str,
    ) {
        match operator {
            "&&" => function_string.push_str("\n\t;and start\n\tpop rax\n\tpop r10\n\tand rax, r10\n\tpush rax\n\t;and end"),
            "||" => function_string.push_str("\n\t;or start\n\tpop rax\n\tpop r10\n\tor rax, r10\n\tpush rax\n\t;or end"),
            "==" | "!=" => {
                function_string.push_str(
                    &"\n\t;bool eq or neq start\n\tpop rax\n\tpop r10\n\tcmp rax, r10\n\t\
                     {} al\n\tand rax, 1\n\tpush rax\n\t;bool eq or neq end"
                        .replace("{}", if operator == "==" { "sete" } else { "setne" }),
                );
            }
            _ => unreachable!(),
        }
        self.print_stack_size(function_string);
    }

    fn get_constant(&mut self, value: AssemblyValue) -> &str {
        let data_section_len = self.data_section.len();
        let entry = self.constants.entry(value.clone()).or_insert_with(|| {
            self.data_section.push(value.clone());
            format!("const{}", data_section_len)
        });
        entry
    }

    fn check_add_alignment_with(
        &mut self, function_string: &mut String,
        t: &Type<'a>,
    ) {
        let s = Self::get_type_stack_size(t);
        self.shadow_stack.push_back((s,false,None));
        self.check_add_alignment(function_string);
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

    fn check_add_alignment(
        &mut self,
        function_string: &mut String,
    ) {
        function_string.push_str("\n\t;Check align");
        if self.stack_size() % 16 == 0 {
            function_string.push_str("\n\tsub rsp, 8 ;Add align");
            self.shadow_stack.push_back((8,true,None));
        } else {
            function_string.push_str("\n\t;no align");
            self.shadow_stack.push_back((0,true,None));
        }
        self.print_stack_size(function_string);
    }

fn check_remove_alignment(
    &mut self,
    function_string: &mut String,
) {
    function_string.push_str("\n\t;remove align marker");
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
            function_string.push_str("\n\t; mid stack align remove");
        }
        if size != 0 {
            function_string.push_str(&format!("\n\tadd rsp, {} ;remove align", size));
        }
    }
    self.print_stack_size(function_string);
}

    fn print_stack_size(
        &self,
        function_string: &mut String,
    ) {
        let mut stac = String::new();
        for (size,padding,typ) in self.shadow_stack.iter() {
            if *padding  {
                stac.push_str(&format!("[{}] ",size));
            } else {
                stac.push_str(
                    &format!("{}:{} ",typ.clone().unwrap_or(Type::Void), size));
            }
        }
       function_string.push_str(&format!("\n\t;Stack#{}# from: {}", self.stack_size(), stac));
    }
}


pub fn generate_assembly<'a>(
    commands: Vec<Command<'a>>,
    environment: TypeEnvironment<'a>,
) -> String {
    let mut generator = AssemblyGenerator::new();
    generator.generate_assembly(commands, environment)
}
