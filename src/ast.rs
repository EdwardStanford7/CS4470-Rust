use crate::utils::Position;
use std::{cell::Cell, fmt::Display};

// -------------------------------------------------------------------------------------------- Command Nodes -----------------------------------------------------------------------------------------------

pub struct Command<'a> {
    pub position: Position,
    pub node: Box<CommandType<'a>>,
}

pub enum CommandType<'a> {
    Read {
        source: &'a str,
        destination: LValue<'a>,
    },
    Write {
        source: Expression<'a>,
        destination: &'a str,
    },
    Let {
        variable: LValue<'a>,
        rvalue: Expression<'a>,
    },
    Assert {
        condition: Expression<'a>,
        message: &'a str,
    },
    Print {
        message: &'a str,
    },
    Show {
        expression: Expression<'a>,
    },
    Time {
        command: Command<'a>,
    },
    Function {
        name: &'a str,
        parameters: Vec<(LValue<'a>, Type<'a>)>,
        return_type: Type<'a>,
        statements: Vec<Statement<'a>>,
        has_return: Cell<bool>,
    },
    Struct {
        name: &'a str,
        elements: Vec<(&'a str, Type<'a>)>,
    },
}

impl Display for Command<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.node.as_ref() {
            CommandType::Read {
                source,
                destination,
            } => {
                write!(f, "(ReadCmd {} {})", source, destination)
            }
            CommandType::Write {
                source,
                destination,
            } => {
                write!(f, "(WriteCmd {} {})", source, destination)
            }
            CommandType::Let { variable, rvalue } => {
                write!(f, "(LetCmd {} {})", variable, rvalue)
            }
            CommandType::Assert { condition, message } => {
                write!(f, "(AssertCmd {} {})", condition, message)
            }
            CommandType::Print { message } => {
                write!(f, "(PrintCmd {})", message)
            }
            CommandType::Show { expression } => {
                write!(f, "(ShowCmd {})", expression)
            }
            CommandType::Time { command } => {
                write!(f, "(TimeCmd {})", command)
            }
            CommandType::Function {
                name,
                parameters,
                return_type,
                statements,
                has_return: _,
            } => {
                let mut result = format!("(FnCmd {} ((", name);
                let mut first = true;
                for (var, typ) in parameters {
                    if !first {
                        result.push(' ');
                    }
                    result.push_str(&format!("{}{}", var, type_to_string(typ)));
                    first = false;
                }
                result.push_str(&format!(")){}", type_to_string(return_type)));
                for stmt in statements {
                    result.push_str(&format!(" {}", stmt));
                }
                result.push(')');
                write!(f, "{}", result)
            }
            CommandType::Struct { name, elements } => {
                let mut result = format!("(StructCmd {}", name);
                for (field, typ) in elements {
                    result.push_str(&format!(" {}{}", field, type_to_string(typ)));
                }
                result.push(')');
                write!(f, "{}", result)
            }
        }
    }
}

// -------------------------------------------------------------------------------------------- Expression Nodes -----------------------------------------------------------------------------------------------

#[derive(Debug)]
pub struct Expression<'a> {
    pub position: Position,
    pub node: Box<ExpressionType<'a>>,
    pub resolved_type: Type<'a>,
}

#[derive(Debug)]
pub enum ExpressionType<'a> {
    Int {
        value: i64,
    },
    Float {
        value: f64,
    },
    True,
    False,
    Void,
    Variable {
        name: &'a str,
    },
    ArrayLiteral {
        elements: Vec<Expression<'a>>,
    },
    ArrayIndex {
        array: Expression<'a>,
        indices: Vec<Expression<'a>>,
    },
    Dot {
        struct_variable: Expression<'a>,
        field: &'a str,
    },
    Call {
        function: &'a str,
        arguments: Vec<Expression<'a>>,
    },
    StructLiteral {
        name: &'a str,
        fields: Vec<Expression<'a>>,
    },
    Unop {
        operator: &'a str,
        expression: Expression<'a>,
    },
    Binop {
        operator: &'a str,
        left: Expression<'a>,
        right: Expression<'a>,
    },
    If {
        condition: Expression<'a>,
        then_branch: Expression<'a>,
        else_branch: Expression<'a>,
    },
    ArrayLoop {
        range: Vec<(&'a str, Expression<'a>)>,
        body: Expression<'a>,
    },
    SumLoop {
        range: Vec<(&'a str, Expression<'a>)>,
        body: Expression<'a>,
    },
}

impl Display for Expression<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.node.as_ref() {
            ExpressionType::Int { value } => write!(
                f,
                "(IntExpr{} {})",
                type_to_string(&self.resolved_type),
                value
            ),
            ExpressionType::Float { value } => {
                write!(
                    f,
                    "(FloatExpr{} {})",
                    type_to_string(&self.resolved_type),
                    *value as i64
                )
            }
            ExpressionType::True => write!(f, "(TrueExpr{})", type_to_string(&self.resolved_type)),
            ExpressionType::False => {
                write!(f, "(FalseExpr{})", type_to_string(&self.resolved_type))
            }
            ExpressionType::Void => write!(f, "(VoidExpr{})", type_to_string(&self.resolved_type)),
            ExpressionType::Variable { name } => {
                write!(
                    f,
                    "(VarExpr{} {})",
                    type_to_string(&self.resolved_type),
                    name
                )
            }
            ExpressionType::ArrayLiteral { elements } => {
                let mut result =
                    format!("(ArrayLiteralExpr{}", type_to_string(&self.resolved_type));
                for element in elements {
                    result.push_str(&format!(" {}", element));
                }
                result.push(')');
                write!(f, "{}", result)
            }
            ExpressionType::ArrayIndex { array, indices } => {
                let mut result = format!(
                    "(ArrayIndexExpr{} {}",
                    type_to_string(&self.resolved_type),
                    array
                );
                for index in indices {
                    result.push_str(&format!(" {}", index));
                }
                result.push(')');
                write!(f, "{}", result)
            }
            ExpressionType::Dot {
                struct_variable,
                field,
            } => {
                write!(
                    f,
                    "(DotExpr{} {} {})",
                    type_to_string(&self.resolved_type),
                    struct_variable,
                    field
                )
            }
            ExpressionType::Call {
                function,
                arguments,
            } => {
                let mut result = format!(
                    "(CallExpr{} {}",
                    type_to_string(&self.resolved_type),
                    function
                );
                for arg in arguments {
                    result.push_str(&format!(" {}", arg));
                }
                result.push(')');
                write!(f, "{}", result)
            }
            ExpressionType::StructLiteral { name, fields } => {
                let mut result = format!(
                    "(StructLiteralExpr{} {}",
                    type_to_string(&self.resolved_type),
                    name
                );
                for field in fields {
                    result.push_str(&format!(" {}", field));
                }
                result.push(')');
                write!(f, "{}", result)
            }
            ExpressionType::Unop {
                operator,
                expression,
            } => {
                write!(
                    f,
                    "(UnopExpr{} {} {})",
                    type_to_string(&self.resolved_type),
                    operator,
                    expression
                )
            }
            ExpressionType::Binop {
                operator,
                left,
                right,
            } => write!(
                f,
                "(BinopExpr{} {} {} {})",
                type_to_string(&self.resolved_type),
                left,
                operator,
                right
            ),
            ExpressionType::If {
                condition,
                then_branch,
                else_branch,
            } => write!(
                f,
                "(IfExpr{} {} {} {})",
                type_to_string(&self.resolved_type),
                condition,
                then_branch,
                else_branch
            ),
            ExpressionType::ArrayLoop { range, body } => {
                let mut result = format!("(ArrayLoopExpr{}", type_to_string(&self.resolved_type));
                for (var, expr) in range {
                    result.push_str(&format!(" {} {}", var, expr));
                }
                result.push_str(&format!(" {})", body));
                write!(f, "{}", result)
            }
            ExpressionType::SumLoop { range, body } => {
                let mut result = format!("(SumLoopExpr{}", type_to_string(&self.resolved_type));
                for (var, expr) in range {
                    result.push_str(&format!(" {} {}", var, expr));
                }
                result.push_str(&format!(" {})", body));
                write!(f, "{}", result)
            }
        }
    }
}

// -------------------------------------------------------------------------------------------- Statement Nodes -----------------------------------------------------------------------------------------------

pub struct Statement<'a> {
    pub node: StatementType<'a>,
}

pub enum StatementType<'a> {
    Let {
        variable: LValue<'a>,
        rvalue: Expression<'a>,
    },
    Assert {
        condition: Expression<'a>,
        message: &'a str,
    },
    Return {
        value: Expression<'a>,
    },
}

impl Display for Statement<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.node {
            StatementType::Let { variable, rvalue } => {
                write!(f, "(LetStmt {} {})", variable, rvalue)
            }
            StatementType::Assert { condition, message } => {
                write!(f, "(AssertStmt {} {})", condition, message)
            }
            StatementType::Return { value } => {
                write!(f, "(ReturnStmt {})", value)
            }
        }
    }
}

// -------------------------------------------------------------------------------------------- Type Nodes -----------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type<'a> {
    Unresolved,
    Int,
    Float,
    Bool,
    Void,
    Struct {
        name: &'a str,
        elements: Vec<(&'a str, Type<'a>)>,
    },
    Array {
        element_type: Box<Type<'a>>,
        rank: usize,
    },
    Function {
        param_types: Vec<Type<'a>>,
        return_type: Box<Type<'a>>,
    },
}

impl<'a> Type<'a> {
    pub fn isize(&self) -> isize {
        match self {
            Type::Unresolved => unreachable!(),
            Type::Int => 8,
            Type::Float => 8,
            Type::Bool => 8,
            Type::Void => 8,
            Type::Struct { .. } => unreachable!(),
            Type::Array {
                element_type: _,
                rank,
            } => 8 + 8 * *rank as isize,
            Type::Function { .. } => unreachable!(),
        }
    }
    pub fn usize(&self) -> usize {
        match self {
            Type::Unresolved => unreachable!(),
            Type::Int => 8,
            Type::Float => 8,
            Type::Bool => 8,
            Type::Void => 8,
            Type::Struct { .. } => unreachable!(),
            Type::Array {
                element_type: _,
                rank,
            } => 8 + 8 * *rank as usize,
            Type::Function { .. } => unreachable!(),
        }
    }
}

fn type_to_string(typ: &Type<'_>) -> String {
    match typ {
        Type::Unresolved => "".to_string(),
        _ => format!(" {}", typ),
    }
}

impl Display for Type<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Type::Unresolved => write!(f, ""),
            Type::Int => write!(f, "(IntType)"),
            Type::Float => write!(f, "(FloatType)"),
            Type::Bool => write!(f, "(BoolType)"),
            Type::Void => write!(f, "(VoidType)"),
            Type::Struct { name, elements: _ } => write!(f, "(StructType {})", name),
            Type::Array { element_type, rank } => {
                write!(f, "(ArrayType {} {})", element_type, rank)
            }
            Type::Function {
                param_types: _,
                return_type,
            } => write!(f, "{}", return_type),
        }
    }
}

// -------------------------------------------------------------------------------------------- LValue Nodes -----------------------------------------------------------------------------------------------

pub struct LValue<'a> {
    pub position: Position,
    pub name: &'a str,
    pub node: LValueType<'a>,
}

pub enum LValueType<'a> {
    Variable,
    Array { indices: Vec<&'a str> },
}

impl Display for LValue<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.node {
            LValueType::Variable => write!(f, "(VarLValue {})", self.name),
            LValueType::Array { indices } => {
                let mut result = format!("(ArrayLValue {}", self.name);
                for idx in indices {
                    result.push_str(&format!(" {}", idx));
                }
                result.push(')');
                write!(f, "{}", result)
            }
        }
    }
}
