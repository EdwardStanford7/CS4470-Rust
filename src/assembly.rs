use crate::ast::*;
use crate::utils::*;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::fmt;
use std::fmt::Display;
use std::fmt::Write;

const INT_REGS: [&str; 6] = ["rdi", "rsi", "rdx", "rcx", "r8", "r9"];
const FLO_REGS: [&str; 8] = [
    "xmm0", "xmm1", "xmm2", "xmm3", "xmm4", "xmm5", "xmm6", "xmm7",
];

#[derive(Debug)]
enum Shadow<'a> {
    Padding(bool),
    Item(Type<'a>),
}

impl Display for Shadow<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Shadow::Padding(true) => write!(f, "p:[8]"),
            Shadow::Padding(false) => write!(f, "p:[0]"),
            Shadow::Item(typ) => write!(f, "{}:[{}]", typ, AsmFunction::get_type_stack_size(typ)),
        }
    }
}

struct AsmFunction<'a> {
    body: String,
    stack_size: isize,
    shadow_stack: VecDeque<Shadow<'a>>,
    assert_stack: VecDeque<usize>,
}

impl<'a> AsmFunction<'a> {
    fn new() -> Self {
        Self {
            body: String::new(),
            stack_size: 0,
            shadow_stack: VecDeque::new(),
            assert_stack: VecDeque::new(),
        }
    }

    fn push_assert(&mut self) {
        self.assert_stack.push_back(self.shadow_stack.len());
    }

    fn pop_assert_opt(&mut self, added_sizes: Vec<usize>) -> bool {
        let size_opt = self.assert_stack.pop_back();
        match size_opt {
            Some(size) => {
                let mut valid = false;
                for added_size in added_sizes {
                    valid = valid || size == self.shadow_stack.len() - added_size;
                }
                true
            }
            None => false,
        }
    }

    fn pop_assert(&mut self, added_size: usize) -> bool {
        let size_opt = self.assert_stack.pop_back();
        match size_opt {
            Some(size) => size == self.shadow_stack.len() - added_size,
            None => false,
        }
    }

    fn push_comment(&mut self, string: &str) {
        _ = write!(&mut self.body, "\n\t ; {}", string);
    }

    fn push_instructions(&mut self, strings: Vec<&str>) {
        strings
            .iter()
            .for_each(|string| self.push_instruction(string));
    }

    fn push_label(&mut self, string: &str) {
        _ = write!(&mut self.body, "\n{}:", string);
    }

    fn push_instruction(&mut self, string: &str) {
        _ = write!(&mut self.body, "\n\t{}", string);
    }

    fn add_shadow_expr(&mut self, expr: &Expression<'a>) {
        self.add_shadow_type(&expr.resolved_type);
    }

    fn add_shadow_type(&mut self, typ: &Type<'a>) {
        self.shadow_stack.push_back(Shadow::Item(typ.clone()));
        self.stack_size += Self::get_type_stack_size(typ);
    }

    fn remove_shadow(&mut self) {
        let index = self
            .shadow_stack
            .iter()
            .enumerate()
            .rev()
            .find(|(_, element)| matches!(element, Shadow::Item(..)))
            .map(|(i, _)| i)
            .unwrap_or_else(|| unreachable!("popped with no elements"));
        match self.shadow_stack.remove(index).unwrap() {
            Shadow::Item(element) => {
                self.stack_size -= Self::get_type_stack_size(&element);
            }
            _ => unreachable!(),
        }
    }

    fn pad_shadow(&mut self) {
        self.push_comment("check align");
        let align = self.stack_size % 16 == 0;
        self.shadow_stack.push_back(Shadow::Padding(align));
        if align {
            self.stack_size += 8;
            self.push_instruction("sub rsp, 8");
        }
    }

    fn unpad_shadow(&mut self) {
        let index = self
            .shadow_stack
            .iter()
            .enumerate()
            .rev()
            .find(|(_, element)| matches!(element, Shadow::Padding(..)))
            .map(|(i, _)| i)
            .unwrap_or_else(|| unreachable!("popped with no elements"));
        match self.shadow_stack.remove(index).unwrap() {
            Shadow::Padding(true) => {
                self.stack_size -= 8;
                self.push_instruction("add rsp, 8")
            }
            Shadow::Padding(false) => self.push_comment("no unpad"),
            _ => unreachable!(),
        }
    }

    fn print_shadow_stack(&mut self) {
        self.push_comment(&format!(
            "shadow stack #{}#: {}",
            self.stack_size,
            &self
                .shadow_stack
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }

    fn pad_shadow_with_all(&mut self, arguments: &[Expression<'a>]) {
        let float_params_on_stack = arguments
            .iter()
            .filter(|arg| matches!(arg.resolved_type, Type::Float))
            .skip(FLO_REGS.len())
            .map(|arg| arg.resolved_type.clone())
            .collect::<Vec<_>>();
        let int_params = arguments
            .iter()
            .filter(|arg| matches!(arg.resolved_type, Type::Int))
            .skip(INT_REGS.len())
            .map(|arg| arg.resolved_type.clone())
            .collect::<Vec<_>>();
        let arr_params = arguments
            .iter()
            .filter(|arg| matches!(arg.resolved_type, Type::Array { .. }))
            .map(|arg| arg.resolved_type.clone())
            .collect::<Vec<_>>();
        let all_stack_params = float_params_on_stack
            .into_iter()
            .chain(int_params)
            .chain(arr_params)
            .collect::<Vec<_>>();
        _ = all_stack_params.iter().map(|t| self.add_shadow_type(t));
        self.pad_shadow();
        _ = all_stack_params.iter().map(|_| self.remove_shadow());
    }

    fn pad_shadow_with(&mut self, arguments: &Expression<'a>) {
        self.add_shadow_type(&arguments.resolved_type);
        self.pad_shadow();
        self.remove_shadow();
    }

    fn get_type_stack_size(typ: &Type<'a>) -> isize {
        match typ {
            Type::Int | Type::Float | Type::Bool | Type::Void => 8,
            Type::Array {
                element_type: _,
                rank,
            } => 8 + 8 * *rank as isize,
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
}

impl Display for AsmFunction<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.body)
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
    functions: Vec<AsmFunction<'a>>,
    constants: HashMap<AssemblyValue, String>,
    jump_counter: usize,
    offsets: HashMap<String, ((usize, Type<'a>), isize, bool)>,
}

impl Display for AssemblyGenerator<'_> {
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
        }
    }

    pub fn generate_assembly(
        &mut self,
        commands: Vec<Command<'a>>,
        environment: TypeEnvironment<'a>,
    ) -> String {
        let mut main_function = AsmFunction::new();
        main_function.push_assert();
        main_function.push_label("jpl_main");
        main_function.push_label("_jpl_main");
        main_function.push_instructions(vec![
            "push rbp",
            "mov rbp, rsp",
            "push r12",
            "mov r12, rbp",
        ]);
        main_function.add_shadow_type(&Type::Int); //rbp
        main_function.add_shadow_type(&Type::Int); //r12
        main_function.push_comment("end of jpl_main prelude");
        for command in commands {
            main_function.push_assert();
            self.generate_command(&mut main_function, &command, &environment);
            main_function.print_shadow_stack();
            main_function.print_shadow_stack();
            assert!(
                main_function.pop_assert_opt(vec![0, 1]),
                "assertion failed for command {}\n",
                command
            );
        }
        if main_function.stack_size > 16 {
            main_function.push_instruction(&format!("add rsp, {}", main_function.stack_size - 16));
            while main_function.stack_size > 16 {
                main_function.remove_shadow();
            }
        }
        main_function.push_instructions(vec!["pop r12", "pop rbp", "ret"]);
        main_function.remove_shadow();
        main_function.remove_shadow();
        main_function.print_shadow_stack();
        assert!(main_function.pop_assert(0), "\n\n{}", main_function.body);
        format!(
            "global jpl_main\nglobal _jpl_main\nextern _fail_assertion\nextern _jpl_alloc\nextern _get_time\nextern _show\nextern _print\nextern _print_time\nextern _read_image\nextern _write_image\nextern _fmod\nextern _sqrt\nextern _exp\nextern _sin\nextern _cos\nextern _tan\nextern _asin\nextern _acos\nextern _atan\nextern _log\nextern _pow\nextern _atan2\nextern _to_int\nextern _to_float{}{}",
            self,
            main_function
        )
    }

    fn generate_statement(
        &mut self,
        asm_function: &mut AsmFunction<'a>,
        statement: &Statement<'a>,
        environment: &TypeEnvironment<'a>,
    ) {
        match &statement.node {
            StatementType::Return { value } => {
                self.generate_expression(asm_function, value, environment, true);
                asm_function.remove_shadow();
                let return_type = value.resolved_type.clone();
                match return_type {
                    Type::Int | Type::Bool => {
                        asm_function.push_instruction("pop rax");
                    }
                    Type::Float => {
                        asm_function.push_instructions(vec!["movsd xmm0, [rsp]", "add rsp, 8"]);
                    }
                    Type::Array {
                        rank,
                        element_type: _,
                    } => {
                        // sussy
                        asm_function.push_instruction("mov rax, [rbp - 8]");
                        for i in (0..rank + 1).rev() {
                            asm_function.push_instruction(&format!("mov r10, [rsp + {}]", i * 8));
                            asm_function.push_instruction(&format!("mov [rax + {}], r10", i * 8));
                        }
                        asm_function.add_shadow_type(&return_type);
                    }
                    _ => unimplemented!(
                        "\n\nfailure for function return type {}",
                        return_type.to_string()
                    ),
                }
                asm_function.push_instruction(&format!("add rsp, {}", asm_function.stack_size - 8));
                match return_type {
                    Type::Array { .. } => {
                        asm_function.remove_shadow();
                    }
                    _ => {}
                }
                asm_function.push_instructions(vec!["pop rbp", "ret"]);
            }
            StatementType::Let { variable, rvalue } => {
                self.handle_let(asm_function, environment, variable, rvalue, true);
            }
            _ => unimplemented!("\n\nfailure because of statement {}", statement.to_string()),
        }
    }

    fn generate_command(
        &mut self,
        asm_function: &mut AsmFunction<'a>,
        command: &Command<'a>,
        environment: &TypeEnvironment<'a>,
    ) {
        match command.node.as_ref() {
            CommandType::Show { expression } => {
                asm_function.push_assert();
                asm_function.print_shadow_stack();
                asm_function.pad_shadow_with(expression);
                let expr_result =
                    self.generate_expression(asm_function, expression, environment, false);
                let size = expr_result.0;
                let typ_str = expr_result.1.to_string();
                let const_name = self.get_constant(AssemblyValue::String(typ_str));
                // asm_function.push_str(&format!("\n\tlea rdi, [rel {}]", const_name));
                asm_function.push_instruction(&format!("lea rdi, [rel {}]", const_name));
                // self.shadow_stack.push_back((8,false,None));
                asm_function.add_shadow_type(&Type::Int);

                // asm_function.push_str("\n\tlea rsi, [rsp]");
                asm_function.push_instruction("lea rsi, [rsp]");
                // asm_function.push_str("\n\tcall _show");
                asm_function.push_instruction("call _show");
                // self.shadow_stack.pop_back();
                asm_function.remove_shadow();
                // asm_function.push_str(&format!("\n\tadd rsp, {}", size));
                asm_function.push_instruction(&format!("add rsp, {}", size));
                // self.check_remove_alignment(asm_function);
                asm_function.unpad_shadow();
                // self.shadow_stack.pop_back();
                // asm_function.unpad_shadow();
                asm_function.remove_shadow();
                asm_function.print_shadow_stack();
                assert!(asm_function.pop_assert(0), "\n\n{}", asm_function.body);
            }
            CommandType::Let { variable, rvalue } => {
                self.handle_let(asm_function, environment, variable, rvalue, false);
            }
            CommandType::Function {
                name,
                parameters,
                return_type,
                statements,
                has_return: _,
            } => {
                self.fun_name(environment, name, parameters, return_type, statements);
            }
            _ => unimplemented!("\n\nfailure for command type {}", command.to_string()),
        }
    }

    fn fun_name(
        &mut self,
        environment: &TypeEnvironment<'a>,
        name: &&str,
        parameters: &Vec<(LValue<'a>, Type<'a>)>,
        return_type: &Type<'a>,
        statements: &Vec<Statement<'a>>,
    ) {
        let mut new_asm_function = AsmFunction::new();
        new_asm_function.push_label(name);
        new_asm_function.push_label(&format!("_{}", name));
        new_asm_function.push_instructions(vec!["push rbp", "mov rbp, rsp"]);
        new_asm_function.add_shadow_type(&Type::Int);
        let mut int_param_num = 0;
        let mut flo_param_num = 0;
        let mut arr_param_size = 0;
        match return_type {
            Type::Array { .. } => {
                new_asm_function.push_instruction(&format!("push {}", INT_REGS[int_param_num]));
                new_asm_function.add_shadow_type(&Type::Int);
                int_param_num += 1;
            }
            _ => {}
        }
        for param in parameters {
            match &param.1 {
                Type::Int | Type::Bool => {
                    new_asm_function.push_instruction(&format!("push {}", INT_REGS[int_param_num]));
                    new_asm_function.add_shadow_type(&param.1);
                    self.offsets.insert(
                        param.0.name.to_string(),
                        (
                            (
                                AsmFunction::get_type_stack_size(&param.1) as usize,
                                param.1.clone(),
                            ),
                            new_asm_function.stack_size,
                            false,
                        ),
                    );
                    int_param_num += 1;
                }
                Type::Float => {
                    new_asm_function.push_instructions(vec![
                        "sub rsp, 8",
                        &format!("movsd [rsp], {}", FLO_REGS[flo_param_num]),
                    ]);
                    flo_param_num += 1;
                    let size = AsmFunction::get_type_stack_size(&param.1);
                    new_asm_function.add_shadow_type(&param.1);
                    self.offsets.insert(
                        param.0.name.to_string(),
                        (
                            (size as usize, param.1.clone()),
                            new_asm_function.stack_size,
                            false,
                        ),
                    );
                }
                Type::Array {
                    element_type,
                    rank: _,
                } => {
                    match element_type.as_ref() {
                        Type::Int | Type::Bool | Type::Float => {}
                        _ => {
                            unimplemented!(
                                "TODO: element type array not supported: ({})",
                                element_type.to_string()
                            )
                        }
                    }
                    match &param.0.node {
                        LValueType::Array { indices } => {
                            let start = -(arr_param_size + 8);
                            let size = AsmFunction::get_type_stack_size(&param.1);
                            self.offsets.insert(
                                param.0.name.to_string(),
                                ((size as usize, param.1.clone()), start, false),
                            );
                            arr_param_size += size;
                            for (i, b_name) in indices.iter().enumerate() {
                                let stack_location = start - (i * 8) as isize;
                                self.offsets.insert(
                                    b_name.to_string(),
                                    ((8, Type::Int), stack_location, false),
                                );
                            }
                        }
                        LValueType::Variable { .. } => {
                            let size = AsmFunction::get_type_stack_size(&param.1);
                            self.offsets.insert(
                                param.0.name.to_string(),
                                (
                                    (size as usize, param.1.clone()),
                                    -(arr_param_size + 8),
                                    false,
                                ),
                            );
                            arr_param_size += size;
                        }
                    }
                }
                _ => unimplemented!("TODO: struct param types ({})", param.1.to_string()),
            }
        }
        for statement in statements {
            self.generate_statement(&mut new_asm_function, statement, environment);
        }
        self.functions.push(new_asm_function);
    }

    fn handle_let(
        &mut self,
        asm_function: &mut AsmFunction<'a>,
        environment: &TypeEnvironment<'a>,
        variable: &LValue<'_>,
        rvalue: &Expression<'a>,
        in_statement: bool,
    ) {
        match &variable.node {
            LValueType::Variable => {
                asm_function.push_assert();
                let res = self.generate_expression(asm_function, rvalue, environment, in_statement);
                self.offsets.insert(
                    variable.name.to_string(),
                    (res, asm_function.stack_size, !in_statement),
                );
                asm_function.print_shadow_stack();
                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
            }
            LValueType::Array { indices } => {
                asm_function.push_assert();
                let res = self.generate_expression(asm_function, rvalue, environment, in_statement);
                self.offsets.insert(
                    variable.name.to_string(),
                    (res.clone(), asm_function.stack_size, !in_statement),
                );
                for (i, b_name) in indices.iter().enumerate() {
                    let loc = 8 * i as isize;
                    let place = 16 - res.0 as isize;
                    let stack_location = loc + place + asm_function.stack_size;
                    self.offsets.insert(
                        b_name.to_string(),
                        ((8, Type::Int), stack_location, !in_statement),
                    );
                }
                asm_function.print_shadow_stack();
                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
            }
        }
    }

    /// Generate assembly code for an expression
    /// Location of generated expression is always rax
    /// Returns size of expression in bytes
    fn generate_expression(
        &mut self,
        asm_function: &mut AsmFunction<'a>,
        expression: &Expression<'a>,
        environment: &TypeEnvironment<'a>,
        in_statement: bool,
    ) -> (usize, Type<'a>) {
        match expression.node.as_ref() {
            ExpressionType::Int { value } => {
                asm_function.print_shadow_stack();
                asm_function.push_assert();
                let constant = self.get_constant(AssemblyValue::Number(value.to_string()));
                asm_function
                    .push_instructions(vec![&format!("mov rax, [rel {}]", constant), "push rax"]);
                asm_function.add_shadow_type(&expression.resolved_type);
                asm_function.print_shadow_stack();
                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
                (8, Type::Int)
            }
            ExpressionType::Float { value } => {
                asm_function.push_assert();
                let constant = self.get_constant(AssemblyValue::Number(format!("{:?}", value)));
                asm_function
                    .push_instructions(vec![&format!("mov rax, [rel {}]", constant), "push rax"]);
                asm_function.add_shadow_type(&expression.resolved_type);
                asm_function.print_shadow_stack();
                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
                (8, Type::Float)
            }
            ExpressionType::True => {
                asm_function.push_assert();
                let constant = self.get_constant(AssemblyValue::Number("1".to_string()));
                asm_function
                    .push_instructions(vec![&format!("mov rax, [rel {}]", constant), "push rax"]);
                asm_function.add_shadow_type(&expression.resolved_type);
                asm_function.print_shadow_stack();
                asm_function.print_shadow_stack();
                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
                (8, Type::Bool)
            }
            ExpressionType::False => {
                asm_function.push_assert();
                let constant = self.get_constant(AssemblyValue::Number("0".to_string()));
                asm_function
                    .push_instructions(vec![&format!("mov rax, [rel {}]", constant), "push rax"]);
                asm_function.add_shadow_type(&expression.resolved_type);
                asm_function.print_shadow_stack();
                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
                (8, Type::Bool)
            }
            ExpressionType::Unop {
                operator: _,
                expression,
            } => {
                asm_function.push_assert();
                asm_function.push_assert();
                let (size, typ) =
                    self.generate_expression(asm_function, expression, environment, in_statement);
                asm_function.print_shadow_stack();
                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);

                match typ {
                    Type::Int => {
                        asm_function.push_instructions(vec!["pop rax", "neg rax", "push rax"]);
                    }
                    Type::Float => {
                        asm_function.push_instructions(vec![
                            "movsd xmm1, [rsp]",
                            "add rsp, 8",
                            "pxor xmm0, xmm0",
                            "subsd xmm0, xmm1",
                            "sub rsp, 8",
                            "movsd [rsp], xmm0",
                        ]);
                    }
                    Type::Bool => {
                        asm_function.push_instructions(vec!["pop rax", "xor rax, 1", "push rax"]);
                    }
                    _ => unreachable!(),
                }
                asm_function.print_shadow_stack();
                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
                (size, typ)
            }
            ExpressionType::Binop {
                operator,
                left,
                right,
            } => {
                asm_function.push_assert();
                if *operator == "%" && matches!(left.resolved_type, Type::Float) {
                    asm_function.pad_shadow();
                }
                asm_function.push_assert();
                self.generate_expression(asm_function, right, environment, in_statement);
                asm_function.print_shadow_stack();
                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
                asm_function.push_assert();
                let (_, left_type) =
                    self.generate_expression(asm_function, left, environment, in_statement);
                asm_function.print_shadow_stack();
                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
                let result = match expression.resolved_type {
                    Type::Int => self.generate_int_op(asm_function, operator),
                    Type::Float => self.generate_float_op(asm_function, operator),
                    Type::Bool => match left_type {
                        Type::Int => self.generate_bool_op(asm_function, operator, Type::Int),
                        Type::Float => self.generate_bool_op(asm_function, operator, Type::Float),
                        Type::Bool => self.generate_bool_op(asm_function, operator, Type::Bool),
                        _ => unreachable!(),
                    },
                    _ => unreachable!(),
                };
                asm_function.print_shadow_stack();
                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
                result
            }
            ExpressionType::ArrayLiteral { elements } => {
                asm_function.push_assert();
                let mut element_size = 0;
                for element in elements.iter().rev() {
                    let (size, _) =
                        self.generate_expression(asm_function, element, environment, in_statement);
                    element_size = size;
                }
                asm_function
                    .push_instruction(&format!("mov rdi, {}", elements.len() * element_size));
                asm_function.pad_shadow();
                asm_function.push_instruction("call _jpl_alloc");
                asm_function.unpad_shadow();
                for i in (0..elements.len() * element_size / 8).rev() {
                    let loc = i * 8;
                    asm_function.push_instructions(vec![
                        &format!("mov r10, [rsp + {}]", loc),
                        &format!("mov [rax + {}], r10", loc),
                    ]);
                }
                let data_size = elements.len() * element_size;
                asm_function.push_instruction(&format!("add rsp, {}", data_size));
                for _ in 0..elements.len() {
                    asm_function.remove_shadow();
                }
                asm_function.push_instruction("push rax");
                asm_function.push_instruction(&format!("mov rax, {}", elements.len()));
                asm_function.push_instruction("push rax");
                asm_function.add_shadow_expr(expression);
                asm_function.print_shadow_stack();
                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
                (16, expression.resolved_type.clone())
            }
            ExpressionType::Variable { name } => {
                asm_function.push_assert();
                let ret = self.handle_variable(asm_function, name, expression, in_statement);
                asm_function.print_shadow_stack();
                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
                ret
            }
            ExpressionType::Call {
                function,
                arguments,
            } => {
                let ret_type = expression.resolved_type.clone();
                asm_function.push_assert();
                let mut int_arg_num = 0;
                let mut flo_arg_num = 0;
                let mut arr_arg_size = 0;
                match &expression.resolved_type {
                    Type::Int | Type::Bool | Type::Float => {
                        asm_function.pad_shadow();
                    }
                    Type::Array { .. } => {
                        asm_function.add_shadow_type(&ret_type);
                        asm_function.pad_shadow_with_all(arguments);
                        asm_function.push_instruction(&format!(
                            "sub rsp, {}",
                            AsmFunction::get_type_stack_size(&ret_type)
                        ));
                    }
                    _ => {
                        unimplemented!("\n\nfailure for call return type {}", ret_type.to_string())
                    }
                }
                for arg in arguments.iter().rev() {
                    match arg.resolved_type {
                        Type::Array { .. } => {
                            self.generate_expression(asm_function, arg, environment, in_statement);
                        }
                        _ => {}
                    }
                }
                for arg in arguments.iter().rev() {
                    match arg.resolved_type {
                        Type::Array { .. } => {}
                        _ => {
                            self.generate_expression(asm_function, arg, environment, in_statement);
                        }
                    }
                }
                for arg in arguments.iter() {
                    match &arg.resolved_type {
                        Type::Int | Type::Bool => {
                            asm_function
                                .push_instruction(&format!("pop {}", INT_REGS[int_arg_num]));
                            int_arg_num += 1;
                            asm_function.remove_shadow();
                        }
                        Type::Float => {
                            asm_function.push_instruction(&format!(
                                "movsd {}, [rsp]",
                                FLO_REGS[flo_arg_num]
                            ));
                            asm_function.push_instruction("add rsp, 8");
                            flo_arg_num += 1;
                            asm_function.remove_shadow();
                        }
                        Type::Array { .. } => {
                            arr_arg_size += AsmFunction::get_type_stack_size(&arg.resolved_type);
                        }
                        _ => unimplemented!(
                            "arg type not supported: {}",
                            arg.resolved_type.to_string()
                        ),
                    }
                }
                match &expression.resolved_type {
                    Type::Int | Type::Bool | Type::Float => {}
                    Type::Array { .. } => {
                        asm_function
                            .push_instruction(&format!("lea rdi, [rsp + {}]", arr_arg_size));
                    }
                    _ => {
                        unimplemented!("\n\nfailure for call return type {}", ret_type.to_string())
                    }
                }
                asm_function.push_instruction(&format!("call _{}", function));
                for arg in arguments.iter() {
                    match &arg.resolved_type {
                        Type::Array {
                            element_type,
                            rank: _,
                        } => {
                            match element_type.as_ref() {
                                Type::Int | Type::Bool | Type::Float => {}
                                _ => {
                                    unimplemented!(
                                        "TODO: element type array not supported: ({})",
                                        element_type.to_string()
                                    )
                                }
                            }
                            let size = AsmFunction::get_type_stack_size(&arg.resolved_type);
                            asm_function.push_instruction(&format!("add rsp, {}", size));
                            asm_function.remove_shadow();
                        }
                        Type::Int | Type::Bool | Type::Float => {}
                        _ => unimplemented!("unsupported arg type"),
                    }
                }
                asm_function.unpad_shadow();

                match &expression.resolved_type {
                    Type::Int | Type::Bool | Type::Float => {
                        asm_function.add_shadow_type(&ret_type);
                    }
                    Type::Array { .. } => {}
                    _ => {
                        unimplemented!("\n\nfailure for call return type {}", ret_type.to_string())
                    }
                }
                match &expression.resolved_type {
                    Type::Int | Type::Bool => {
                        asm_function.push_instruction("push rax");
                    }
                    Type::Float => {
                        asm_function.push_instructions(vec!["sub rsp, 8", "movsd [rsp], xmm0"]);
                    }
                    Type::Array { .. } => {
                        asm_function.push_comment("array value in stack allocated placeholder");
                    }
                    _ => {
                        unimplemented!("\n\nfailure for call return type {}", ret_type.to_string())
                    }
                }
                asm_function.print_shadow_stack();
                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
                let size = AsmFunction::get_type_stack_size(&ret_type);
                (size as usize, ret_type)
            }
            _ => unimplemented!("failure because for expression: {}", expression.to_string()),
        }
    }

    fn handle_variable(
        &mut self,
        asm_function: &mut AsmFunction<'a>,
        name: &str,
        expression: &Expression<'a>,
        in_statement: bool,
    ) -> (usize, Type<'a>) {
        match expression.resolved_type {
            Type::Int | Type::Bool | Type::Float => {
                asm_function.push_assert();
                let (res, offset, from_main) = self.offsets.get(name).unwrap_or_else(|| {
                    unimplemented!("expression was this: {}", expression.to_string())
                });
                asm_function.push_instruction(&format!("sub rsp, {}", res.0));
                let var_offset_reg = if *from_main && in_statement {
                    "r12"
                } else {
                    "rbp"
                };
                asm_function.push_instruction(&format!(
                    "mov r10, [{} - {}]",
                    var_offset_reg,
                    offset - 8
                ));
                asm_function.push_instruction("mov [rsp + 0], r10");
                asm_function.add_shadow_type(&expression.resolved_type);
                asm_function.print_shadow_stack();
                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
                res.clone()
            }
            Type::Array {
                element_type: _,
                rank,
            } => {
                asm_function.push_assert();
                let (res, offset, from_main) = self.offsets.get(name).unwrap_or_else(|| {
                    unimplemented!("expression was this: {}", expression.to_string())
                });
                asm_function.add_shadow_type(&expression.resolved_type);
                asm_function.push_instruction(&format!("sub rsp, {}", res.0));
                let var_offset_reg = if *from_main && in_statement {
                    "r12"
                } else {
                    "rbp"
                };
                for i in (0..rank + 1).rev() {
                    let loc = i * 8;
                    let bounds_offset = offset - 8;
                    asm_function.push_instruction(&format!(
                        "mov r10, [{} - {} + {}]",
                        var_offset_reg, bounds_offset, loc
                    ));
                    asm_function.push_instruction(&format!("mov [rsp + {}], r10", loc));
                }
                asm_function.print_shadow_stack();
                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
                res.clone()
            }
            _ => unreachable!(),
        }
    }

    fn generate_int_op(
        &mut self,
        asm_function: &mut AsmFunction<'a>,
        operator: &str,
    ) -> (usize, Type<'a>) {
        match operator {
            "+" => {
                asm_function.push_instructions(vec![
                    "pop rax",
                    "pop r10",
                    "add rax, r10",
                    "push rax",
                ]);
                asm_function.remove_shadow();
                asm_function.remove_shadow();
                asm_function.add_shadow_type(&Type::Int);
            }
            "-" => {
                asm_function.push_instructions(vec![
                    "pop rax",
                    "pop r10",
                    "sub rax, r10",
                    "push rax",
                ]);
                asm_function.remove_shadow();
                asm_function.remove_shadow();
                asm_function.add_shadow_type(&Type::Int);
            }
            "*" => {
                asm_function.push_instructions(vec![
                    "pop rax",
                    "pop r10",
                    "imul rax, r10",
                    "push rax",
                ]);
                asm_function.remove_shadow();
                asm_function.remove_shadow();
                asm_function.add_shadow_type(&Type::Int);
            }
            "/" | "%" => {
                asm_function.push_instructions(vec!["pop rax", "pop r10", "cmp r10, 0"]);
                asm_function.remove_shadow();
                asm_function.remove_shadow();

                let jump_label = &format!(".jump{}", self.jump_counter);
                self.jump_counter += 1;
                asm_function.push_instruction(&format!("jne {}", jump_label));
                asm_function.pad_shadow();
                asm_function.push_instruction(&format!(
                    "lea rdi, [rel {}]",
                    self.get_constant(AssemblyValue::String(
                        if operator == "/" {
                            "divide by zero"
                        } else {
                            "mod by zero"
                        }
                        .to_string()
                    ))
                ));
                asm_function.push_instruction("call _fail_assertion");
                asm_function.unpad_shadow();
                asm_function.push_label(jump_label);
                asm_function.push_instructions(vec!["cqo", "idiv r10"]);

                if operator == "%" {
                    asm_function.push_instruction("mov rax, rdx");
                }
                asm_function.add_shadow_type(&Type::Int);
                asm_function.push_instruction("push rax");
            }
            _ => {
                unreachable!();
            }
        }
        asm_function.push_comment("int op");
        (8, Type::Int)
    }

    fn generate_float_op(
        &mut self,
        asm_function: &mut AsmFunction<'a>,
        operator: &str,
    ) -> (usize, Type<'a>) {
        match operator {
            "+" | "-" | "*" | "/" => {
                let op_instruction = match operator {
                    "+" => "addsd",
                    "-" => "subsd",
                    "*" => "mulsd",
                    "/" => "divsd",
                    _ => unreachable!(),
                };
                asm_function.push_instructions(vec![
                    "movsd xmm0, [rsp]",
                    "add rsp, 8",
                    "movsd xmm1, [rsp]",
                    "add rsp, 8",
                    &format!("{} xmm0, xmm1", op_instruction),
                    "sub rsp, 8",
                    "movsd [rsp], xmm0",
                ]);
            }
            "%" => {
                asm_function.push_instructions(vec![
                    "movsd xmm0, [rsp]",
                    "add rsp, 8",
                    "movsd xmm1, [rsp]",
                    "add rsp, 8",
                    "call _fmod",
                ]);
                asm_function.unpad_shadow();

                asm_function.push_instructions(vec!["sub rsp, 8", "movsd [rsp], xmm0"]);
            }
            _ => unreachable!(),
        }
        asm_function.remove_shadow();
        asm_function.remove_shadow();
        asm_function.add_shadow_type(&Type::Float);
        (8, Type::Float)
    }

    fn generate_bool_op(
        &mut self,
        asm_function: &mut AsmFunction<'a>,
        operator: &str,
        left_type: Type<'a>,
    ) -> (usize, Type<'a>) {
        match left_type {
            Type::Int => self.generate_int_comparison(asm_function, operator),
            Type::Float => self.generate_float_comparison(asm_function, operator),
            Type::Bool => self.generate_bool_comparison(asm_function, operator),
            _ => unreachable!(),
        }
        asm_function.remove_shadow();
        asm_function.remove_shadow();
        asm_function.add_shadow_type(&Type::Bool);
        (8, Type::Bool)
    }

    fn generate_int_comparison(&self, asm_function: &mut AsmFunction<'a>, operator: &str) {
        let set_instruction = match operator {
            "<" => "setl",
            "<=" => "setle",
            ">" => "setg",
            ">=" => "setge",
            "==" => "sete",
            "!=" => "setne",
            _ => unreachable!(),
        };

        asm_function.push_instructions(vec![
            "pop rax",
            "pop r10",
            "cmp rax, r10",
            &format!("{} al", set_instruction),
            "and rax, 1",
            "push rax",
        ]);
    }

    fn generate_float_comparison(&self, asm_function: &mut AsmFunction<'a>, operator: &str) {
        asm_function.push_instructions(vec![
            "movsd xmm0, [rsp]",
            "add rsp, 8",
            "movsd xmm1, [rsp]",
            "add rsp, 8",
        ]);

        let (cmp_instruction, swap_operands) = match operator {
            "<" => ("cmpltsd", false),
            "<=" => ("cmplesd", false),
            ">" => ("cmpltsd", true),
            ">=" => ("cmplesd", true),
            "==" => ("cmpeqsd", false),
            "!=" => ("cmpneqsd", false),
            _ => unreachable!(),
        };

        if swap_operands {
            asm_function.push_instructions(vec![
                &format!("{} xmm1, xmm0", cmp_instruction),
                "movq rax, xmm1",
                "and rax, 1",
                "push rax",
                ";float end",
            ]);
        } else {
            asm_function.push_instructions(vec![
                &format!("{} xmm0, xmm1", cmp_instruction),
                "movq rax, xmm0",
                "and rax, 1",
                "push rax",
                ";float end",
            ]);
        };
    }

    fn generate_bool_comparison(&self, asm_function: &mut AsmFunction<'a>, operator: &str) {
        match operator {
            "&&" => asm_function.push_instructions(vec![
                "pop rax",
                "pop r10",
                "and rax, r10",
                "push rax",
            ]),
            "||" => asm_function.push_instructions(vec![
                "pop rax",
                "pop r10",
                "or rax, r10",
                "push rax",
            ]),
            "==" | "!=" => {
                asm_function.push_instructions(vec![
                    "pop rax",
                    "pop r10",
                    "cmp rax, r10",
                    &format!("{} al", if operator == "==" { "sete" } else { "setne" }),
                    "and rax, 1",
                    "push rax",
                ]);
            }
            _ => unreachable!(),
        }
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
) -> String {
    let mut generator = AssemblyGenerator::new();
    generator.generate_assembly(commands, environment)
}
