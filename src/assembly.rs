use crate::ast::*;
use petgraph::algo::toposort;
use petgraph::graphmap::DiGraphMap;
use regex::Regex;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::fmt;
use std::fmt::Display;
use std::fmt::Write;
use std::vec;

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
            Shadow::Item(typ) => write!(f, "{}:[{}]", typ, typ.isize()),
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
        _ = write!(&mut self.body, "\n\t; {}", string);
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
        self.stack_size += typ.isize();
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
                self.stack_size -= element.isize();
            }
            _ => unreachable!(),
        }
    }

    fn pad_shadow(&mut self) -> bool {
        let align = self.stack_size % 16 == 8;
        self.shadow_stack.push_back(Shadow::Padding(align));
        if align {
            self.stack_size += 8;
            self.push_instruction("sub rsp, 8; padding");
        } else {
            self.push_comment("no pad");
        }
        align
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

    fn pad_shadow_with_all(&mut self, arguments: &[Expression<'a>]) -> bool {
        let mut total_stack_param_size = 0;
        let mut int_params = 0;
        let mut float_params = 0;

        for param in arguments {
            match param.resolved_type {
                Type::Int | Type::Bool => {
                    int_params += 1;
                    if int_params > INT_REGS.len() {
                        total_stack_param_size += 8;
                    }
                }
                Type::Float => {
                    float_params += 1;
                    if float_params > FLO_REGS.len() {
                        total_stack_param_size += 8;
                    }
                }
                Type::Array { .. } => {
                    total_stack_param_size += param.resolved_type.isize();
                }
                _ => unreachable!(),
            }
        }

        self.stack_size += total_stack_param_size;
        let padded = self.pad_shadow();
        self.stack_size -= total_stack_param_size;
        padded
    }

    fn pad_shadow_with(&mut self, arguments: &Expression<'a>) {
        self.add_shadow_type(&arguments.resolved_type);
        self.pad_shadow();
        self.remove_shadow();
    }
}

impl Display for AsmFunction<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.body)
    }
}

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
    offsets: HashMap<String, (isize, bool)>,
    optimization_level: u8,
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
    pub fn new(optimization_level: u8) -> Self {
        Self {
            data_section: Vec::new(),
            functions: Vec::new(),
            constants: HashMap::new(),
            offsets: HashMap::new(),
            jump_counter: 1,
            optimization_level,
        }
    }

    pub fn generate_assembly(&mut self, commands: Vec<Command<'a>>) -> String {
        let mut main_function = AsmFunction::new();
        self.offsets.insert("args".to_string(), (-16, false));
        self.offsets.insert("argnum".to_string(), (-16, false));
        main_function.push_assert();
        main_function.push_label("jpl_main");
        main_function.push_label("_jpl_main");
        main_function.push_instructions(vec![
            "push rbp",
            "mov rbp, rsp",
            "push r12",
            "mov r12, rbp",
        ]);
        main_function.add_shadow_type(&Type::Int); //r12
        main_function.push_comment("end of jpl_main prelude");
        for command in commands {
            main_function.push_assert();
            self.generate_command(&mut main_function, &command);
            main_function.print_shadow_stack();
            assert!(
                main_function.pop_assert_opt(vec![0, 1]),
                "assertion failed for command {}\n",
                command
            );
        }
        // Very sussy
        if main_function.stack_size > 8 {
            main_function.push_instruction(&format!("add rsp, {}", main_function.stack_size - 8));
            while main_function.stack_size > 8 {
                main_function.remove_shadow();
            }
        }

        main_function.push_instructions(vec!["pop r12", "pop rbp", "ret"]);
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
    ) {
        match &statement.node {
            StatementType::Return { value } => {
                self.generate_expression(asm_function, value, true);
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
                asm_function.push_comment("deallocate local variables");
                asm_function.push_instruction(&format!("add rsp, {}", asm_function.stack_size));
                asm_function.push_instructions(vec!["pop rbp", "ret"]);
            }
            StatementType::Let { variable, rvalue } => {
                self.handle_let(asm_function, variable, rvalue, true);
            }
            _ => unimplemented!("\n\nfailure because of statement {}", statement.to_string()),
        }
    }

    fn generate_command(&mut self, asm_function: &mut AsmFunction<'a>, command: &Command<'a>) {
        match command.node.as_ref() {
            CommandType::Show { expression } => {
                asm_function.push_comment("show start");
                asm_function.push_assert();
                asm_function.print_shadow_stack();
                asm_function.pad_shadow_with(expression);
                asm_function.print_shadow_stack();
                self.generate_expression(asm_function, expression, false);
                self.insert_string_constant(
                    asm_function,
                    Self::get_show_format_string(&expression.resolved_type),
                );
                asm_function.add_shadow_type(&Type::Int);
                asm_function.push_instructions(vec!["lea rsi, [rsp]", "call _show"]);
                asm_function.remove_shadow();
                asm_function
                    .push_instruction(&format!("add rsp, {}", expression.resolved_type.usize()));
                asm_function.unpad_shadow();
                asm_function.remove_shadow();
                asm_function.print_shadow_stack();
                assert!(asm_function.pop_assert(0), "\n\n{}", asm_function.body);
                asm_function.push_comment("show end");
            }
            CommandType::Let { variable, rvalue } => {
                self.handle_let(asm_function, variable, rvalue, false);
            }
            CommandType::Function {
                name,
                parameters,
                return_type,
                statements,
                has_return: _,
            } => {
                let mut new_asm_function = AsmFunction::new();
                new_asm_function.push_label(name);
                new_asm_function.push_label(&format!("_{}", name));
                new_asm_function.push_instructions(vec!["push rbp", "mov rbp, rsp"]);

                let mut int_param_num = 0;
                let mut flo_param_num = 0;
                let mut arr_param_size = 0;

                if matches!(return_type, Type::Array { .. }) {
                    new_asm_function.push_instruction(&format!("push {}", INT_REGS[int_param_num]));
                    new_asm_function.add_shadow_type(&Type::Int);
                    int_param_num += 1;
                }

                for param in parameters {
                    match &param.1 {
                        Type::Int | Type::Bool | Type::Float => {
                            if matches!(param.1, Type::Int | Type::Bool) {
                                new_asm_function
                                    .push_instruction(&format!("push {}", INT_REGS[int_param_num]));
                                int_param_num += 1;
                            } else {
                                new_asm_function.push_instructions(vec![
                                    "sub rsp, 8",
                                    &format!("movsd [rsp], {}", FLO_REGS[flo_param_num]),
                                ]);
                                flo_param_num += 1;
                            }
                            new_asm_function.add_shadow_type(&param.1);
                            self.offsets.insert(
                                param.0.name.to_string(),
                                (new_asm_function.stack_size, false),
                            );
                            new_asm_function.push_comment(&format!(
                                "saving parameter {} with offset {}",
                                param.0.name, new_asm_function.stack_size
                            ));
                        }
                        Type::Array { .. } => {
                            //sus
                            let data_pointer = -arr_param_size - 16;
                            self.offsets
                                .insert(param.0.name.to_string(), (data_pointer, false));
                            arr_param_size += param.1.isize();
                            if let LValueType::Array { indices } = &param.0.node {
                                for (i, b_name) in indices.iter().enumerate() {
                                    let loc = data_pointer - 8 * i as isize;
                                    self.offsets.insert(b_name.to_string(), (loc, false));
                                }
                            }
                        }
                        _ => todo!("struct param types ({})", param.1.to_string()),
                    }
                }

                new_asm_function.push_comment("pushed parameters");
                new_asm_function.print_shadow_stack();

                for statement in statements {
                    self.generate_statement(&mut new_asm_function, statement);
                }

                self.functions.push(new_asm_function);
            }
            CommandType::Read {
                source,
                destination,
            } => {
                asm_function.push_comment("read start");
                asm_function.push_assert();

                asm_function.push_instruction("sub rsp, 24");
                let array_type = Type::Array {
                    element_type: Box::new(Type::Struct {
                        name: "rgba",
                        elements: vec![
                            ("r", Type::Int),
                            ("g", Type::Int),
                            ("b", Type::Int),
                            ("a", Type::Int),
                        ],
                    }),
                    rank: 2,
                };
                asm_function.add_shadow_type(&array_type);

                asm_function.push_instruction("lea rdi, [rsp]");
                asm_function.pad_shadow();
                asm_function.push_instruction(&format!(
                    "lea rsi, [rel {}]",
                    self.insert_asm_constant(AssemblyValue::String(source.to_string()))
                ));
                asm_function.push_instruction("call _read_image");
                asm_function.unpad_shadow();

                self.offsets.insert(
                    destination.name.to_string(),
                    (asm_function.stack_size, true),
                );
                asm_function.push_comment(&format!(
                    "saving variable {} with offset {}",
                    destination.name, asm_function.stack_size
                ));

                if let LValueType::Array { indices } = &destination.node {
                    for (i, b_name) in indices.iter().enumerate() {
                        self.offsets.insert(
                            b_name.to_string(),
                            (asm_function.stack_size - 8 * i as isize, true),
                        );
                        asm_function.push_comment(&format!(
                            "saving variable {} with offset {}",
                            b_name,
                            asm_function.stack_size + 8 * i as isize
                        ));
                    }
                }

                asm_function.pop_assert(1);
            }
            _ => unimplemented!("\n\nfailure for command type {}", command.to_string()),
        }
    }

    fn handle_let(
        &mut self,
        asm_function: &mut AsmFunction<'a>,
        variable: &LValue<'_>,
        rvalue: &Expression<'a>,
        in_statement: bool,
    ) {
        asm_function.push_assert();
        self.generate_expression(asm_function, rvalue, in_statement);
        self.offsets.insert(
            variable.name.to_string(),
            (asm_function.stack_size, !in_statement),
        );
        asm_function.push_comment(&format!(
            "saving variable {} with offset {}",
            variable.name, asm_function.stack_size
        ));
        if let LValueType::Array { indices } = &variable.node {
            for (i, b_name) in indices.iter().enumerate() {
                self.offsets.insert(
                    b_name.to_string(),
                    (asm_function.stack_size - 8 * i as isize, !in_statement),
                );
                asm_function.push_comment(&format!(
                    "saving variable {} with offset {}",
                    b_name,
                    asm_function.stack_size + 8 * i as isize
                ));
            }
        }
        assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
    }

    fn generate_expression(
        &mut self,
        asm_function: &mut AsmFunction<'a>,
        expression: &Expression<'a>,
        in_statement: bool,
    ) {
        match expression.node.as_ref() {
            ExpressionType::Int { value } => {
                asm_function.push_assert();
                self.insert_int_constant(asm_function, *value);
                asm_function.add_shadow_type(&expression.resolved_type);
                asm_function.print_shadow_stack();
                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
            }
            ExpressionType::Float { value } => {
                asm_function.push_assert();
                self.insert_float_constant(asm_function, *value);
                asm_function.add_shadow_type(&expression.resolved_type);
                asm_function.print_shadow_stack();
                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
            }
            ExpressionType::True => {
                asm_function.push_assert();
                self.insert_int_constant(asm_function, 1);
                asm_function.add_shadow_type(&expression.resolved_type);
                asm_function.print_shadow_stack();
                asm_function.print_shadow_stack();
                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
            }
            ExpressionType::False => {
                asm_function.push_assert();
                self.insert_int_constant(asm_function, 0);
                asm_function.add_shadow_type(&expression.resolved_type);
                asm_function.print_shadow_stack();
                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
            }
            ExpressionType::Unop {
                operator: _,
                expression,
            } => {
                asm_function.push_assert();
                asm_function.push_assert();
                self.generate_expression(asm_function, expression, in_statement);
                asm_function.print_shadow_stack();
                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);

                if matches!(expression.resolved_type, Type::Int) {
                    asm_function.push_instructions(vec!["pop rax", "neg rax", "push rax"]);
                } else if matches!(expression.resolved_type, Type::Float) {
                    asm_function.push_instructions(vec![
                        "movsd xmm1, [rsp]",
                        "add rsp, 8",
                        "pxor xmm0, xmm0",
                        "subsd xmm0, xmm1",
                        "sub rsp, 8",
                        "movsd [rsp], xmm0",
                    ]);
                } else if matches!(expression.resolved_type, Type::Bool) {
                    asm_function.push_instructions(vec!["pop rax", "xor rax, 1", "push rax"]);
                }

                asm_function.print_shadow_stack();
                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
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

                // Short circuiting operators
                if *operator == "||" || *operator == "&&" {
                    asm_function.push_assert();
                    self.generate_expression(asm_function, left, in_statement);
                    assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);

                    let jump_label = format!(".jump{}", self.jump_counter);
                    self.jump_counter += 1;

                    asm_function.push_instructions(vec![
                        "pop rax",
                        "cmp rax, 0",
                        &format!(
                            "{} {}",
                            if *operator == "&&" { "je" } else { "jne" },
                            jump_label
                        ),
                    ]);

                    asm_function.remove_shadow();

                    asm_function.push_assert();
                    self.generate_expression(asm_function, right, in_statement);
                    assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);

                    asm_function.push_instruction("pop rax");
                    asm_function.push_label(&jump_label);
                    asm_function.push_instruction("push rax");

                    asm_function.remove_shadow();
                    asm_function.add_shadow_type(&Type::Bool);

                    assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
                    return;
                }

                // Multiplication optimization
                if self.optimization_level > 0 && *operator == "*" {
                    if let ExpressionType::Int { value: value_1 } = left.node.as_ref() {
                        if (Self::is_32_bit(value_1) || (*value_1 as u64).is_power_of_two())
                            && *value_1 != 0
                        {
                            self.generate_expression(asm_function, right, in_statement);

                            if let ExpressionType::Int { value: value_2 } = right.node.as_ref() {
                                if (Self::is_32_bit(value_2) || (*value_2 as u64).is_power_of_two())
                                    && *value_2 != 0
                                    && *value_1 != 1
                                    && *value_2 != 1
                                    && !(*value_1 as u64).is_power_of_two()
                                {
                                    self.generate_expression(asm_function, left, in_statement);
                                    asm_function.push_instructions(vec![
                                        "pop rax",
                                        "pop r10",
                                        "imul rax, r10",
                                        "push rax",
                                    ]);

                                    asm_function.remove_shadow();

                                    assert!(
                                        asm_function.pop_assert(1),
                                        "\n\n{}",
                                        asm_function.body
                                    );
                                    return;
                                }
                            }

                            if *value_1 != 1 {
                                asm_function.push_instructions(vec![
                                    "pop rax",
                                    &format!("imul rax, {}", value_1),
                                    "push rax",
                                ]);
                            }

                            assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
                            return;
                        }
                    } else if let ExpressionType::Int { value: value_2 } = right.node.as_ref() {
                        if (Self::is_32_bit(value_2) || (*value_2 as u64).is_power_of_two())
                            && *value_2 != 0
                        {
                            self.generate_expression(asm_function, left, in_statement);
                            if *value_2 != 1 {
                                asm_function.push_instructions(vec![
                                    "pop rax",
                                    &format!("imul rax, {}", value_2),
                                    "push rax",
                                ]);
                            }

                            assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
                            return;
                        }
                    }
                }

                asm_function.push_assert();
                self.generate_expression(asm_function, right, in_statement);
                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
                asm_function.push_assert();
                self.generate_expression(asm_function, left, in_statement);
                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);

                match expression.resolved_type {
                    Type::Int => self.generate_int_op(asm_function, operator),
                    Type::Float => self.generate_float_op(asm_function, operator),
                    Type::Bool => match left.resolved_type {
                        Type::Int => self.generate_bool_op(asm_function, operator, Type::Int),
                        Type::Float => self.generate_bool_op(asm_function, operator, Type::Float),
                        Type::Bool => self.generate_bool_op(asm_function, operator, Type::Bool),
                        _ => unreachable!(),
                    },
                    _ => unreachable!(),
                };
                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
            }
            ExpressionType::ArrayLiteral { elements } => {
                asm_function.push_comment("array literal start");
                asm_function.push_assert();
                let element_size = elements[0].resolved_type.usize();
                for element in elements.iter().rev() {
                    self.generate_expression(asm_function, element, in_statement);
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
                asm_function.push_comment("array literal end");
            }
            ExpressionType::Variable { name } => {
                asm_function.push_assert();
                self.handle_variable(asm_function, name, expression, in_statement);
                asm_function.print_shadow_stack();
                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
            }
            ExpressionType::Call {
                function,
                arguments,
            } => {
                asm_function.push_comment(&format!("calling function {}", function));
                let ret_type = expression.resolved_type.clone();
                asm_function.push_assert();
                let mut int_arg_num = 0;
                let mut flo_arg_num = 0;
                let mut arr_arg_size = 0;
                match &expression.resolved_type {
                    Type::Int | Type::Bool | Type::Float => {
                        asm_function.push_comment("check call alignment");
                        asm_function.print_shadow_stack();
                        asm_function.pad_shadow_with_all(arguments);
                        asm_function.print_shadow_stack();
                    }
                    Type::Array { .. } => {
                        int_arg_num += 1;
                        asm_function.push_instruction(&format!("sub rsp, {}", ret_type.usize()));
                        asm_function.add_shadow_type(&ret_type);
                        if asm_function.pad_shadow_with_all(arguments) {
                            arr_arg_size += 8;
                        }
                    }
                    _ => {
                        unimplemented!("\n\nfailure for call return type {}", ret_type.to_string())
                    }
                }
                for arg in arguments.iter().rev() {
                    if let Type::Array { .. } = arg.resolved_type {
                        //stack messed up here
                        asm_function.push_comment("check here");
                        asm_function.print_shadow_stack();
                        self.generate_expression(asm_function, arg, in_statement);
                    }
                }
                for arg in arguments.iter().rev() {
                    if !matches!(arg.resolved_type, Type::Array { .. }) {
                        self.generate_expression(asm_function, arg, in_statement);
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
                            arr_arg_size += arg.resolved_type.usize();
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
                    if matches!(arg.resolved_type, Type::Array { .. }) {
                        asm_function
                            .push_instruction(&format!("add rsp, {}", arg.resolved_type.usize()));
                        asm_function.remove_shadow();
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
            }
            ExpressionType::If {
                condition,
                then_branch,
                else_branch,
            } => {
                asm_function.push_assert();
                self.generate_expression(asm_function, condition, in_statement);

                // Boolean to integer casts
                if self.optimization_level > 0 {
                    // Check if the branches are integer literals that can be optimized
                    if let ExpressionType::Int { value: value_1 } = then_branch.node.as_ref() {
                        if let ExpressionType::Int { value: value_2 } = else_branch.node.as_ref() {
                            if *value_1 == 1 && *value_2 == 0 {
                                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
                                return;
                            }
                        }
                    }
                }

                asm_function.push_instruction("pop rax");
                asm_function.remove_shadow();
                asm_function.push_instruction("cmp rax, 0");

                let else_label = format!(".jump{}", self.jump_counter);
                self.jump_counter += 1;
                let end_label = format!(".jump{}", self.jump_counter);
                self.jump_counter += 1;

                asm_function.push_instruction(&format!("je {}", else_label));
                self.generate_expression(asm_function, then_branch, in_statement);
                asm_function.remove_shadow();
                asm_function.push_instruction(&format!("jmp {}", end_label));
                asm_function.push_label(&else_label);
                self.generate_expression(asm_function, else_branch, in_statement);
                asm_function.push_label(&end_label);
                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
            }
            ExpressionType::ArrayIndex { array, indices } => {
                if let ExpressionType::Variable { name } = array.node.as_ref() {
                    let (offset, from_main) = self.offsets.get(*name).unwrap();
                    if self.optimization_level > 0 && !(*from_main && in_statement) {
                        asm_function.push_comment("array index optimization");
                        return self.optimized_array_index(
                            asm_function,
                            indices,
                            expression,
                            *offset,
                            in_statement,
                        );
                    }
                }
                asm_function.push_comment("array index start");
                if let Type::Array { element_type, rank } = &array.resolved_type {
                    asm_function.push_assert();
                    let element_type = element_type.as_ref();

                    asm_function.push_comment(&format!("generating array expression {}", array));
                    self.generate_expression(asm_function, array, in_statement);

                    asm_function.push_comment("generating index expressions");
                    for index_expr in indices.iter().rev() {
                        self.generate_expression(asm_function, index_expr, in_statement);
                    }

                    asm_function.push_comment("generating bounds checks");
                    asm_function.print_shadow_stack();
                    for (index, _) in indices.iter().enumerate() {
                        asm_function.push_instructions(vec![
                            &format!("mov rax, [rsp + {}]", index * 8),
                            "cmp rax, 0",
                        ]);
                        self.assert(asm_function, "jge", "negative array index");
                        asm_function
                            .push_instruction(&format!("cmp rax, [rsp + {}]", (index + rank) * 8));
                        self.assert(asm_function, "jl", "index too large");
                    }

                    self.calculate_linear_index(
                        asm_function,
                        None,
                        indices.len(),
                        0,
                        indices.len() * 8,
                        expression.resolved_type.usize(),
                    );

                    for _ in indices.iter() {
                        asm_function.remove_shadow();
                        asm_function.push_instruction("add rsp, 8");
                    }

                    asm_function.remove_shadow();
                    asm_function
                        .push_instruction(&format!("add rsp, {}", array.resolved_type.usize()));

                    asm_function.push_instruction(&format!("sub rsp, {}", element_type.usize()));
                    asm_function.add_shadow_type(element_type);

                    asm_function.push_comment("copying data from array to stack");
                    asm_function.print_shadow_stack();
                    for i in (0..(element_type.usize())).step_by(8).rev() {
                        asm_function.push_instructions(vec![
                            &format!("mov r10, [rax + {}]", i),
                            &format!("mov [rsp + {}], r10", i),
                        ]);
                    }
                    asm_function.push_comment("array index end");
                    assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
                } else {
                    unreachable!("Expected array type")
                }
            }
            ExpressionType::ArrayLoop { range, body } => {
                // Check if the array loop can be tensor contraction optimized
                if let ExpressionType::SumLoop {
                    range: sum_range,
                    body: sum_body,
                } = body.node.as_ref()
                {
                    if self.optimization_level > 2 && Self::tensor_optimizable_expressions(sum_body)
                    {
                        self.optimized_array_loop(
                            asm_function,
                            range,
                            sum_range,
                            sum_body,
                            in_statement,
                        );
                        return;
                    }
                }

                asm_function.push_comment("array loop start");
                asm_function.print_shadow_stack();
                asm_function.push_assert();

                asm_function.push_comment("allocate 8 bytes for pointer");
                asm_function.push_instruction("sub rsp, 8"); // Allocate data pointer for array
                asm_function.add_shadow_type(&Type::Int); // temp type that will be removed and re-contextualized as an array type

                asm_function.push_comment("before checking bounds");
                asm_function.print_shadow_stack();
                asm_function.push_comment("array loop check bounds");
                self.check_loop_bounds(asm_function, in_statement, range);
                asm_function.push_comment("after checking bounds");
                asm_function.print_shadow_stack();

                // Calculate the size of the array and store it in rax
                asm_function.push_comment("calculate total array heap size");
                asm_function.push_instruction(&format!("mov rdi, {}", body.resolved_type.usize()));
                (0..range.len()).for_each(|i| {
                    asm_function.push_instruction(&format!("imul rdi, [rsp + {}]", i * 8));
                    self.assert(asm_function, "jno", "overflow computing array size");
                });

                asm_function.print_shadow_stack();

                asm_function.pad_shadow();
                asm_function.push_instruction("call _jpl_alloc");
                asm_function.unpad_shadow();
                asm_function.push_instruction(&format!("mov [rsp + {}], rax", range.len() * 8));

                asm_function.print_shadow_stack();

                // Initialize looping variables
                self.init_indices(asm_function, range);

                asm_function.print_shadow_stack();

                // Loop body
                asm_function.push_comment("loop body");
                asm_function.push_label(&format!(".jump{}", self.jump_counter));
                let continue_label = self.jump_counter;
                self.jump_counter += 1;
                self.generate_expression(asm_function, body, in_statement);

                asm_function.print_shadow_stack();

                // Calculate linear index
                self.calculate_linear_index(
                    asm_function,
                    Some(range),
                    range.len(),
                    body.resolved_type.usize(),
                    body.resolved_type.usize() + range.len() * 8,
                    body.resolved_type.usize(),
                );

                asm_function.print_shadow_stack();

                // Copy data from stack to heap
                asm_function.push_comment("copying element from stack to array");
                for i in (0..body.resolved_type.usize()).step_by(8).rev() {
                    asm_function.push_instructions(vec![
                        &format!("mov r10, [rsp + {}]", i),
                        &format!("mov [rax + {}], r10", i),
                    ]);
                }

                asm_function.print_shadow_stack();

                // Deallocate space for loop body
                asm_function.push_comment("deallocating space for loop body");
                asm_function.remove_shadow();
                asm_function.push_instruction(&format!("add rsp, {}", body.resolved_type.usize()));

                asm_function.print_shadow_stack();

                // Increment the loop index
                self.increment_loop_index(asm_function, range.len(), continue_label);

                asm_function.print_shadow_stack();

                // De-init loop variables
                asm_function.push_comment("de-init loop variables");
                (0..range.len()).for_each(|_| asm_function.remove_shadow());
                asm_function.push_instruction(&format!("add rsp, {}", 8 * range.len()));

                asm_function.print_shadow_stack();

                // Re-contextualize the array type
                asm_function.push_comment("re-contextualizing array type");
                (0..=range.len()).for_each(|_| {
                    // Remove loop bounds and data pointer types
                    asm_function.remove_shadow();
                });
                asm_function.add_shadow_type(&expression.resolved_type);

                asm_function.print_shadow_stack();

                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
            }
            ExpressionType::SumLoop { range, body } => {
                asm_function.push_comment("sum loop start");
                asm_function.print_shadow_stack();
                let resolved_type = expression.resolved_type.clone();
                asm_function.push_assert();
                asm_function.push_comment("allocate return");
                asm_function.push_instruction(&format!("sub rsp, {}", resolved_type.usize()));
                asm_function.add_shadow_type(&resolved_type);

                asm_function.push_comment("sum loop check bounds");
                self.check_loop_bounds(asm_function, in_statement, range);

                asm_function.push_comment("init return");
                asm_function.push_instructions(vec![
                    "mov rax, 0",
                    &format!("mov [rsp + {}], rax", 8 * range.len()),
                ]);

                self.init_indices(asm_function, range);

                asm_function.push_comment("loop body");
                asm_function.push_label(&format!(".jump{}", self.jump_counter));
                let continue_label = self.jump_counter;
                self.jump_counter += 1;
                self.generate_expression(asm_function, body, in_statement);
                let accumulator_address = 2 * 8 * range.len();

                if matches!(resolved_type, Type::Int) {
                    asm_function.push_instructions(vec![
                        "pop rax",
                        &format!("add [rsp + {}], rax", accumulator_address),
                    ]);
                } else if matches!(resolved_type, Type::Float) {
                    asm_function.push_instructions(vec![
                        "movsd xmm0, [rsp]",
                        "add rsp, 8",
                        &format!("addsd xmm0, [rsp + {}]", accumulator_address),
                        &format!("movsd [rsp + {}], xmm0", accumulator_address),
                    ]);
                }

                asm_function.remove_shadow();

                self.increment_loop_index(asm_function, range.len(), continue_label);
                asm_function.print_shadow_stack();

                asm_function.push_instruction(&format!("add rsp, {}", 8 * range.len()));
                range.iter().for_each(|_| {
                    asm_function.remove_shadow();
                });
                asm_function.push_instruction(&format!("add rsp, {}", 8 * range.len()));
                range.iter().for_each(|_| {
                    asm_function.remove_shadow();
                });

                asm_function.print_shadow_stack();
                assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
                asm_function.push_comment("sum loop end");
            }
            _ => unimplemented!("failure because expression: {}", expression.to_string()),
        }
    }

    fn tensor_optimizable_expressions(sum_body: &Expression<'_>) -> bool {
        match sum_body.node.as_ref() {
            ExpressionType::Binop {
                operator: _,
                left,
                right,
            } => {
                Self::tensor_optimizable_expressions(left)
                    && Self::tensor_optimizable_expressions(right)
            }
            ExpressionType::Int { .. }
            | ExpressionType::Float { .. }
            | ExpressionType::Variable { .. } => true,
            ExpressionType::ArrayIndex { array, indices } => {
                Self::tensor_optimizable_expressions(array)
                    && indices
                        .iter()
                        .all(|index| Self::tensor_optimizable_expressions(index))
            }
            _ => false,
        }
    }

    fn optimized_array_loop(
        &mut self,
        asm_function: &mut AsmFunction<'a>,
        array_range: &[(&'a str, Expression<'a>)],
        sum_range: &[(&'a str, Expression<'a>)],
        sum_body: &Expression<'a>,
        in_statement: bool,
    ) {
        asm_function.push_comment("tensor contraction optimization");

        let graph = Self::build_traversal_graph(array_range, sum_range, sum_body);
        asm_function.push_comment(&format!("tensor contraction graph: {:?}", graph));

        let topological_order = toposort(&graph, None).unwrap();

        asm_function.push_comment(&format!(
            "tensor contraction topological order: {:?}",
            topological_order
        ));

        asm_function.print_shadow_stack();
        asm_function.push_assert();

        asm_function.push_comment("allocate 8 bytes for pointer");
        asm_function.push_instruction("sub rsp, 8"); // Allocate data pointer for array
        asm_function.add_shadow_type(&Type::Int); // temp type that will be removed and re-contextualized as an array type

        // Check bounds for array and sum
        asm_function.push_comment("check bounds for array and sum");
        self.check_loop_bounds(asm_function, in_statement, array_range);
        self.check_loop_bounds(asm_function, in_statement, sum_range);

        // Calculate the size of the array and store it in rax
        asm_function.push_comment("calculate total array heap size");
        asm_function.push_instruction(&format!("mov rdi, {}", sum_body.resolved_type.usize()));
        (0..array_range.len()).for_each(|i| {
            asm_function
                .push_instruction(&format!("imul rdi, [rsp + {}]", (i + sum_range.len()) * 8)); // add sum_range length to skip past sum bounds
            self.assert(asm_function, "jno", "overflow computing array size");
        });

        asm_function.pad_shadow();
        asm_function.push_instruction("call _jpl_alloc");
        asm_function.unpad_shadow();
        asm_function.push_instruction(&format!(
            "mov [rsp + {}], rax",
            (array_range.len() + sum_range.len()) * 8
        ));

        // Initialize loop variables in topological order
        asm_function.push_comment("init loop variables");
        self.init_indices(asm_function, array_range);
        self.init_indices(asm_function, sum_range);

        // Loop body
        asm_function.push_comment("loop body");
        asm_function.push_label(&format!(".jump{}", self.jump_counter));
        let continue_label = self.jump_counter;
        self.jump_counter += 1;
        self.generate_expression(asm_function, sum_body, in_statement);

        // Calculate linear index
        self.calculate_linear_index(
            asm_function,
            Some(array_range),
            array_range.len(),
            sum_body.resolved_type.usize() + (sum_range.len() * 8), // Skip past sum loop vars
            sum_body.resolved_type.usize()
                + ((sum_range.len() + array_range.len() + sum_range.len()) * 8), // Skip past sum loop vars, array loop vars, and sum loop bounds
            sum_body.resolved_type.usize(),
        );

        // Copy data from stack to heap
        asm_function.push_comment("copying element from stack to array");
        match sum_body.resolved_type {
            Type::Int => {
                asm_function.push_instructions(vec![
                    "pop r10",
                    &format!("add [rax + {}], r10", 8 * (array_range.len() - 1)), // Already popped so minus 8 on the offset
                ]);
                asm_function.remove_shadow();
            }
            Type::Float => {
                asm_function.push_instructions(vec![
                    "movsd xmm0, [rsp]",
                    "add rsp, 8",
                    "addsd xmm0, [rax]",
                    "movsd [rax], xmm0",
                ]);
                asm_function.remove_shadow();
            }
            _ => {
                for i in (0..sum_body.resolved_type.usize()).step_by(8).rev() {
                    asm_function.push_instructions(vec![
                        &format!("mov r10, [rsp + {}]", i),
                        &format!("mov [rax + {}], r10", i),
                    ]);
                }
                asm_function.remove_shadow();
                asm_function
                    .push_instruction(&format!("add rsp, {}", sum_body.resolved_type.usize()));
            }
        }

        let mut stack_ordering = array_range
            .iter()
            .rev()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>();
        stack_ordering.extend(sum_range.iter().rev().map(|(name, _)| *name));
        stack_ordering.reverse(); // MARK: I know why this is necessary but I don't know why 4/5 of the tests pass without it

        asm_function.push_comment(&format!("stack ordering is {:?}", stack_ordering));

        // Iterate over topological ordering and figure out the offsets based on stack ordering
        let mut offsets = Vec::new();
        for name in topological_order.iter().rev() {
            // MARK: 4/5 tests pass without this double reverse
            asm_function.push_comment(&format!("checking if {} is in stack ordering", name));
            if let Some(index) = stack_ordering.iter().position(|&x| x == *name) {
                offsets.push(index);
            }
        }

        asm_function.push_comment(&format!("topological ordering is {:?}", topological_order));
        asm_function.push_comment(&format!("offsets are {:?}", offsets));

        for offset in offsets.iter().take(offsets.len() - 1) {
            asm_function.push_instructions(vec![
                &format!("add qword [rsp + {}], 1", offset * 8),
                &format!("mov rax, [rsp + {}]", offset * 8),
                &format!("cmp rax, [rsp + {}]", (offset + offsets.len()) * 8),
                &format!("jl .jump{}", continue_label),
                &format!("mov qword [rsp + {}], 0", offset * 8),
            ])
        }

        asm_function.push_instructions(vec![
            &format!("add qword [rsp + {}], 1", offsets.last().unwrap() * 8),
            &format!("mov rax, [rsp + {}]", offsets.last().unwrap() * 8),
            &format!(
                "cmp rax, [rsp + {}]",
                (offsets.last().unwrap() + offsets.len()) * 8
            ),
            &format!("jl .jump{}", continue_label),
        ]);

        // De-init loop variables
        asm_function.push_comment("de-init array and loop variables");
        (0..sum_range.len()).for_each(|_| asm_function.remove_shadow());
        array_range
            .iter()
            .for_each(|_| asm_function.remove_shadow());
        asm_function.push_instruction(&format!(
            "add rsp, {}",
            8 * (sum_range.len() + array_range.len()) // sum loops vars, array loop vars, sum loop bounds
        ));

        // De-init sum bounds
        asm_function.push_comment("de-init sum bounds");
        (0..sum_range.len()).for_each(|_| asm_function.remove_shadow());
        asm_function.push_instruction(&format!(
            "add rsp, {}",
            8 * sum_range.len() // sum loops vars, array loop vars, sum loop bounds
        ));

        // Re-contextualize the array type
        asm_function.push_comment("re-contextualizing array type");
        (0..=array_range.len()).for_each(|_| {
            // Remove loop bounds and data pointer types
            asm_function.remove_shadow();
        });
        asm_function.add_shadow_type(&Type::Array {
            element_type: Box::new(sum_body.resolved_type.clone()),
            rank: array_range.len(),
        });

        assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
    }

    fn build_traversal_graph(
        array_range: &[(&'a str, Expression<'a>)],
        sum_range: &[(&'a str, Expression<'a>)],
        sum_body: &Expression<'a>,
    ) -> DiGraphMap<&'a str, ()> {
        let mut graph: DiGraphMap<&'a str, ()> =
            DiGraphMap::with_capacity(array_range.len() + sum_range.len(), 0);

        // Create edges from array range - connecting array indices
        for (i, (name_i, _)) in array_range.iter().enumerate() {
            for (name_j, _) in array_range.iter().skip(i + 1) {
                graph.add_edge(name_i, name_j, ());
            }
        }

        // Create edges from sum range - connecting sum indices
        for (i, (name_i, _)) in sum_range.iter().enumerate() {
            if matches!(sum_body.resolved_type, Type::Float) {
                for (name_j, _) in sum_range.iter().skip(i + 1) {
                    graph.add_edge(name_i, name_j, ());
                }
            }
        }

        // Extract edges from array indices in the expression
        Self::extract_edges_from_expression(&mut graph, sum_body);

        graph
    }

    fn extract_edges_from_expression(
        graph: &mut DiGraphMap<&'a str, ()>,
        expression: &Expression<'a>,
    ) {
        match expression.node.as_ref() {
            ExpressionType::Binop {
                operator: _,
                left,
                right,
            } => {
                Self::extract_edges_from_expression(graph, left);
                Self::extract_edges_from_expression(graph, right);
            }
            ExpressionType::ArrayIndex { array: _, indices } => {
                for i in 0..indices.len() {
                    if let ExpressionType::Variable { name: name_i } = indices[i].node.as_ref() {
                        for j in (0..indices.len()).skip(i + 1) {
                            if let ExpressionType::Variable { name: name_j } =
                                indices[j].node.as_ref()
                            {
                                graph.add_edge(name_i, name_j, ());
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn optimized_array_index(
        &mut self,
        asm_function: &mut AsmFunction<'a>,
        indices: &Vec<Expression<'a>>,
        expression: &Expression<'a>,
        offset: isize,
        in_statement: bool,
    ) {
        asm_function.push_assert();
        asm_function.push_comment("generating index expressions");
        for index_expr in indices.iter().rev() {
            self.generate_expression(asm_function, index_expr, in_statement);
        }

        let location = asm_function.stack_size - offset;

        asm_function.push_comment("generating bounds checks");
        asm_function.print_shadow_stack();
        for (index, _) in indices.iter().enumerate() {
            asm_function.push_instructions(vec![
                &format!("mov rax, [rsp + {}]", index as isize * 8),
                "cmp rax, 0",
            ]);
            self.assert(asm_function, "jge", "negative array index");
            asm_function.push_instruction(&format!(
                "cmp rax, [rsp + {}]",
                location + (index as isize) * 8
            ));
            self.assert(asm_function, "jl", "index too large");
        }

        asm_function.push_instruction(&format!("mov rax, [rsp + {}]", 0));

        for i in 1..indices.len() {
            asm_function.push_instruction(&format!(
                "imul rax, [rsp + {}]",
                location + (i as isize * 8)
            ));
            asm_function.push_instruction(&format!("add rax, [rsp + {}]", i as isize * 8));
        }

        asm_function.push_instruction(&format!("imul rax, {}", expression.resolved_type.usize()));
        asm_function.push_instruction(&format!(
            "add rax, [rsp + {}]",
            location + indices.len() as isize * 8 // Skip loop body, all indices, and all bounds to get to the data pointer.
        ));

        for _ in indices.iter() {
            asm_function.remove_shadow();
        }
        asm_function.push_instruction(&format!("add rsp, {}", indices.len() as isize * 8));

        asm_function.push_instruction(&format!("sub rsp, {}", expression.resolved_type.usize()));
        asm_function.add_shadow_type(&expression.resolved_type);

        asm_function.push_comment("copying data from array to stack");
        asm_function.print_shadow_stack();
        for i in (0..(expression.resolved_type.usize())).step_by(8).rev() {
            asm_function.push_instructions(vec![
                &format!("mov r10, [rax + {}]", i),
                &format!("mov [rsp + {}], r10", i),
            ]);
        }

        asm_function.push_comment("array index end");
        asm_function.print_shadow_stack();
        assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
    }

    fn generate_int_op(&mut self, asm_function: &mut AsmFunction<'a>, operator: &str) {
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

                asm_function.push_comment("checking for division by zero");
                asm_function.print_shadow_stack();

                self.assert(
                    asm_function,
                    "jne",
                    if operator == "/" {
                        "divide by zero"
                    } else {
                        "mod by zero"
                    },
                );

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
    }

    fn generate_float_op(&mut self, asm_function: &mut AsmFunction<'a>, operator: &str) {
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
    }

    fn generate_bool_op(
        &mut self,
        asm_function: &mut AsmFunction<'a>,
        operator: &str,
        left_type: Type<'a>,
    ) {
        match left_type {
            Type::Int => self.generate_int_comparison(asm_function, operator),
            Type::Float => self.generate_float_comparison(asm_function, operator),
            Type::Bool => self.generate_bool_comparison(asm_function, operator),
            _ => unreachable!(),
        }
        asm_function.remove_shadow();
        asm_function.remove_shadow();
        asm_function.add_shadow_type(&Type::Bool);
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

    fn init_indices(
        &mut self,
        asm_function: &mut AsmFunction<'a>,
        range: &[(&str, Expression<'_>)],
    ) {
        asm_function.push_comment("init indices");
        //rev good
        range.iter().rev().for_each(|(var, _)| {
            asm_function.push_instructions(vec!["mov rax, 0", "push rax"]);
            asm_function.add_shadow_type(&Type::Int);
            self.offsets
                .insert(var.to_string(), (asm_function.stack_size, false));
            asm_function.push_comment(&format!(
                "saving loop variable {} with offset {}",
                var, asm_function.stack_size
            ));
        });
    }

    fn check_loop_bounds(
        &mut self,
        asm_function: &mut AsmFunction<'a>,
        in_statement: bool,
        range: &[(&str, Expression<'a>)],
    ) {
        asm_function.push_comment("make bounds");
        range.iter().rev().for_each(|(variable, expr)| {
            asm_function.push_comment(&format!(
                "generating bounds for {} with expr {}",
                variable, expr
            ));
            self.generate_expression(asm_function, expr, in_statement);
            asm_function.push_instructions(vec!["mov rax, [rsp]", "cmp rax, 0"]);
            self.assert(asm_function, "jg", "non-positive loop bound");
        });
    }

    fn calculate_linear_index(
        &mut self,
        asm_function: &mut AsmFunction<'a>,
        bounds: Option<&[(&'a str, Expression<'a>)]>,
        rank: usize,
        loop_vars_offset: usize,
        bounds_offset: usize,
        element_size: usize,
    ) {
        asm_function.push_comment("calculating linear index");
        if self.optimization_level > 0 {
            asm_function.push_instruction(&format!("mov rax, [rsp + {}]", loop_vars_offset));

            for i in 1..rank {
                if let Some(bounds) = bounds {
                    if let ExpressionType::Int { value } = bounds[i].1.node.as_ref() {
                        if Self::is_32_bit(value) || (*value as u64).is_power_of_two() {
                            asm_function.push_instruction(&format!("imul rax, {}", value));
                            asm_function.push_instruction(&format!(
                                "add rax, [rsp + {}]",
                                i * 8 + loop_vars_offset
                            ));
                            continue;
                        }
                    }
                }

                asm_function
                    .push_instruction(&format!("imul rax, [rsp + {}]", bounds_offset + (i * 8)));
                asm_function
                    .push_instruction(&format!("add rax, [rsp + {}]", i * 8 + loop_vars_offset));
            }
        } else {
            asm_function.push_instruction("mov rax, 0");
            for i in 0..rank {
                asm_function.push_instruction(&format!(
                    "imul rax, [rsp + {}]",
                    bounds_offset + (i * 8) // Skip loop body and indices and get to correct bound.
                ));
                asm_function
                    .push_instruction(&format!("add rax, [rsp + {}]", i * 8 + loop_vars_offset));
            }
        }

        asm_function.push_instruction(&format!("imul rax, {}", element_size));
        asm_function.push_instruction(&format!(
            "add rax, [rsp + {}]",
            bounds_offset + rank * 8 // Skip loop body, all indices, and all bounds to get to the data pointer.
        ));
    }

    fn increment_loop_index(
        &self,
        asm_function: &mut AsmFunction<'a>,
        rank: usize,
        continue_label: usize,
    ) {
        asm_function.push_comment("incrementing loop index");

        for i in (0..rank).skip(1).rev() {
            asm_function.push_instructions(vec![
                &format!("add qword [rsp + {}], 1", i * 8),
                &format!("mov rax, [rsp + {}]", i * 8),
                &format!("cmp rax, [rsp + {}]", 8 * (i + rank)),
                &format!("jl .jump{}", continue_label),
                &format!("mov qword [rsp + {}], 0", i * 8),
            ]);
        }

        asm_function.push_instructions(vec![
            "add qword [rsp], 1",
            "mov rax, [rsp]",
            &format!("cmp rax, [rsp + {}]", 8 * rank),
            &format!("jl .jump{}", continue_label),
        ]);
    }

    fn handle_variable(
        &mut self,
        asm_function: &mut AsmFunction<'a>,
        name: &str,
        expression: &Expression<'a>,
        in_statement: bool,
    ) {
        asm_function.push_assert();
        let (offset, from_main) = self.offsets.get(name).unwrap_or_else(|| {
            unimplemented!(
                "\n\texpression was this: {}\n\tcurrent offsets are: {:?}",
                expression.to_string(),
                self.offsets
            )
        });

        asm_function.push_comment(&format!("loading variable {} with offset {}", name, offset));

        asm_function.push_instruction(&format!("sub rsp, {}", expression.resolved_type.usize()));
        asm_function.add_shadow_type(&expression.resolved_type);
        let var_offset_reg = if *from_main && in_statement {
            "r12"
        } else {
            "rbp"
        };
        for i in (0..expression.resolved_type.usize()).step_by(8).rev() {
            asm_function.push_instructions(vec![
                &format!("mov r10, [{} - {} + {}]", var_offset_reg, offset, i),
                &format!("mov [rsp + {}], r10", i),
            ]);
        }
        assert!(asm_function.pop_assert(1), "\n\n{}", asm_function.body);
    }

    fn insert_string_constant(&mut self, asm_function: &mut AsmFunction, constant: String) {
        let const_name = self.insert_asm_constant(AssemblyValue::String(constant));
        asm_function.push_instruction(&format!("lea rdi, [rel {}]", const_name));
    }

    fn insert_int_constant(&mut self, asm_function: &mut AsmFunction, number: i64) {
        if self.optimization_level > 0 && Self::is_32_bit(&number) {
            asm_function.push_instruction(&format!("push qword {}", number));
        } else {
            let const_name = self.insert_asm_constant(AssemblyValue::Number(number.to_string()));
            asm_function
                .push_instructions(vec![&format!("mov rax, [rel {}]", const_name), "push rax"]);
        }
    }

    fn insert_float_constant(&mut self, asm_function: &mut AsmFunction, number: f64) {
        let const_name = self.insert_asm_constant(AssemblyValue::Number(format!("{:?}", number)));
        asm_function.push_instructions(vec![&format!("mov rax, [rel {}]", const_name), "push rax"]);
    }

    fn insert_asm_constant(&mut self, value: AssemblyValue) -> &str {
        let data_section_len = self.data_section.len();
        let entry = self.constants.entry(value.clone()).or_insert_with(|| {
            self.data_section.push(value.clone());
            format!("const{}", data_section_len)
        });
        entry
    }

    fn assert(&mut self, asm_function: &mut AsmFunction, cmp_mode: &str, message: &str) {
        let assert_label = format!(".jump{}", self.jump_counter);
        self.jump_counter += 1;
        asm_function.push_instruction(&format!("{} {}", cmp_mode, assert_label));
        asm_function.pad_shadow();
        self.insert_string_constant(asm_function, message.to_string());
        asm_function.push_instruction("call _fail_assertion");
        asm_function.unpad_shadow();
        asm_function.push_label(&assert_label);
    }

    fn is_32_bit(value: &i64) -> bool {
        num_traits::cast::ToPrimitive::to_i32(value).is_some()
    }

    fn get_show_format_string(typ: &Type) -> String {
        match typ {
            Type::Struct { name: _, elements } => {
                let mut format_string = "(TupleType".to_string();
                for (_, typ) in elements {
                    format_string.push(' ');
                    format_string.push_str(&Self::get_show_format_string(typ));
                }
                format_string.push(')');
                format_string
            }
            Type::Array { element_type, rank } => {
                format!(
                    "(ArrayType {} {})",
                    Self::get_show_format_string(element_type),
                    rank
                )
            }
            _ => typ.to_string(),
        }
    }
}

struct RegexReplacement {
    pattern: Regex,
    replacer: Box<dyn Fn(&regex::Captures) -> String>,
}

fn string_replacement_optimization(code: &str) -> String {
    // Define all our regex replacements
    let replacements = vec![
        // Replace imul with power of 2 by shift
        RegexReplacement {
            pattern: Regex::new(r"imul rax, (\d+)").unwrap(),
            replacer: Box::new(|caps| {
                let value = caps[1].parse::<u64>().unwrap();
                // Check if it's a power of 2
                if value.is_power_of_two() {
                    let shift = value.trailing_zeros();
                    format!("shl rax, {}", shift)
                } else {
                    // Return original if not a power of 2
                    caps[0].to_string()
                }
            }),
        },
        // Remove multiply by 1 instructions entirely.
        RegexReplacement {
            pattern: Regex::new(r"imul rax, 1").unwrap(),
            replacer: Box::new(|_| "".to_string()),
        },
    ];

    // Apply all replacements one by one
    let mut result = code.to_string();
    for replacement in replacements {
        result = replacement
            .pattern
            .replace_all(&result, |caps: &regex::Captures| {
                (replacement.replacer)(caps)
            })
            .to_string();
    }

    result
}

pub fn generate_assembly(commands: Vec<Command<'_>>, optimization_level: u8) -> String {
    let mut generator = AssemblyGenerator::new(optimization_level);
    let code = generator.generate_assembly(commands);
    if optimization_level > 0 {
        string_replacement_optimization(&code)
    } else {
        code
    }
}
