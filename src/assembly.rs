use std::collections::HashMap;

use crate::ast::*;
use crate::utils::*;

pub fn generate_assembly<'a>(
    commands: Vec<Command<'a>>,
    environment: TypeEnvironment<'a>,
) -> String {
    let mut generator = AssemblyGenerator::new();
    generator.generate_assembly(commands, environment)
}

struct AssemblyGenerator<'a> {
    defined_types: HashMap<Type<'a>, (u64, String)>,
    defined_consts: HashMap<String, u64>,
    global_counter: u64,
    globals: String,
    code: String,
}

impl<'a> AssemblyGenerator<'a> {
    pub fn new() -> Self {
        Self {
            defined_types: HashMap::new(),
            defined_consts: HashMap::new(),
            global_counter: 0,
            globals: String::new(),
            code: String::new(),
        }
    }

    pub fn get_type_name(&mut self, expr: &Expression<'a>) -> (String, u64) {
        if let Some((num, name)) = self.defined_types.get(&expr.resolved_type) {
            return (name.clone(), *num);
        } else {
            let strval = format!("`{}`", expr.resolved_type.to_string().trim());
            self.globals.push_str(&format!("const{}: db {}, 0\n", self.global_counter, strval));
            
            let counter = self.global_counter;
            self.defined_types.insert(expr.resolved_type.clone(), (counter, strval.clone()));
            self.global_counter += 1;
            
            (strval, counter)
        }
    }

    pub fn get_const_name(&mut self, expr: &Expression) -> u64 {
        let mut genvar = String::new();
        match expr.node.as_ref() {
            ExpressionType::Float { value } => {
                if value.fract() == 0.0 {
                    genvar.push_str(&format!("dq {}.0\n", value));
                    if let Some(num) = self.defined_consts.get(&expr.to_string()) {
                        return *num;
                    }
                } else {
                    genvar.push_str(&format!("dq {}\n", value));
                    if let Some(num) = self.defined_consts.get(&expr.to_string()) {
                        return *num;
                    }
                }
            }
            ExpressionType::Int { value } => {
                genvar.push_str(&format!("dq {}\n", value));
                if let Some(num) = self.defined_consts.get(&expr.to_string()) {
                    return *num;
                }
            }
            ExpressionType::True {} => {
                genvar.push_str(&format!("dq {}\n", 1));
                if let Some(num) = self.defined_consts.get(&expr.to_string()) {
                    return *num;
                }
            }
            ExpressionType::False {} => {
                genvar.push_str(&format!("dq {}\n", 0));
                if let Some(num) = self.defined_consts.get(&expr.to_string()) {
                    return *num;
                }
            }
            _ => {}
        }
        
        let counter = self.global_counter;
        self.defined_consts.insert(genvar.clone(), counter);
        self.globals.push_str(&format!("const{}: ", counter));
        self.globals.push_str(&genvar);
        self.global_counter += 1;
        
        counter
    }

    /// Generate assembly for a single expression.
    pub fn generate_expression(&mut self, expr: &Expression) {
        match expr.node.as_ref() {
            ExpressionType::ArrayIndex { array, indices } => {
                self.code.push_str("// TODO: Generate assembly for array index expression\n");
                self.generate_expression(array);

                for index in indices {
                    self.generate_expression(index);
                }
            }
            ExpressionType::ArrayLiteral { elements } => {
                self.code.push_str("// TODO: Generate assembly for array literal expression\n");
                for element in elements {
                    self.generate_expression(element);
                }
            }
            ExpressionType::Binop { operator, left, right } => {
                self.generate_expression(left);
                self.generate_expression(right);
                
                match left.resolved_type {
                    Type::Int => {
                        self.code.push_str("\tpop rax\n");
                        self.code.push_str("\tpop r10\n");
                        match *operator {
                            "+" => { self.code.push_str("\tadd rax, r10\n"); }
                            "-" => { self.code.push_str("\tsub rax, r10\n"); }
                            "*" => { self.code.push_str("\tmul rax, r10\n"); }
                            "/" => { self.code.push_str("\tdiv rax, r10\n"); }
                            "%" => { self.code.push_str("\tmod rax, r10\n"); }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            ExpressionType::Call { function, arguments } => {
                self.code.push_str("// TODO: Generate assembly for function call\n");
                for arg in arguments {
                    self.generate_expression(arg);
                }
                self.code.push_str(&format!("// Call function {}\n", function));
            }
            ExpressionType::Dot { struct_variable, field } => {
                self.code.push_str("// TODO: Generate assembly for struct field access (dot expression)\n");
                self.generate_expression(struct_variable);
                self.code.push_str(&format!("// Access field {}\n", field));
            }
            ExpressionType::False | ExpressionType::Float { .. } | ExpressionType::Int { .. } | ExpressionType::True => {
                let num = self.get_const_name(expr);
                self.code.push_str(&format!("\tmov rax, [rel const{}]\n", num));
                self.code.push_str("\tpush rax\n");
            }
            ExpressionType::If { condition, then_branch, else_branch } => {
                self.code.push_str("// TODO: Generate assembly for if expression with branching\n");
                self.generate_expression(condition);
                
                self.code.push_str("// Then branch:\n");
                self.generate_expression(then_branch);
                
                self.code.push_str("// Else branch:\n");
                self.generate_expression(else_branch);
            }
            ExpressionType::StructLiteral { name, fields } => {
                self.code.push_str("// TODO: Generate assembly for struct literal\n");
                self.code.push_str(&format!("// Begin struct literal: {}\n", name));
                for field in fields {
                    self.generate_expression(field);
                }
                self.code.push_str("// End struct literal\n");
            }
            ExpressionType::SumLoop { range, body } => {
                self.code.push_str("// TODO: Generate assembly for sum loop\n");
                // For each range variable, generate code for the bound expression
                for (_, bound) in range {
                    self.generate_expression(bound);
                }
                
                // Generate code for the body
                self.generate_expression(body);
            }
            ExpressionType::ArrayLoop { range, body } => {
                self.code.push_str("// TODO: Generate assembly for array loop\n");
                // For each range variable, generate code for the bound expression
                for (_, bound) in range {
                    self.generate_expression(bound);
                }
                
                // Generate code for the body
                self.generate_expression(body);
            }
            ExpressionType::Unop { operator: _, expression } => {
                match expression.resolved_type {
                    Type::Bool => {
                        self.generate_expression(expression);
                        self.code.push_str("\tpop rax\n");
                        self.code.push_str("\txor rax, 1\n");
                        self.code.push_str("\tpush rax\n");
                    }
                    Type::Float => {
                        self.generate_expression(expression);
                        self.code.push_str("\tmovsd xmm1, [rsp]\n\n");
                        self.code.push_str("\tadd rsp, 8\n");
                        self.code.push_str("\tpxor xmm0, xmm0\n");
                        self.code.push_str("\tsubsd xmm0, xmm1\n");
                        self.code.push_str("\tsub rsp, 8\n");
                        self.code.push_str("\tmovsd [rsp], xmm0\n");
                    }
                    Type::Int => {
                        self.generate_expression(expression);
                        self.code.push_str("\tpop rax\n");
                        self.code.push_str("\tneg rax\n");
                        self.code.push_str("\tpush rax\n");
                    }
                    _ => { unreachable!() }
                }
            }
            ExpressionType::Variable { name } => {
                self.code.push_str("// Generate assembly for variable access\n");
                self.code.push_str(&format!("LOAD_VAR {}\n", name));
            }
            ExpressionType::Void => {
                self.code.push_str("// Generate assembly for void literal (no operation)\n");
            }
        }
    }

    /// Generate assembly code for a statement.
    fn generate_statement(&mut self, stmt: &Statement) {
        match &stmt.node {
            StatementType::Let { variable, rvalue } => {
                self.code.push_str("// Generate assembly for let statement\n");
                self.generate_expression(rvalue);
                self.code.push_str(&format!("// Assign result to variable {}\n", variable.name));
            }
            StatementType::Assert { condition, message } => {
                self.code.push_str("// Generate assembly for assert statement\n");
                self.generate_expression(condition);
                self.code.push_str(&format!("// Assert with message: {}\n", message));
            }
            StatementType::Return { value } => {
                self.code.push_str("// Generate assembly for return statement\n");
                self.generate_expression(value);
                self.code.push_str("// Return the value\n");
            }
        }
    }

    /// Generate assembly for a single command.
    fn generate_command(&mut self, command: &Command<'a>) {
        match command.node.as_ref() {
            CommandType::Assert { message, condition } => {
                self.generate_expression(condition);
                self.code.push_str("\tpop rax\n");
                self.code.push_str("\tcmp rax, 0\n");
                self.code.push_str(&format!("\tje _fail_assertion ; {}\n", message));
            }
            CommandType::Function { name, parameters: _, return_type: _, statements, has_return: _ } => {
                self.code.push_str(&format!("\n{}:\n", name));
                self.code.push_str("\tpush rbp\n");
                self.code.push_str("\tmov rbp, rsp\n");
                
                for stmt in statements {
                    self.generate_statement(stmt);
                }
                
                self.code.push_str("\tpop rbp\n");
                self.code.push_str("\tret\n");
            }
            CommandType::Let { variable, rvalue } => {
                self.generate_expression(rvalue);
                
                // Store variable in local storage or memory
                self.code.push_str(&format!("\t; Store variable {}\n", variable.name));
            }
            CommandType::Print { message } => {
                // Add the message string to globals
                self.globals.push_str(&format!("msg{}: db `{}`, 0\n", self.global_counter, message));
                
                // Print the message
                self.code.push_str(&format!("\tlea rdi, [rel msg{}]\n", self.global_counter));
                self.code.push_str("\tcall _print\n");
                
                self.global_counter += 1;
            }
            CommandType::Read { source, destination: _ } => {
                // Add the source filename to globals
                self.globals.push_str(&format!("source{}: db `{}`, 0\n", self.global_counter, source));
                
                // Generate code to call _read_image
                self.code.push_str(&format!("\tlea rdi, [rel source{}]\n", self.global_counter));
                self.code.push_str("\t; Set up destination buffer\n");
                self.code.push_str("\tcall _read_image\n");
                
                self.global_counter += 1;
            }
            CommandType::Show { expression } => {
                self.generate_expression(expression);
                let (comment, count) = self.get_type_name(expression);
                
                self.code.push_str(&format!("\tlea rdi, [rel const{}] ; {}\n", count, comment));
                self.code.push_str("\tlea rsi, [rsp]\n");
                self.code.push_str("\tcall _show\n");
                self.code.push_str("\tadd rsp, 8\n");
            }
            CommandType::Struct { name, elements } => {
                // Just a type definition, no runtime code needed
                for (field, typ) in elements {
                    // Add struct information to assembly comments
                    self.code.push_str(&format!("\t; Struct {} field {} type {}\n", name, field, typ));
                }
            }
            CommandType::Time { command: inner_command } => {
                // Call _get_time before the command
                self.code.push_str("\tcall _get_time\n");
                self.code.push_str("\tpush rax\n");
                
                // Generate code for the inner command
                self.generate_command(inner_command);
                
                // Call _print_time after the command
                self.code.push_str("\tpop rdi\n");
                self.code.push_str("\tcall _print_time\n");
            }
            CommandType::Write { source, destination } => {
                // Generate code for the source expression
                self.generate_expression(source);
                
                // Add the destination filename to globals
                self.globals.push_str(&format!("dest{}: db `{}`, 0\n", self.global_counter, destination));
                
                // Generate code to call _write_image
                self.code.push_str("\t; Set up source buffer from stack\n");
                self.code.push_str(&format!("\tlea rdi, [rel dest{}]\n", self.global_counter));
                self.code.push_str("\tcall _write_image\n");
                
                self.global_counter += 1;
            }
        }
    }

    pub fn generate_assembly(&mut self, commands: Vec<Command<'a>>, _environment: TypeEnvironment<'a>) -> String {
        // Generate imports section
        let mut imports = String::new();
        imports.push_str("\tglobal jpl_main\n\tglobal _jpl_main\n\textern _fail_assertion\n\textern _jpl_alloc\n\textern _get_time\n\textern _show\n\textern _print\n\textern _print_time\n\textern _read_image\n\textern _write_image\n\textern _fmod\n\textern _sqrt\n\textern _exp\n\textern _sin\n\textern _cos\n\textern _tan\n\textern _asin\n\textern _acos\n\textern _atan\n\textern _log\n\textern _pow\n\textern _atan2\n\textern _to_int\n\textern _to_float\n\n");
        
        // Initialize globals section
        self.globals = String::from("section .data\n");
        
        // Initialize main function
        self.code = String::from("\nsection .text\n");
        self.code.push_str("jpl_main:\n");
        self.code.push_str("_jpl_main:\n");
        self.code.push_str("\tpush rbp\n");
        self.code.push_str("\tmov rbp, rsp\n");
        self.code.push_str("\tpush r12\n");
        self.code.push_str("\tmov r12, rbp\n");
        
        // Process all commands
        for command in commands {
            self.generate_command(&command);
        }
        
        // Finalize main function
        self.code.push_str("\tpop r12\n");
        self.code.push_str("\tpop rbp\n");
        self.code.push_str("\tret\n");

        // Combine all sections
        let mut assembly = String::new();
        assembly.push_str(&imports);
        assembly.push_str(&self.globals);
        assembly.push_str(&self.code);
        
        assembly
    }
}