// Revised assembly.rs

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

    // Revised: Pass the incoming counter instead of resetting to zero.
    pub fn generate_expression(&self, expr: &Expression, global_counter: u64) -> (String, String, u64) {
        let mut globals = String::new();
        let mut func = String::new();
        let mut current_counter = global_counter; // use the passed-in counter

        match expr.node.as_ref() {
            ExpressionType::ArrayIndex { array, indices } => {
                func.push_str("// TODO: Generate assembly for array index expression\n");
                // (If needed, update current_counter via recursive calls.)
            }
            ExpressionType::ArrayLiteral { elements } => {
                func.push_str("// TODO: Generate assembly for array literal expression\n");
                for element in elements {
                    let (child_globals, child_func, new_counter) =
                        self.generate_expression(element, current_counter);
                    globals.push_str(&child_globals);
                    func.push_str(&child_func);
                    current_counter = new_counter;
                }
            }
            ExpressionType::Binop { operator, left, right } => {
                func.push_str("// Generate assembly for binary operation\n");
                let (child_globals, left_func, new_counter) =
                    self.generate_expression(left, current_counter);
                globals.push_str(&child_globals);
                func.push_str(&left_func);
                current_counter = new_counter;
                let (child_globals, right_func, new_counter) =
                    self.generate_expression(right, current_counter);
                globals.push_str(&child_globals);
                func.push_str(&right_func);
                current_counter = new_counter;
                func.push_str(&format!("// Apply binary operator {}\n", operator));
            }
            ExpressionType::Call { function, arguments } => {
                func.push_str("// TODO: Generate assembly for function call\n");
                for arg in arguments {
                    let (child_globals, child_func, new_counter) =
                        self.generate_expression(arg, current_counter);
                    globals.push_str(&child_globals);
                    func.push_str(&child_func);
                    current_counter = new_counter;
                }
                func.push_str(&format!("// Call function {}\n", function));
            }
            ExpressionType::Dot { struct_variable, field } => {
                func.push_str("// TODO: Generate assembly for struct field access (dot expression)\n");
                let (child_globals, child_func, new_counter) =
                    self.generate_expression(struct_variable, current_counter);
                globals.push_str(&child_globals);
                func.push_str(&child_func);
                current_counter = new_counter;
                func.push_str(&format!("// Access field {}\n", field));
            }
            ExpressionType::False => {
                func.push_str("// Generate assembly for boolean false\n");
                func.push_str("LOAD_BOOL 0\n");
            }
            ExpressionType::Float { value } => {
                func.push_str("// Generate assembly for float literal\n");
                func.push_str(&format!("LOAD_FLOAT {}\n", value));
            }
            ExpressionType::If { condition, then_branch, else_branch } => {
                func.push_str("// TODO: Generate assembly for if expression with branching\n");
                let (child_globals, cond_func, new_counter) =
                    self.generate_expression(condition, current_counter);
                globals.push_str(&child_globals);
                func.push_str(&cond_func);
                current_counter = new_counter;
                let (child_globals, then_func, new_counter) =
                    self.generate_expression(then_branch, current_counter);
                globals.push_str(&child_globals);
                func.push_str("// Then branch:\n");
                func.push_str(&then_func);
                current_counter = new_counter;
                let (child_globals, else_func, new_counter) =
                    self.generate_expression(else_branch, current_counter);
                globals.push_str(&child_globals);
                func.push_str("// Else branch:\n");
                func.push_str(&else_func);
                current_counter = new_counter;
            }
            ExpressionType::Int { value } => {
                func.push_str("// Generate assembly for int literal\n");
                globals.push_str(&format!("const{}: dq {}\n", current_counter, value));
                current_counter += 1;
            }
            ExpressionType::StructLiteral { name, fields } => {
                func.push_str("// TODO: Generate assembly for struct literal\n");
                func.push_str(&format!("// Begin struct literal: {}\n", name));
                for field in fields {
                    let (child_globals, child_func, new_counter) =
                        self.generate_expression(field, current_counter);
                    globals.push_str(&child_globals);
                    func.push_str(&child_func);
                    current_counter = new_counter;
                }
                func.push_str("// End struct literal\n");
            }
            ExpressionType::SumLoop { range, body } => {
                func.push_str("// TODO: Generate assembly for sum loop\n");
                // (Set up loop, update counter, etc.)
            }
            ExpressionType::True => {
                func.push_str("// Generate assembly for boolean true\n");
                func.push_str("LOAD_BOOL 1\n");
            }
            ExpressionType::Unop { operator, expression } => {
                func.push_str("// Generate assembly for unary operation\n");
                let (child_globals, child_func, new_counter) =
                    self.generate_expression(expression, current_counter);
                globals.push_str(&child_globals);
                func.push_str(&child_func);
                current_counter = new_counter;
                func.push_str(&format!("// Apply unary operator {}\n", operator));
            }
            ExpressionType::Variable { name } => {
                func.push_str("// Generate assembly for variable access\n");
                func.push_str(&format!("LOAD_VAR {}\n", name));
            }
            ExpressionType::Void => {
                func.push_str("// Generate assembly for void literal (no operation)\n");
            }
            _ => {
                func.push_str("// Unhandled expression type\n");
            }
        }

        (globals, func, current_counter)
    }

    // Revised generate_statement now also passes and returns the counter.
    fn generate_statement(&self, stmt: &Statement, global_counter: u64) -> (String, String, u64) {
        let mut globals = String::new();
        let mut func = String::new();
        let mut current_counter = global_counter;

        match &stmt.node {
            StatementType::Let { variable, rvalue } => {
                func.push_str("// Generate assembly for let statement\n");
                let (child_globals, child_func, new_counter) =
                    self.generate_expression(rvalue, current_counter);
                globals.push_str(&child_globals);
                func.push_str(&child_func);
                current_counter = new_counter;
                func.push_str(&format!("// Assign result to variable {}\n", variable.name));
            }
            StatementType::Assert { condition, message } => {
                func.push_str("// Generate assembly for assert statement\n");
                let (child_globals, child_func, new_counter) =
                    self.generate_expression(condition, current_counter);
                globals.push_str(&child_globals);
                func.push_str(&child_func);
                current_counter = new_counter;
                func.push_str(&format!("// Assert with message: {}\n", message));
            }
            StatementType::Return { value } => {
                func.push_str("// Generate assembly for return statement\n");
                let (child_globals, child_func, new_counter) =
                    self.generate_expression(value, current_counter);
                globals.push_str(&child_globals);
                func.push_str(&child_func);
                current_counter = new_counter;
                func.push_str("// Return the value\n");
            }
        }

        (globals, func, current_counter)
    }

    // Revised generate_command passes the counter along similarly.
    fn generate_command(&self, command: &Command, global_counter: u64) -> (String, String, u64) {
        let mut globals = String::new();
        let mut func = String::new();
        let mut current_counter = global_counter;

        match command.node.as_ref() {
            CommandType::Assert { message, condition } => {
                func.push_str("// Generate assembly for assert command\n");
                let (child_globals, child_func, new_counter) =
                    self.generate_expression(condition, current_counter);
                globals.push_str(&child_globals);
                func.push_str(&child_func);
                current_counter = new_counter;
                func.push_str(&format!("// Assert: {}\n", message));
            }
            CommandType::Function { name, parameters, return_type: _, statements, has_return: _ } => {
                func.push_str(&format!("// Begin function {}\n", name));
                for (param, param_type) in parameters {
                    func.push_str(&format!("// Parameter: {} of type {}\n", param.name, param_type));
                }
                for stmt in statements {
                    let (child_globals, child_func, new_counter) =
                        self.generate_statement(stmt, current_counter);
                    globals.push_str(&child_globals);
                    func.push_str(&child_func);
                    current_counter = new_counter;
                }
                func.push_str(&format!("// End function {}\n", name));
            }
            CommandType::Let { variable, rvalue } => {
                func.push_str("// Generate assembly for let command\n");
                let (child_globals, child_func, new_counter) =
                    self.generate_expression(rvalue, current_counter);
                globals.push_str(&child_globals);
                func.push_str(&child_func);
                current_counter = new_counter;
                func.push_str(&format!("// Let assignment for variable {}\n", variable.name));
            }
            CommandType::Print { message } => {
                func.push_str("// Generate assembly for print command\n");
                func.push_str(&format!("PRINT \"{}\"\n", message));
            }
            CommandType::Read { source, destination } => {
                func.push_str("// Generate assembly for read command\n");
                func.push_str(&format!("READ {} -> {}\n", source, destination.name));
            }
            CommandType::Show { expression } => {
                func.push_str("// Generate assembly for show command\n");
                let (child_globals, child_func, new_counter) =
                    self.generate_expression(expression, current_counter);
                globals.push_str(&child_globals);
                func.push_str(&child_func);
                current_counter = new_counter;
                func.push_str("// Show result\n");
            }
            CommandType::Struct { name, elements } => {
                func.push_str(&format!("// Generate assembly for struct definition: {}\n", name));
                for (field, typ) in elements {
                    func.push_str(&format!("// Field: {} with type {}\n", field, typ));
                }
            }
            CommandType::Time { command: inner_command } => {
                func.push_str("// Generate assembly for time command\n");
                let (child_globals, child_func, new_counter) =
                    self.generate_command(inner_command, current_counter);
                globals.push_str(&child_globals);
                func.push_str(&child_func);
                current_counter = new_counter;
                func.push_str("// Timing start\n");
                func.push_str(&format!("// Timing end\n"));
            }
            CommandType::Write { source, destination } => {
                func.push_str("// Generate assembly for write command\n");
                let (child_globals, child_func, new_counter) =
                    self.generate_expression(source, current_counter);
                globals.push_str(&child_globals);
                func.push_str(&child_func);
                current_counter = new_counter;
                func.push_str(&format!("// Write to destination: {}\n", destination));
            }
        }
        (globals, func, current_counter)
    }

    // The top-level function remains mostly the same.
    pub fn generate_assembly(&self) -> String {
        let mut imports = String::new();
        imports.push_str("global jpl_main\n");
        imports.push_str("global _jpl_main\n");
        imports.push_str("extern _fail_assertion\n");
        imports.push_str("extern _jpl_alloc\n");
        imports.push_str("extern _get_time\n");
        imports.push_str("extern _show\n");
        imports.push_str("extern _print\n");
        imports.push_str("extern _print_time\n");
        imports.push_str("extern _read_image\n");
        imports.push_str("extern _write_image\n");
        imports.push_str("extern _fmod\n");
        imports.push_str("extern _sqrt\n");
        imports.push_str("extern _exp\n");
        imports.push_str("extern _sin\n");
        imports.push_str("extern _cos\n");
        imports.push_str("extern _tan\n");
        imports.push_str("extern _asin\n");
        imports.push_str("extern _acos\n");
        imports.push_str("extern _atan\n");
        imports.push_str("extern _log\n");
        imports.push_str("extern _pow\n");
        imports.push_str("extern _atan2\n");
        imports.push_str("extern _to_int\n");
        imports.push_str("extern _to_float\n");
        imports.push_str("\n");

        let mut globals = String::new();
        let mut funcs = String::new();
        let mut main = String::new();
        let mut global_counter = 0;

        globals.push_str("section .data\n");

        for command in &self.commands {
            let (cmd_globals, cmd_func, new_counter) =
                self.generate_command(command, global_counter);
            globals.push_str(&cmd_globals);
            funcs.push_str(&cmd_func);
            global_counter = new_counter;
        }

        let mut assembly = String::new();
        assembly.push_str(&imports);
        assembly.push_str(&globals);
        assembly.push_str(&funcs);
        assembly.push_str(&main);
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