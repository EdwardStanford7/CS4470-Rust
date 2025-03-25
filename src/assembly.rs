use std::collections::HashMap;
use std::fmt::Write as _;

use crate::ast::*;
use crate::utils::*;

pub fn generate_assembly<'a>(
    commands: Vec<Command<'a>>,
    environment: TypeEnvironment<'a>,
) -> String {
    let mut generator = AssemblyGenerator::new();
    generator.generate_assembly(commands, environment)
}

/// Assembly generator that produces x86-64 assembly code from AST nodes
struct AssemblyGenerator<'a> {
    /// Maps type values to their constant ID and string representation
    type_cache: HashMap<Type<'a>, (u64, String)>,
    
    /// Maps constant expressions to their global IDs
    const_cache: HashMap<String, u64>,
    
    /// Maps variable names to their locations
    variable_map: HashMap<&'a str, VariableLocation>,
    
    /// Counter for generating unique global IDs
    global_counter: u64,
    
    /// Label counter for generating unique labels
    label_counter: u64,
    
    /// Data section of the assembly
    data_section: String,
    
    /// Text section of the assembly
    text_section: String,
    
    /// Current stack offset for local variables
    stack_offset: i32,
}

/// Represents the location of a variable
enum VariableLocation {
    Stack(i32),         // Offset from rbp
    Register(String),   // Register name
    Global(String),     // Global label
}

impl<'a> AssemblyGenerator<'a> {
    pub fn new() -> Self {
        Self {
            type_cache: HashMap::new(),
            const_cache: HashMap::new(),
            variable_map: HashMap::new(),
            global_counter: 0,
            label_counter: 0,
            data_section: String::from("section .data\n"),
            text_section: String::from("\nsection .text\n"),
            stack_offset: 0,
        }
    }

    /// Generate a unique label for jumps and branches
    fn next_label(&mut self, prefix: &str) -> String {
        let label = format!("{}_{}", prefix, self.label_counter);
        self.label_counter += 1;
        label
    }

    /// Get or create a global constant for a type
    fn get_type_name(&mut self, typ: &Type<'a>) -> (String, u64) {
        if let Some((num, name)) = self.type_cache.get(typ) {
            return (name.clone(), *num);
        }
        
        let strval = format!("`{}`", typ.to_string().trim());
        let counter = self.global_counter;
        
        self.data_section.push_str(&format!("const{}: db {}, 0\n", counter, strval));
        self.type_cache.insert(typ.clone(), (counter, strval.clone()));
        self.global_counter += 1;
        
        (strval, counter)
    }

    /// Get or create a global constant for a literal value
    fn get_const_id(&mut self, expr: &Expression) -> u64 {
        let key = expr.to_string();
        
        if let Some(id) = self.const_cache.get(&key) {
            return *id;
        }
        
        let mut value_str = String::new();
        match expr.node.as_ref() {
            ExpressionType::Float { value } => {
                if value.fract() == 0.0 {
                    value_str.push_str(&format!("dq {}.0\n", value));
                } else {
                    value_str.push_str(&format!("dq {}\n", value));
                }
            }
            ExpressionType::Int { value } => {
                value_str.push_str(&format!("dq {}\n", value));
            }
            ExpressionType::True => {
                value_str.push_str("dq 1\n");
            }
            ExpressionType::False => {
                value_str.push_str("dq 0\n");
            }
            _ => panic!("Cannot create constant for expression type"),
        }
        
        let counter = self.global_counter;
        self.const_cache.insert(key, counter);
        self.data_section.push_str(&format!("const{}: {}", counter, value_str));
        self.global_counter += 1;
        
        counter
    }

    /// Add a string to the data section
    fn add_string(&mut self, s: &str) -> u64 {
        let counter = self.global_counter;
        self.data_section.push_str(&format!("str{}: db `{}`, 0\n", counter, s));
        self.global_counter += 1;
        counter
    }

    /// Push a value onto the stack
    fn push_value(&mut self, register: &str) {
        self.text_section.push_str(&format!("\tpush {}\n", register));
        self.stack_offset += 8;
    }

    /// Pop a value from the stack into a register
    fn pop_value(&mut self, register: &str) {
        self.text_section.push_str(&format!("\tpop {}\n", register));
        self.stack_offset -= 8;
    }

    /// Load a constant to a register
    fn load_constant(&mut self, expr: &Expression, register: &str) {
        let const_id = self.get_const_id(expr);
        self.text_section.push_str(&format!("\tmov {}, [rel const{}]\n", register, const_id));
    }

    /// Generate assembly for a literal expression (int, float, bool)
    fn generate_literal(&mut self, expr: &Expression) {
        self.load_constant(expr, "rax");
        self.push_value("rax");
    }

    /// Generate assembly for unary operations
    fn generate_unop(&mut self, operator: &str, expr: &Expression) {
        // Generate code for the operand
        self.generate_expression(expr);
        
        match expr.resolved_type {
            Type::Bool => {
                self.pop_value("rax");
                self.text_section.push_str("\txor rax, 1\n"); // Boolean NOT
                self.push_value("rax");
            }
            Type::Float => {
                self.text_section.push_str("\tmovsd xmm1, [rsp]\n");
                self.text_section.push_str("\tadd rsp, 8\n");
                self.stack_offset -= 8;
                self.text_section.push_str("\tpxor xmm0, xmm0\n");
                self.text_section.push_str("\tsubsd xmm0, xmm1\n"); // Floating-point negation
                self.text_section.push_str("\tsub rsp, 8\n");
                self.stack_offset += 8;
                self.text_section.push_str("\tmovsd [rsp], xmm0\n");
            }
            Type::Int => {
                self.pop_value("rax");
                self.text_section.push_str("\tneg rax\n"); // Integer negation
                self.push_value("rax");
            }
            _ => panic!("Unsupported type for unary operation: {}", expr.resolved_type),
        }
    }

    /// Generate assembly for binary operations
    fn generate_binop(&mut self, operator: &str, left: &Expression, right: &Expression) {
        // Generate code for the right operand first (stack order)
        self.generate_expression(right);
        self.generate_expression(left);
        
        match left.resolved_type {
            Type::Int => {
                self.pop_value("rax"); // Right operand
                self.pop_value("r10"); // Left operand
                match operator {
                    "+" => self.text_section.push_str("\tadd rax, r10\n"),
                    "-" => self.text_section.push_str("\tsub rax, r10\n"),
                    "*" => self.text_section.push_str("\timul rax, r10\n"),
                    "/" => {
                        self.text_section.push_str("\tmov rdx, 0\n");
                        self.text_section.push_str("\tidiv r10\n");
                        self.text_section.push_str("\tmov r10, rax\n");
                    },
                    "%" => {
                        self.text_section.push_str("\tmov rdx, 0\n");
                        self.text_section.push_str("\tidiv r10\n");
                        self.text_section.push_str("\tmov r10, rdx\n");
                    },
                    "==" => {
                        self.text_section.push_str("\tcmp r10, rax\n");
                        self.text_section.push_str("\tsete r10b\n");
                        self.text_section.push_str("\tmovzx r10, r10b\n");
                    },
                    "!=" => {
                        self.text_section.push_str("\tcmp r10, rax\n");
                        self.text_section.push_str("\tsetne r10b\n");
                        self.text_section.push_str("\tmovzx r10, r10b\n");
                    },
                    "<" => {
                        self.text_section.push_str("\tcmp r10, rax\n");
                        self.text_section.push_str("\tsetl r10b\n");
                        self.text_section.push_str("\tmovzx r10, r10b\n");
                    },
                    "<=" => {
                        self.text_section.push_str("\tcmp r10, rax\n");
                        self.text_section.push_str("\tsetle r10b\n");
                        self.text_section.push_str("\tmovzx r10, r10b\n");
                    },
                    ">" => {
                        self.text_section.push_str("\tcmp r10, rax\n");
                        self.text_section.push_str("\tsetg r10b\n");
                        self.text_section.push_str("\tmovzx r10, r10b\n");
                    },
                    ">=" => {
                        self.text_section.push_str("\tcmp r10, rax\n");
                        self.text_section.push_str("\tsetge r10b\n");
                        self.text_section.push_str("\tmovzx r10, r10b\n");
                    },
                    _ => panic!("Unsupported binary operator for integers: {}", operator),
                }
                self.push_value("rax");
            },
            Type::Float => {
                // Implement floating-point operations
                self.text_section.push_str("\tmovsd xmm0, [rsp]\n");     // Left operand
                self.text_section.push_str("\tadd rsp, 8\n");
                self.stack_offset -= 8;
                self.text_section.push_str("\tmovsd xmm1, [rsp]\n");     // Right operand
                self.text_section.push_str("\tadd rsp, 8\n");
                self.stack_offset -= 8;
                
                match operator {
                    "+" => self.text_section.push_str("\taddsd xmm0, xmm1\n"),
                    "-" => self.text_section.push_str("\tsubsd xmm0, xmm1\n"),
                    "*" => self.text_section.push_str("\tmulsd xmm0, xmm1\n"),
                    "/" => self.text_section.push_str("\tdivsd xmm0, xmm1\n"),
                    _ => panic!("Unsupported binary operator for floats: {}", operator),
                }
                
                self.text_section.push_str("\tsub rsp, 8\n");
                self.stack_offset += 8;
                self.text_section.push_str("\tmovsd [rsp], xmm0\n");
            },
            Type::Bool => {
                self.pop_value("r10"); // Left operand
                self.pop_value("rax"); // Right operand
                
                match operator {
                    "&&" => {
                        self.text_section.push_str("\tand r10, rax\n");
                    },
                    "||" => {
                        self.text_section.push_str("\tor r10, rax\n");
                    },
                    "==" => {
                        self.text_section.push_str("\tcmp r10, rax\n");
                        self.text_section.push_str("\tsete r10b\n");
                        self.text_section.push_str("\tmovzx r10, r10b\n");
                    },
                    "!=" => {
                        self.text_section.push_str("\tcmp r10, rax\n");
                        self.text_section.push_str("\tsetne r10b\n");
                        self.text_section.push_str("\tmovzx r10, r10b\n");
                    },
                    _ => panic!("Unsupported binary operator for booleans: {}", operator),
                }
                
                self.push_value("r10");
            },
            _ => panic!("Unsupported type for binary operation: {}", left.resolved_type),
        }
    }

    /// Generate assembly for if expressions
    fn generate_if_expression(&mut self, condition: &Expression, then_branch: &Expression, else_branch: &Expression) {
        let else_label = self.next_label("else");
        let end_label = self.next_label("endif");
        
        // Generate condition code
        self.generate_expression(condition);
        self.pop_value("rax");
        self.text_section.push_str("\ttest rax, rax\n");
        self.text_section.push_str(&format!("\tjz {}\n", else_label));
        
        // Then branch
        self.generate_expression(then_branch);
        self.text_section.push_str(&format!("\tjmp {}\n", end_label));
        
        // Else branch
        self.text_section.push_str(&format!("{}:\n", else_label));
        self.generate_expression(else_branch);
        
        // End
        self.text_section.push_str(&format!("{}:\n", end_label));
    }

    /// Generate assembly for a variable reference
    fn generate_variable(&mut self, name: &str) {
        if let Some(location) = self.variable_map.get(name) {
            match location {
                VariableLocation::Stack(offset) => {
                    self.text_section.push_str(&format!("\tmov rax, [rbp{}]\n", if *offset >= 0 { format!("+{}", offset) } else { format!("{}", offset) }));
                    self.push_value("rax");
                },
                VariableLocation::Register(reg) => {
                    self.text_section.push_str(&format!("\tmov rax, {}\n", reg));
                    self.push_value("rax");
                },
                VariableLocation::Global(label) => {
                    self.text_section.push_str(&format!("\tmov rax, [rel {}]\n", label));
                    self.push_value("rax");
                },
            }
        } else {
            // Default fallback behavior for undefined variables (should be caught by typechecker)
            self.text_section.push_str(&format!("\t; LOAD_VAR {} (unresolved)\n", name));
            self.text_section.push_str("\tmov rax, 0\n");
            self.push_value("rax");
        }
    }

    /// Generate assembly for a function call
    fn generate_function_call(&mut self, function: &str, arguments: &[Expression]) {
        // Preserve registers according to calling convention
        self.text_section.push_str("\t; Save registers for function call\n");
        
        // Generate code for arguments (in reverse order for the stack)
        for arg in arguments.iter().rev() {
            self.generate_expression(arg);
        }
        
        // Setup argument registers according to x86-64 calling convention
        let arg_regs = ["rdi", "rsi", "rdx", "rcx", "r8", "r9"];
        let stack_args = arguments.len().saturating_sub(arg_regs.len());
        
        // Pop arguments into registers (first 6 args)
        for (i, reg) in arg_regs.iter().enumerate().take(arguments.len().min(arg_regs.len())) {
            self.pop_value(reg);
        }
        
        // Call the function
        self.text_section.push_str(&format!("\tcall {}\n", function));
        
        // Clean up any remaining stack arguments
        if stack_args > 0 {
            self.text_section.push_str(&format!("\tadd rsp, {}\n", stack_args * 8));
            self.stack_offset -= (stack_args * 8) as i32;
        }
        
        // Return value is in rax, push it onto stack
        self.push_value("rax");
    }

    /// Generate assembly for array access
    fn generate_array_index(&mut self, array: &Expression, indices: &[Expression]) {
        // TODO: Implement array indexing
        self.text_section.push_str("// TODO: Implement array indexing\n");
        self.generate_expression(array);
        
        for index in indices {
            self.generate_expression(index);
            self.pop_value("rcx"); // Index
            self.pop_value("rax"); // Array pointer
            
            // Calculate element address based on type size
            // This is a simplified implementation - real code would need to handle
            // multi-dimensional arrays and various element types properly
            self.text_section.push_str("\timul rcx, 8\n"); // Assuming 8-byte elements
            self.text_section.push_str("\tadd rax, rcx\n");
            self.text_section.push_str("\tmov rax, [rax]\n"); // Load the element
            self.push_value("rax");
        }
    }

    /// Generate assembly for a loop (array or sum)
    fn generate_loop(&mut self, range: &[(&str, Expression)], body: &Expression, is_sum: bool) {
        // TODO: Implement loops
        self.text_section.push_str(&format!("// TODO: Implement {} loop\n", if is_sum { "sum" } else { "array" }));
        
        // For now, just evaluate the body once as a placeholder
        self.generate_expression(body);
    }
    
    /// Generate assembly code for an expression
    pub fn generate_expression(&mut self, expr: &Expression) {
        // Add a comment for debugging
        self.text_section.push_str(&format!("\t; Expression: {}\n", expr));
        
        match expr.node.as_ref() {
            ExpressionType::Int { .. } | ExpressionType::Float { .. } | 
            ExpressionType::True | ExpressionType::False => {
                self.generate_literal(expr);
            }
            ExpressionType::Variable { name } => {
                self.generate_variable(name);
            }
            ExpressionType::Unop { operator, expression } => {
                self.generate_unop(operator, expression);
            }
            ExpressionType::Binop { operator, left, right } => {
                self.generate_binop(operator, left, right);
            }
            ExpressionType::If { condition, then_branch, else_branch } => {
                self.generate_if_expression(condition, then_branch, else_branch);
            }
            ExpressionType::Call { function, arguments } => {
                self.generate_function_call(function, arguments);
            }
            ExpressionType::ArrayIndex { array, indices } => {
                self.generate_array_index(array, indices);
            }
            ExpressionType::ArrayLoop { range, body } => {
                self.generate_loop(range, body, false);
            }
            ExpressionType::SumLoop { range, body } => {
                self.generate_loop(range, body, true);
            }
            ExpressionType::Void => {
                // No operation for void
                self.text_section.push_str("\t; void expression (no operation)\n");
            }
            ExpressionType::ArrayLiteral { elements } => {
                self.text_section.push_str("// TODO: Implement array literals\n");
                for element in elements {
                    self.generate_expression(element);
                }
            }
            ExpressionType::Dot { struct_variable, field } => {
                self.text_section.push_str(&format!("// TODO: Implement struct field access: {}\n", field));
                self.generate_expression(struct_variable);
            }
            ExpressionType::StructLiteral { name, fields } => {
                self.text_section.push_str(&format!("// TODO: Implement struct literal: {}\n", name));
                for field in fields {
                    self.generate_expression(field);
                }
            }
        }
    }
    
    /// Generate assembly code for a statement
    fn generate_statement(&mut self, stmt: &Statement) {
        match &stmt.node {
            StatementType::Let { variable, rvalue } => {
                // Generate code for the right-hand side
                self.generate_expression(rvalue);
                
                // Allocate space for the variable if needed
                // let var_offset = self.stack_offset - 8; // Top of stack after expression evaluation
                // self.variable_map.insert(variable.name, VariableLocation::Stack(var_offset));
                
                self.text_section.push_str(&format!("\t; Let statement: {}\n", variable.name));
                // The value is already on the stack, so we don't need to do anything else
            }
            StatementType::Assert { condition, message } => {
                self.text_section.push_str(&format!("\t; Assert: {}\n", message));
                
                // Generate code for the condition
                self.generate_expression(condition);
                self.pop_value("rax");
                
                // Check the condition
                let skip_label = self.next_label("assert_pass");
                self.text_section.push_str("\ttest rax, rax\n");
                self.text_section.push_str(&format!("\tjnz {}\n", skip_label));
                
                // Call the assertion failure function
                let msg_id = self.add_string(message);
                self.text_section.push_str(&format!("\tlea rdi, [rel str{}]\n", msg_id));
                self.text_section.push_str("\tcall _fail_assertion\n");
                
                // Skip label
                self.text_section.push_str(&format!("{}:\n", skip_label));
            }
            StatementType::Return { value } => {
                self.text_section.push_str("\t; Return statement\n");
                
                // Generate code for the return value
                self.generate_expression(value);
                self.pop_value("rax"); // Return value goes in rax
                
                // Clean up and return
                self.text_section.push_str("\tmov rsp, rbp\n");
                self.text_section.push_str("\tpop rbp\n");
                self.text_section.push_str("\tret\n");
            }
        }
    }
    
    /// Generate assembly for a single command
    fn generate_command(&mut self, command: &Command<'a>) {
        
        match command.node.as_ref() {
            CommandType::Assert { condition, message } => {
                self.text_section.push_str(&format!("\t; Assert: {}\n", message));
                
                // Generate code for the condition
                self.generate_expression(condition);
                self.pop_value("rax");
                
                // Check the condition
                let skip_label = self.next_label("assert_pass");
                self.text_section.push_str("\ttest rax, rax\n");
                self.text_section.push_str(&format!("\tjnz {}\n", skip_label));
                
                // Call the assertion failure function
                let msg_id = self.add_string(message);
                self.text_section.push_str(&format!("\tlea rdi, [rel str{}]\n", msg_id));
                self.text_section.push_str("\tcall _fail_assertion\n");
                
                // Skip label
                self.text_section.push_str(&format!("{}:\n", skip_label));
            }
            CommandType::Function { name, parameters, return_type: _, statements, has_return: _ } => {
                // Function prologue
                self.text_section.push_str(&format!("\n{}:\n", name));
                self.text_section.push_str("\tpush rbp\n");
                self.text_section.push_str("\tmov rbp, rsp\n");
                
                // Reset stack offset for new function
                self.stack_offset = 0;
                
                // Save callee-saved registers
                self.text_section.push_str("\tpush r12\n");
                self.text_section.push_str("\tpush r13\n");
                self.text_section.push_str("\tpush r14\n");
                self.text_section.push_str("\tpush r15\n");
                self.stack_offset += 32; // 4 registers * 8 bytes
                
                // Allocate space for parameters
                let arg_regs = ["rdi", "rsi", "rdx", "rcx", "r8", "r9"];
                for (i, (param, _)) in parameters.iter().enumerate() {
                    if i < arg_regs.len() {
                        // Parameter is in a register
                        self.text_section.push_str(&format!("\tpush {}\n", arg_regs[i]));
                        self.stack_offset += 8;
                        self.variable_map.insert(param.name, VariableLocation::Stack(-self.stack_offset));
                    } else {
                        // Parameter is on the stack
                        // In x86-64, parameters beyond the 6th are already on the stack
                        let offset = (i - arg_regs.len() + 2) * 8; // +2 to account for return address and saved rbp
                        self.variable_map.insert(param.name, VariableLocation::Stack(offset as i32));
                    }
                }
                
                // Generate code for function body
                for stmt in statements {
                    self.generate_statement(stmt);
                }
                
                // Function epilogue (if no explicit return)
                self.text_section.push_str("\n\t; Function epilogue\n");
                self.text_section.push_str("\tpop r15\n");
                self.text_section.push_str("\tpop r14\n");
                self.text_section.push_str("\tpop r13\n");
                self.text_section.push_str("\tpop r12\n");
                self.text_section.push_str("\tmov rsp, rbp\n");
                self.text_section.push_str("\tpop rbp\n");
                self.text_section.push_str("\tret\n");
                
                // Clear variables defined in this function
                self.variable_map.clear();
            }
            CommandType::Let { variable, rvalue } => {
                // Generate code for the right-hand side
                self.generate_expression(rvalue);
                
                // Store the result in the variable
                self.text_section.push_str(&format!("\t; Assign to {}\n", variable.name));
                
                // For now, we keep the value on the stack and record its location
                let var_offset = -self.stack_offset; // Negative offset from rbp
                self.variable_map.insert(variable.name, VariableLocation::Stack(var_offset));
            }
            CommandType::Print { message } => {
                // Add the message string to globals
                let msg_id = self.add_string(message);
                
                // Print the message
                self.text_section.push_str(&format!("\tlea rdi, [rel str{}]\n", msg_id));
                self.text_section.push_str("\tcall _print\n");
            }
            CommandType::Read { source, destination } => {
                // Add the source filename to globals
                let source_id = self.add_string(source);
                
                // Call _read_image with the filename
                self.text_section.push_str(&format!("\tlea rdi, [rel str{}]\n", source_id));
                
                // TODO: Setup destination array
                self.text_section.push_str("\t; TODO: Setup destination array\n");
                
                self.text_section.push_str("\tcall _read_image\n");
                
                // Store the result in the destination variable
                self.text_section.push_str(&format!("\t; Store result in {}\n", destination.name));
                // TODO: Implement array storage
            }
            CommandType::Show { expression } => {
                // Generate code for the expression
                self.generate_expression(expression);
                
                // Get type information
                let (comment, type_id) = self.get_type_name(&expression.resolved_type);
                
                // Call _show with the expression result and type info
                self.text_section.push_str(&format!("\tlea rdi, [rel const{}] ; {}\n", type_id, comment));
                self.text_section.push_str("\tlea rsi, [rsp]\n");
                self.text_section.push_str("\tcall _show\n");
                
                // Clean up the stack
                self.text_section.push_str("\tadd rsp, 8\n");
                self.stack_offset -= 8;
            }
            CommandType::Struct { name, elements } => {
                // Struct definitions don't generate runtime code, just add comments
                self.text_section.push_str(&format!("\t; Struct definition: {}\n", name));
                
                for (field, typ) in elements {
                    self.text_section.push_str(&format!("\t; Field: {} Type: {}\n", field, typ));
                }
            }
            CommandType::Time { command } => {
                // Call _get_time to get start time
                self.text_section.push_str("\tcall _get_time\n");
                self.text_section.push_str("\tpush rax\n");
                self.stack_offset += 8;
                
                // Generate code for the inner command
                self.generate_command(command);
                
                // Call _print_time with start time
                self.text_section.push_str("\tpop rdi\n");
                self.stack_offset -= 8;
                self.text_section.push_str("\tcall _print_time\n");
            }
            CommandType::Write { source, destination } => {
                // Generate code for the source expression
                self.generate_expression(source);
                
                // Add the destination filename to globals
                let dest_id = self.add_string(destination);
                
                // Call _write_image with the destination filename and source data
                self.text_section.push_str(&format!("\tlea rdi, [rel str{}]\n", dest_id));
                self.text_section.push_str("\tmov rsi, rsp\n"); // Source data is on the stack
                self.text_section.push_str("\tcall _write_image\n");
                
                // Clean up the stack
                self.text_section.push_str("\tadd rsp, 8\n");
                self.stack_offset -= 8;
            }
        }
    }
    
    /// Generate the complete assembly for the program
    pub fn generate_assembly(&mut self, commands: Vec<Command<'a>>, _environment: TypeEnvironment<'a>) -> String {
        // Generate external declarations and imports section
        let mut imports = String::new();
        writeln!(imports, "\tglobal jpl_main").unwrap();
        writeln!(imports, "\tglobal _jpl_main").unwrap();
        writeln!(imports, "\textern _fail_assertion").unwrap();
        writeln!(imports, "\textern _jpl_alloc").unwrap();
        writeln!(imports, "\textern _get_time").unwrap();
        writeln!(imports, "\textern _show").unwrap();
        writeln!(imports, "\textern _print").unwrap();
        writeln!(imports, "\textern _print_time").unwrap();
        writeln!(imports, "\textern _read_image").unwrap();
        writeln!(imports, "\textern _write_image").unwrap();
        writeln!(imports, "\textern _fmod").unwrap();
        writeln!(imports, "\textern _sqrt").unwrap();
        writeln!(imports, "\textern _exp").unwrap();
        writeln!(imports, "\textern _sin").unwrap();
        writeln!(imports, "\textern _cos").unwrap();
        writeln!(imports, "\textern _tan").unwrap();
        writeln!(imports, "\textern _asin").unwrap();
        writeln!(imports, "\textern _acos").unwrap();
        writeln!(imports, "\textern _atan").unwrap();
        writeln!(imports, "\textern _log").unwrap();
        writeln!(imports, "\textern _pow").unwrap();
        writeln!(imports, "\textern _atan2").unwrap();
        writeln!(imports, "\textern _to_int").unwrap();
        writeln!(imports, "\textern _to_float").unwrap();
        writeln!(imports).unwrap();
        
        // Initialize the main function
        self.text_section.push_str("jpl_main:\n");
        self.text_section.push_str("_jpl_main:\n");
        self.text_section.push_str("\tpush rbp\n");
        self.text_section.push_str("\tmov rbp, rsp\n");
        self.text_section.push_str("\tpush r12\n");
        self.text_section.push_str("\tmov r12, rbp\n");
        
        // Process all commands
        for command in commands {
            self.generate_command(&command);
        }
        
        // Finalize the main function
        self.text_section.push_str("\n\t; Main function epilogue\n");
        self.text_section.push_str("\tpop r12\n");
        self.text_section.push_str("\tpop rbp\n");
        self.text_section.push_str("\tret\n");
        
        // Combine all sections
        let mut assembly = String::new();
        assembly.push_str(&imports);
        assembly.push_str(&self.data_section);
        assembly.push_str(&self.text_section);
        
        assembly
    }
}