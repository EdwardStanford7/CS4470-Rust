use crate::ast::*;
use crate::utils::*;

pub fn generate_assembly<'a>(
    commands: Vec<Command<'a>>,
    environment: TypeEnvironment<'a>,
) -> String {
    let generator = AssemblyGenerator::new(commands, environment);
    generator.generate_assembly()
}

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

    pub fn get_type_name(&self, expr: &Expression, global_counter: u64) -> (String,u64,String,u64) {
        let mut globals = String::new();
        //TODO: check if type exists
        match expr.resolved_type {
            Type::Int => {
                let strval = "`(IntType)`";
                globals.push_str(&format!("const{}: db {}, 0\n", global_counter,strval));
                (strval.to_string(),global_counter,globals,global_counter + 1)
            }
            _ => {
                let strval = "`(UNKNOWN)`";
                globals.push_str(&format!("const{}: db {}, 0\n", global_counter,strval));
                (strval.to_string(),global_counter,globals,global_counter + 1)
            }
        }
    }

    /// Generate assembly for a single expression.
    pub fn generate_expression(&self, expr: &Expression, global_counter: u64) -> (String, String, u64) {
        let mut globals = String::new();
        let mut func = String::new();
        let mut new_global_counter = global_counter;

        match expr.node.as_ref() {
            ExpressionType::ArrayIndex { array, indices } => {
                func.push_str("// TODO: Generate assembly for array index expression\n");
                let (array_globals, array_func, array_counter) = self.generate_expression(array, new_global_counter);
                globals.push_str(&array_globals);
                func.push_str(&array_func);
                new_global_counter = array_counter;

                for index in indices {
                    let (idx_globals, idx_func, idx_counter) = self.generate_expression(index, new_global_counter);
                    globals.push_str(&idx_globals);
                    func.push_str(&idx_func);
                    new_global_counter = idx_counter;
                }
            }
            ExpressionType::ArrayLiteral { elements } => {
                func.push_str("// TODO: Generate assembly for array literal expression\n");
                for element in elements {
                    let (elem_globals, elem_func, elem_counter) = self.generate_expression(element, new_global_counter);
                    globals.push_str(&elem_globals);
                    func.push_str(&elem_func);
                    new_global_counter = elem_counter;
                }
            }
            ExpressionType::Binop { operator, left, right } => {
                func.push_str("// Generate assembly for binary operation\n");
                let (left_globals, left_func, left_counter) = self.generate_expression(left, new_global_counter);
                globals.push_str(&left_globals);
                func.push_str(&left_func);
                new_global_counter = left_counter;

                let (right_globals, right_func, right_counter) = self.generate_expression(right, new_global_counter);
                globals.push_str(&right_globals);
                func.push_str(&right_func);
                new_global_counter = right_counter;

                func.push_str(&format!("// Apply binary operator {}\n", operator));
            }
            ExpressionType::Call { function, arguments } => {
                func.push_str("// TODO: Generate assembly for function call\n");
                for arg in arguments {
                    let (arg_globals, arg_func, arg_counter) = self.generate_expression(arg, new_global_counter);
                    globals.push_str(&arg_globals);
                    func.push_str(&arg_func);
                    new_global_counter = arg_counter;
                }
                func.push_str(&format!("// Call function {}\n", function));
            }
            ExpressionType::Dot { struct_variable, field } => {
                func.push_str("// TODO: Generate assembly for struct field access (dot expression)\n");
                let (var_globals, var_func, var_counter) = self.generate_expression(struct_variable, new_global_counter);
                globals.push_str(&var_globals);
                func.push_str(&var_func);
                new_global_counter = var_counter;
                func.push_str(&format!("// Access field {}\n", field));
            }
            ExpressionType::False => {
                func.push_str("\tmov rax, 0\n");
                func.push_str("\tpush rax\n");
            }
            ExpressionType::Float { value } => {
                globals.push_str(&format!("const{}: dq {}\n", new_global_counter, value));
                func.push_str(&format!("\tmov rax, [rel const{}]\n", new_global_counter));
                func.push_str("\tpush rax\n");
                new_global_counter += 1;
            }
            ExpressionType::If { condition, then_branch, else_branch } => {
                func.push_str("// TODO: Generate assembly for if expression with branching\n");
                let (cond_globals, cond_func, cond_counter) = self.generate_expression(condition, new_global_counter);
                globals.push_str(&cond_globals);
                func.push_str(&cond_func);
                new_global_counter = cond_counter;

                func.push_str("// Then branch:\n");
                let (then_globals, then_func, then_counter) = self.generate_expression(then_branch, new_global_counter);
                globals.push_str(&then_globals);
                func.push_str(&then_func);
                new_global_counter = then_counter;

                func.push_str("// Else branch:\n");
                let (else_globals, else_func, else_counter) = self.generate_expression(else_branch, new_global_counter);
                globals.push_str(&else_globals);
                func.push_str(&else_func);
                new_global_counter = else_counter;
            }
            ExpressionType::Int { value } => {
                globals.push_str(&format!("const{}: dq {}\n", new_global_counter, value));
                func.push_str(&format!("\tmov rax, [rel const{}] ; {}\n", new_global_counter, value));
                func.push_str("\tpush rax\n");
                new_global_counter += 1;
            }
            ExpressionType::StructLiteral { name, fields } => {
                func.push_str("// TODO: Generate assembly for struct literal\n");
                func.push_str(&format!("// Begin struct literal: {}\n", name));
                for field in fields {
                    let (field_globals, field_func, field_counter) = self.generate_expression(field, new_global_counter);
                    globals.push_str(&field_globals);
                    func.push_str(&field_func);
                    new_global_counter = field_counter;
                }
                func.push_str("// End struct literal\n");
            }
            ExpressionType::SumLoop { range, body } => {
                func.push_str("// TODO: Generate assembly for sum loop\n");
                // For each range variable, generate code for the bound expression
                for (_, bound) in range {
                    let (bound_globals, bound_func, bound_counter) = self.generate_expression(bound, new_global_counter);
                    globals.push_str(&bound_globals);
                    func.push_str(&bound_func);
                    new_global_counter = bound_counter;
                }
                
                // Generate code for the body
                let (body_globals, body_func, body_counter) = self.generate_expression(body, new_global_counter);
                globals.push_str(&body_globals);
                func.push_str(&body_func);
                new_global_counter = body_counter;
            }
            ExpressionType::ArrayLoop { range, body } => {
                func.push_str("// TODO: Generate assembly for array loop\n");
                // For each range variable, generate code for the bound expression
                for (_, bound) in range {
                    let (bound_globals, bound_func, bound_counter) = self.generate_expression(bound, new_global_counter);
                    globals.push_str(&bound_globals);
                    func.push_str(&bound_func);
                    new_global_counter = bound_counter;
                }
                
                // Generate code for the body
                let (body_globals, body_func, body_counter) = self.generate_expression(body, new_global_counter);
                globals.push_str(&body_globals);
                func.push_str(&body_func);
                new_global_counter = body_counter;
            }
            ExpressionType::True => {
                func.push_str("// Generate assembly for boolean true\n");
                func.push_str("LOAD_BOOL 1\n");
            }
            ExpressionType::Unop { operator, expression } => {
                func.push_str("// Generate assembly for unary operation\n");
                let (expr_globals, expr_func, expr_counter) = self.generate_expression(expression, new_global_counter);
                globals.push_str(&expr_globals);
                func.push_str(&expr_func);
                new_global_counter = expr_counter;
                func.push_str(&format!("// Apply unary operator {}\n", operator));
            }
            ExpressionType::Variable { name } => {
                func.push_str("// Generate assembly for variable access\n");
                func.push_str(&format!("LOAD_VAR {}\n", name));
            }
            ExpressionType::Void => {
                func.push_str("// Generate assembly for void literal (no operation)\n");
            }
        }

        (globals, func, new_global_counter)
    }

    /// Generate assembly code for a statement.
    fn generate_statement(&self, stmt: &Statement, global_counter: u64) -> (String, String, u64) {
        let mut globals = String::new();
        let mut func = String::new();
        let mut new_global_counter = global_counter;

        match &stmt.node {
            StatementType::Let { variable, rvalue } => {
                func.push_str("// Generate assembly for let statement\n");
                let (rvalue_globals, rvalue_func, rvalue_counter) = self.generate_expression(rvalue, new_global_counter);
                globals.push_str(&rvalue_globals);
                func.push_str(&rvalue_func);
                new_global_counter = rvalue_counter;
                func.push_str(&format!("// Assign result to variable {}\n", variable.name));
            }
            StatementType::Assert { condition, message } => {
                func.push_str("// Generate assembly for assert statement\n");
                let (cond_globals, cond_func, cond_counter) = self.generate_expression(condition, new_global_counter);
                globals.push_str(&cond_globals);
                func.push_str(&cond_func);
                new_global_counter = cond_counter;
                func.push_str(&format!("// Assert with message: {}\n", message));
            }
            StatementType::Return { value } => {
                func.push_str("// Generate assembly for return statement\n");
                let (value_globals, value_func, value_counter) = self.generate_expression(value, new_global_counter);
                globals.push_str(&value_globals);
                func.push_str(&value_func);
                new_global_counter = value_counter;
                func.push_str("// Return the value\n");
            }
        }
        
        (globals, func, new_global_counter)
    }

    /// Generate assembly for a single command.
    fn generate_command(&self, command: &Command, global_counter: u64) -> (String, String, u64) {
        let mut globals = String::new();
        let mut func = String::new();
        let mut new_global_counter = global_counter;
        match command.node.as_ref() {
            CommandType::Assert { message, condition } => {
                let (cond_globals, cond_func, cond_counter) = self.generate_expression(condition, new_global_counter);
                globals.push_str(&cond_globals);
                func.push_str(&cond_func);
                func.push_str("\tpop rax\n");
                func.push_str("\tcmp rax, 0\n");
                func.push_str(&format!("\tje _fail_assertion ; {}\n", message));
                new_global_counter = cond_counter;
            }
            CommandType::Function { name, parameters, return_type, statements, has_return: _ } => {
                func.push_str(&format!("\n{}:\n", name));
                func.push_str("\tpush rbp\n");
                func.push_str("\tmov rbp, rsp\n");
                
                for (param, param_type) in parameters {
                    // Parameter handling would go here
                }
                
                for stmt in statements {
                    let (stmt_globals, stmt_func, stmt_counter) = self.generate_statement(stmt, new_global_counter);
                    globals.push_str(&stmt_globals);
                    func.push_str(&stmt_func);
                    new_global_counter = stmt_counter;
                }
                
                func.push_str("\tpop rbp\n");
                func.push_str("\tret\n");
            }
            CommandType::Let { variable, rvalue } => {
                let (rvalue_globals, rvalue_func, rvalue_counter) = self.generate_expression(rvalue, new_global_counter);
                globals.push_str(&rvalue_globals);
                func.push_str(&rvalue_func);
                
                // Store variable in local storage or memory
                func.push_str(&format!("\t; Store variable {}\n", variable.name));
                
                new_global_counter = rvalue_counter;
            }
            CommandType::Print { message } => {
                // Add the message string to globals
                globals.push_str(&format!("msg{}: db `{}`, 0\n", new_global_counter, message));
                
                // Print the message
                func.push_str(&format!("\tlea rdi, [rel msg{}]\n", new_global_counter));
                func.push_str("\tcall _print\n");
                
                new_global_counter += 1;
            }
            CommandType::Read { source, destination } => {
                // Add the source filename to globals
                globals.push_str(&format!("source{}: db `{}`, 0\n", new_global_counter, source));
                // Generate code to call _read_image
                func.push_str(&format!("\tlea rdi, [rel source{}]\n", new_global_counter));
                func.push_str("\t; Set up destination buffer\n");
                func.push_str("\tcall _read_image\n");
                new_global_counter += 1;
            }
            CommandType::Show { expression } => {
                let (expr_globals, expr_func, expr_counter) = self.generate_expression(expression, new_global_counter);
                new_global_counter = expr_counter;
                globals.push_str(&expr_globals);
                func.push_str(&expr_func);
                let (comment, count, type_globals, type_counter) = self.get_type_name(expression,new_global_counter);
                globals.push_str(&type_globals);
                new_global_counter = type_counter;
                if let ExpressionType::Int { .. } = expression.node.as_ref() {
                    func.push_str(&format!("\tlea rdi, [rel const{}] ; '(IntType)'\n",count));
                    func.push_str("\tlea rsi, [rsp]\n");
                    func.push_str("\tcall _show\n");
                    func.push_str("\tadd rsp, 8\n");
                } else {
                    func.push_str(&format!("\tTODO: implement type printing for {}\n",expression));
                }
            }
            CommandType::Struct { name, elements } => {
                // Just a type definition, no runtime code needed
                for (field, typ) in elements {
                    // Add struct information to assembly comments
                    func.push_str(&format!("\t; Struct {} field {} type {}\n", name, field, typ));
                }
            }
            CommandType::Time { command: inner_command } => {
                // Call _get_time before the command
                func.push_str("\tcall _get_time\n");
                func.push_str("\tpush rax\n");
                
                // Generate code for the inner command
                let (inner_globals, inner_func, inner_counter) = self.generate_command(inner_command, new_global_counter);
                globals.push_str(&inner_globals);
                func.push_str(&inner_func);
                
                // Call _print_time after the command
                func.push_str("\tpop rdi\n");
                func.push_str("\tcall _print_time\n");
                
                new_global_counter = inner_counter;
            }
            CommandType::Write { source, destination } => {
                // Generate code for the source expression
                let (source_globals, source_func, source_counter) = self.generate_expression(source, new_global_counter);
                globals.push_str(&source_globals);
                func.push_str(&source_func);
                // Add the destination filename to globals
                globals.push_str(&format!("dest{}: db `{}`, 0\n", new_global_counter, destination));
                // Generate code to call _write_image
                func.push_str("\t; Set up source buffer from stack\n");
                func.push_str(&format!("\tlea rdi, [rel dest{}]\n", new_global_counter));
                func.push_str("\tcall _write_image\n");
                new_global_counter = source_counter + 1;
            }
        }
        (globals, func, new_global_counter)
    }

    pub fn generate_assembly(&self) -> String {
        let mut imports = String::new();
        imports.push_str("\tglobal jpl_main\n\tglobal _jpl_main\n\textern _fail_assertion\n\textern _jpl_alloc\n\textern _get_time\n\textern _show\n\textern _print\n\textern _print_time\n\textern _read_image\n\textern _write_image\n\textern _fmod\n\textern _sqrt\n\textern _exp\n\textern _sin\n\textern _cos\n\textern _tan\n\textern _asin\n\textern _acos\n\textern _atan\n\textern _log\n\textern _pow\n\textern _atan2\n\textern _to_int\n\textern _to_float\n\n");
        let mut all_globals = String::new();
        all_globals.push_str("section .data\n");
        let mut main = String::new();
        main.push_str("\nsection .text\n");
        main.push_str("jpl_main:\n");
        main.push_str("_jpl_main:\n");
        main.push_str("\tpush rbp\n");
        main.push_str("\tmov rbp, rsp\n");
        main.push_str("\tpush r12\n");
        main.push_str("\tmov r12, rbp\n");
        let mut global_counter = 0;
        for command in &self.commands {
            let (globals, func, new_counter) = self.generate_command(command, global_counter);
            all_globals.push_str(&globals);
            main.push_str(&func);
            global_counter = new_counter;
        }
        main.push_str("\tpop r12\n");
        main.push_str("\tpop rbp\n");
        main.push_str("\tret\n");

        let mut assembly = String::new();
        assembly.push_str(&imports);
        assembly.push_str(&all_globals);
        assembly.push_str(&main);
        
        assembly
    }
}