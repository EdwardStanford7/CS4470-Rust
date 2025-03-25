use std::collections::HashMap;
use std::fmt::Write as _;

use crate::ast::*;
use crate::utils::*;
use crate::asm_consts::asm_consts::*;

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
            data_section: String::from(DATA_SECTION),
            text_section: String::from(TEXT_SECTION),
            stack_offset: 0,
        }
    }

    /// Generate a unique label for jumps and branches
    fn next_label(&mut self, prefix: &str) -> String {
        self.label_counter += 1;
        let label = format!(".{}{}", prefix, self.label_counter);
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
        self.data_section.push_str(&format!("const{}: db `{}`, 0\n", counter, s));
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
        self.load_constant(expr, RAX);
        self.push_value(RAX);
    }

    /// Generate assembly for unary operations
    fn generate_unop(&mut self, operator: &str, expr: &Expression) {
        self.generate_expression(expr);
        
        match expr.resolved_type {
            Type::Bool => {
                self.pop_value(RAX);
                self.text_section.push_str(INT_XOR_BOOL);
                self.push_value(RAX);
            }
            Type::Float => {
                self.text_section.push_str("\tmovsd xmm1, [rsp]\n");
                self.text_section.push_str("\tadd rsp, 8\n");
                self.stack_offset -= 8;
                self.text_section.push_str("\tpxor xmm0, xmm0\n");
                self.text_section.push_str("\tsubsd xmm0, xmm1\n");
                self.text_section.push_str("\tsub rsp, 8\n");
                self.stack_offset += 8;
                self.text_section.push_str("\tmovsd [rsp], xmm0\n");
            }
            Type::Int => {
                self.pop_value(RAX);
                self.text_section.push_str("\tneg rax\n");
                self.push_value(RAX);
            }
            _ => panic!("Unsupported type for unary operation: {}", expr.resolved_type),
        }
    }

    fn generate_binop(&mut self, operator: &str, left: &Expression, right: &Expression) {
        // Generate code for the right operand first (stack order)
        self.generate_expression(right);
        self.generate_expression(left);
        
        match left.resolved_type {
            Type::Int => {
                self.pop_value(RAX);  // Dividend in rax
                self.pop_value(R10);  // Divisor in r10
                
                match operator {
                    "+" => {
                        self.text_section.push_str(INT_ADD);
                        self.push_value(RAX);
                    },
                    "-" => {
                        self.text_section.push_str(INT_SUB);
                        self.push_value(RAX);
                    },
                    "*" => {
                        self.text_section.push_str(INT_MUL);
                        self.push_value(RAX);
                    },
                    "/" => {
                        let ok_label = self.next_label(JUMP);
                        self.text_section.push_str("\tcmp r10, 0\n");
                        self.text_section.push_str(&format!("\tjne {}\n", ok_label));
                        
                        let err_msg = self.add_string(DBZ);
                        self.text_section.push_str(STACK_ALIGN_COMMENT);
                        self.text_section.push_str(&format!("\tlea rdi, [rel const{}] ; '{}'\n", err_msg, DBZ));
                        self.text_section.push_str("\tcall _fail_assertion\n");
                        self.text_section.push_str(STACK_UNALIGN_COMMENT);
                        
                        self.text_section.push_str(&format!("{}:\n", ok_label));
                        self.text_section.push_str(INT_DIV_PREP);
                        self.text_section.push_str(INT_DIV);
                        self.push_value(RAX);
                    },
                    "%" => {
                        let ok_label = self.next_label(JUMP);
                        self.text_section.push_str("\tcmp r10, 0\n");
                        self.text_section.push_str(&format!("\tjne {}\n", ok_label));
                        
                        let err_msg = self.add_string(MBZ);
                        self.text_section.push_str(STACK_ALIGN_COMMENT);
                        self.text_section.push_str(&format!("\tlea rdi, [rel const{}] ; '{}'\n", err_msg, MBZ));
                        self.text_section.push_str("\tcall _fail_assertion\n");
                        self.text_section.push_str(STACK_UNALIGN_COMMENT);
                        
                        self.text_section.push_str(&format!("{}:\n", ok_label));
                        self.text_section.push_str(INT_DIV_PREP);
                        self.text_section.push_str(INT_DIV);
                        self.text_section.push_str(INT_MOV_REMAINDER);
                        self.push_value(RAX);
                    },
                    "==" => {
                        self.text_section.push_str(INT_CMP);
                        self.text_section.push_str(INT_SETE);
                        self.text_section.push_str(INT_AND_ONE);
                        self.push_value(RAX);
                    },
                    "!=" => {
                        self.text_section.push_str(INT_CMP);
                        self.text_section.push_str(INT_SETNE);
                        self.text_section.push_str(INT_AND_ONE);
                        self.push_value(RAX);
                    },
                    "<" => {
                        self.text_section.push_str(INT_CMP);
                        self.text_section.push_str(INT_SETL);
                        self.text_section.push_str(INT_AND_ONE);
                        self.push_value(RAX);
                    },
                    "<=" => {
                        self.text_section.push_str(INT_CMP);
                        self.text_section.push_str(INT_SETLE);
                        self.text_section.push_str(INT_AND_ONE);
                        self.push_value(RAX);
                    },
                    ">" => {
                        self.text_section.push_str(INT_CMP);
                        self.text_section.push_str(INT_SETG);
                        self.text_section.push_str(INT_AND_ONE);
                        self.push_value(RAX);
                    },
                    ">=" => {
                        self.text_section.push_str(INT_CMP);
                        self.text_section.push_str(INT_SETGE);
                        self.text_section.push_str(INT_AND_ONE);
                        self.push_value(RAX);
                    },
                    _ => panic!("Unsupported binary operator for integers: {}", operator),
                }
            },
            Type::Float => {
                self.text_section.push_str("\tmovsd xmm0, [rsp]\n");
                self.text_section.push_str("\tadd rsp, 8\n");
                self.stack_offset -= 8;
                self.text_section.push_str("\tmovsd xmm1, [rsp]\n");
                self.text_section.push_str("\tadd rsp, 8\n");
                self.stack_offset -= 8;
                
                match operator {
                    "+" => self.text_section.push_str(XMM_ADD),
                    "-" => self.text_section.push_str(XMM_SUB),
                    "*" => self.text_section.push_str(XMM_MUL),
                    "/" => self.text_section.push_str(XMM_DIV),
                    _ => panic!("Unsupported binary operator for floats: {}", operator),
                }
                
                self.text_section.push_str("\tsub rsp, 8\n");
                self.stack_offset += 8;
                self.text_section.push_str("\tmovsd [rsp], xmm0\n");
            },
            Type::Bool => {
                self.pop_value(RAX);
                self.pop_value(R10);
                
                match operator {
                    "&&" => {
                        self.text_section.push_str(INT_AND);
                        self.push_value(RAX);
                    },
                    "||" => {
                        self.text_section.push_str(INT_OR);
                        self.push_value(RAX);
                    },
                    "==" => {
                        self.text_section.push_str(INT_CMP);
                        self.text_section.push_str(INT_SETE);
                        self.text_section.push_str(INT_AND_ONE);
                        self.push_value(RAX);
                    },
                    "!=" => {
                        self.text_section.push_str(INT_CMP);
                        self.text_section.push_str(INT_SETNE);
                        self.text_section.push_str(INT_AND_ONE);
                        self.push_value(RAX);
                    },
                    _ => panic!("Unsupported binary operator for booleans: {}", operator),
                }
            },
            _ => panic!("Unsupported type for binary operation: {}", left.resolved_type),
        }
    }

    /// Generate assembly for if expressions
    fn generate_if_expression(&mut self, condition: &Expression, then_branch: &Expression, else_branch: &Expression) {
        let else_label = self.next_label(LABEL_ELSE);
        let end_label = self.next_label(LABEL_ENDIF);
        
        self.generate_expression(condition);
        self.pop_value(RAX);
        self.text_section.push_str(INT_TEST);
        self.text_section.push_str(&format!("\tjz {}\n", else_label));
        
        self.generate_expression(then_branch);
        self.text_section.push_str(&format!("\tjmp {}\n", end_label));
        
        self.text_section.push_str(&format!("{}:\n", else_label));
        self.generate_expression(else_branch);
        
        self.text_section.push_str(&format!("{}:\n", end_label));
    }

    /// Generate assembly for a variable reference
    fn generate_variable(&mut self, name: &str) {
        if let Some(location) = self.variable_map.get(name) {
            match location {
                VariableLocation::Stack(offset) => {
                    self.text_section.push_str(&format!("\tmov rax, [{}{}]\n", RBP, if *offset >= 0 { format!("+{}", offset) } else { format!("{}", offset) }));
                    self.push_value(RAX);
                },
                VariableLocation::Register(reg) => {
                    self.text_section.push_str(&format!("\tmov rax, {}\n", reg));
                    self.push_value(RAX);
                },
                VariableLocation::Global(label) => {
                    self.text_section.push_str(&format!("\tmov rax, [rel {}]\n", label));
                    self.push_value(RAX);
                },
            }
        } else {
            self.text_section.push_str(&format!("\t; LOAD_VAR {} (unresolved)\n", name));
            self.text_section.push_str("\tmov rax, 0\n");
            self.push_value(RAX);
        }
    }

    /// Generate assembly for a function call
    fn generate_function_call(&mut self, function: &str, arguments: &[Expression]) {
        self.text_section.push_str("\t; Save registers for function call\n");
        
        for arg in arguments.iter().rev() {
            self.generate_expression(arg);
        }
        
        let arg_regs = ARG_REGISTERS;
        let stack_args = arguments.len().saturating_sub(arg_regs.len());
        
        for (i, reg) in arg_regs.iter().enumerate().take(arguments.len().min(arg_regs.len())) {
            self.pop_value(reg);
        }
        
        self.text_section.push_str(&format!("\tcall {}\n", function));
        
        if stack_args > 0 {
            self.text_section.push_str(&format!("\tadd rsp, {}\n", stack_args * 8));
            self.stack_offset -= (stack_args * 8) as i32;
        }
        
        self.push_value(RAX);
    }

    /// Generate assembly for array access
    fn generate_array_index(&mut self, array: &Expression, indices: &[Expression]) {
        self.text_section.push_str(TODO_ARRAY_INDEX);
        self.generate_expression(array);
        
        for index in indices {
            self.generate_expression(index);
            self.pop_value("rcx");
            self.pop_value(RAX);
            self.text_section.push_str("\timul rcx, 8\n");
            self.text_section.push_str("\tadd rax, rcx\n");
            self.text_section.push_str("\tmov rax, [rax]\n");
            self.push_value(RAX);
        }
    }

    /// Generate assembly for a loop (array or sum)
    fn generate_loop(&mut self, range: &[(&str, Expression)], body: &Expression, is_sum: bool) {
        self.text_section.push_str(&format!("// TODO: Implement {} loop\n", if is_sum { "sum" } else { "array" }));
        self.generate_expression(body);
    }
    
    /// Generate assembly code for an expression
    pub fn generate_expression(&mut self, expr: &Expression) {
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
                self.text_section.push_str("\t; void expression (no operation)\n");
            }
            ExpressionType::ArrayLiteral { elements } => {
                self.text_section.push_str(TODO_ARRAY_LITERALS);
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
                self.generate_expression(rvalue);
                self.text_section.push_str(&format!("\t; Let statement: {}\n", variable.name));
            }
            StatementType::Assert { condition, message } => {
                self.text_section.push_str(&format!("\t; Assert: {}\n", message));
                self.generate_expression(condition);
                self.pop_value(RAX);
                let skip_label = self.next_label(LABEL_ASSERT_PASS);
                self.text_section.push_str(INT_TEST);
                self.text_section.push_str(&format!("\tjnz {}\n", skip_label));
                let msg_id = self.add_string(message);
                self.text_section.push_str(&format!("\tlea rdi, [rel const{}]\n", msg_id));
                self.text_section.push_str("\tcall _fail_assertion\n");
                self.text_section.push_str(&format!("{}:\n", skip_label));
            }
            StatementType::Return { value } => {
                self.text_section.push_str("\t; Return statement\n");
                self.generate_expression(value);
                self.pop_value(RAX);
                self.text_section.push_str(FUNCTION_EPILOGUE_1);
                self.text_section.push_str(FUNCTION_EPILOGUE_2);
                self.text_section.push_str(FUNCTION_EPILOGUE_3);
            }
        }
    }
    
    /// Generate assembly for a single command
    fn generate_command(&mut self, command: &Command<'a>) {
        match command.node.as_ref() {
            CommandType::Assert { condition, message } => {
                self.text_section.push_str(&format!("\t; Assert: {}\n", message));
                self.generate_expression(condition);
                self.pop_value(RAX);
                let skip_label = self.next_label(LABEL_ASSERT_PASS);
                self.text_section.push_str(INT_TEST);
                self.text_section.push_str(&format!("\tjnz {}\n", skip_label));
                let msg_id = self.add_string(message);
                self.text_section.push_str(&format!("\tlea rdi, [rel const{}]\n", msg_id));
                self.text_section.push_str("\tcall _fail_assertion\n");
                self.text_section.push_str(&format!("{}:\n", skip_label));
            }
            CommandType::Function { name, parameters, return_type: _, statements, has_return: _ } => {
                self.text_section.push_str(&format!("\n{}{}", MAIN_LABEL_1, MAIN_LABEL_2));
                self.text_section.push_str(FUNCTION_PROLOGUE_1);
                self.text_section.push_str(FUNCTION_PROLOGUE_2);
                
                self.stack_offset = 0;
                self.text_section.push_str(FUNCTION_SAVE_REGS);
                self.stack_offset += 32;
                
                let arg_regs = ARG_REGISTERS;
                for (i, (param, _)) in parameters.iter().enumerate() {
                    if i < arg_regs.len() {
                        self.text_section.push_str(&format!("\tpush {}\n", arg_regs[i]));
                        self.stack_offset += 8;
                        self.variable_map.insert(param.name, VariableLocation::Stack(-self.stack_offset));
                    } else {
                        let offset = (i - arg_regs.len() + 2) * 8;
                        self.variable_map.insert(param.name, VariableLocation::Stack(offset as i32));
                    }
                }
                
                for stmt in statements {
                    self.generate_statement(stmt);
                }
                
                self.text_section.push_str("\n\t; Function epilogue\n");
                self.text_section.push_str(FUNCTION_RESTORE_REGS);
                self.text_section.push_str(FUNCTION_EPILOGUE_1);
                self.text_section.push_str(FUNCTION_EPILOGUE_2);
                self.text_section.push_str(FUNCTION_EPILOGUE_3);
                self.variable_map.clear();
            }
            CommandType::Let { variable, rvalue } => {
                self.generate_expression(rvalue);
                self.text_section.push_str(&format!("\t; Assign to {}\n", variable.name));
                let var_offset = -self.stack_offset;
                self.variable_map.insert(variable.name, VariableLocation::Stack(var_offset));
            }
            CommandType::Print { message } => {
                let msg_id = self.add_string(message);
                self.text_section.push_str(&format!("\tlea rdi, [rel const{}]\n", msg_id));
                self.text_section.push_str("\tcall _print\n");
            }
            CommandType::Read { source, destination } => {
                let source_id = self.add_string(source);
                self.text_section.push_str(&format!("\tlea rdi, [rel const{}]\n", source_id));
                self.text_section.push_str("\t; TODO: Setup destination array\n");
                self.text_section.push_str("\tcall _read_image\n");
                self.text_section.push_str(&format!("\t; Store result in {}\n", destination.name));
            }
            CommandType::Show { expression } => {
                self.generate_expression(expression);
                let (comment, type_id) = self.get_type_name(&expression.resolved_type);
                self.text_section.push_str(&format!("\tlea rdi, [rel const{}] ; {}\n", type_id, comment));
                self.text_section.push_str("\tlea rsi, [rsp]\n");
                self.text_section.push_str("\tcall _show\n");
                self.text_section.push_str("\tadd rsp, 8\n");
                self.stack_offset -= 8;
            }
            CommandType::Struct { name, elements } => {
                self.text_section.push_str(&format!("\t; Struct definition: {}\n", name));
                for (field, typ) in elements {
                    self.text_section.push_str(&format!("\t; Field: {} Type: {}\n", field, typ));
                }
            }
            CommandType::Time { command } => {
                self.text_section.push_str("\tcall _get_time\n");
                self.text_section.push_str("\tpush rax\n");
                self.stack_offset += 8;
                self.generate_command(command);
                self.text_section.push_str("\tpop rdi\n");
                self.stack_offset -= 8;
                self.text_section.push_str("\tcall _print_time\n");
            }
            CommandType::Write { source, destination } => {
                self.generate_expression(source);
                let dest_id = self.add_string(destination);
                self.text_section.push_str(&format!("\tlea rdi, [rel const{}]\n", dest_id));
                self.text_section.push_str("\tmov rsi, rsp\n");
                self.text_section.push_str("\tcall _write_image\n");
                self.text_section.push_str("\tadd rsp, 8\n");
                self.stack_offset -= 8;
            }
        }
    }
    
    /// Generate the complete assembly for the program
    pub fn generate_assembly(&mut self, commands: Vec<Command<'a>>, _environment: TypeEnvironment<'a>) -> String {
        let imports = String::from(IMPORTS);
        
        self.text_section.push_str(MAIN_LABEL_1);
        self.text_section.push_str(MAIN_LABEL_2);
        self.text_section.push_str("\tpush rbp\n");
        self.text_section.push_str("\tmov rbp, rsp\n");
        self.text_section.push_str("\tpush r12\n");
        self.text_section.push_str("\tmov r12, rbp\n");
        
        for command in commands {
            self.generate_command(&command);
        }
        
        self.text_section.push_str("\n\t; Main function epilogue\n");
        self.text_section.push_str("\tpop r12\n");
        self.text_section.push_str("\tpop rbp\n");
        self.text_section.push_str("\tret\n");
        
        let mut assembly = String::new();
        assembly.push_str(&imports);
        assembly.push_str(&self.data_section);
        assembly.push_str(&self.text_section);
        
        assembly
    }
}