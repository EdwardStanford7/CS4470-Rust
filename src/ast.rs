use crate::utils::Position;
use std::{
    cell::Cell,
    fmt::{self, Display},
};

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
                write!(f, "(ReadCmd \"{}\" {})", source, destination)
            }
            CommandType::Write {
                source,
                destination,
            } => {
                write!(f, "(WriteCmd {} \"{}\")", source, destination)
            }
            CommandType::Let { variable, rvalue } => {
                write!(f, "(LetCmd {} {})", variable, rvalue)
            }
            CommandType::Assert { condition, message } => {
                write!(f, "(AssertCmd {} \"{}\")", condition, message)
            }
            CommandType::Print { message } => {
                write!(f, "(PrintCmd \"{}\")", message)
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
                write!(f, "(FnCmd {} ((", name)?;
                let mut first = true;
                for (var, typ) in parameters {
                    if !first {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", var)?;
                    write_type_suffix(f, typ)?;
                    first = false;
                }
                write!(f, "))")?;
                write_type_suffix(f, return_type)?;
                for stmt in statements {
                    write!(f, " {}", stmt)?;
                }
                write!(f, ")")
            }
            CommandType::Struct { name, elements } => {
                write!(f, "(StructCmd {}", name)?;
                for (field, typ) in elements {
                    write!(f, " {}", field)?;
                    write_type_suffix(f, typ)?;
                }
                write!(f, ")")
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
            ExpressionType::Int { value } => {
                write!(f, "(IntExpr")?;
                write_type_suffix(f, &self.resolved_type)?;
                write!(f, " {})", value)
            }
            ExpressionType::Float { value } => {
                write!(f, "(FloatExpr")?;
                write_type_suffix(f, &self.resolved_type)?;
                write!(f, " {})", *value as i64)
            }
            ExpressionType::True => {
                write!(f, "(TrueExpr")?;
                write_type_suffix(f, &self.resolved_type)?;
                write!(f, ")")
            }
            ExpressionType::False => {
                write!(f, "(FalseExpr")?;
                write_type_suffix(f, &self.resolved_type)?;
                write!(f, ")")
            }
            ExpressionType::Void => {
                write!(f, "(VoidExpr")?;
                write_type_suffix(f, &self.resolved_type)?;
                write!(f, ")")
            }
            ExpressionType::Variable { name } => {
                write!(f, "(VarExpr")?;
                write_type_suffix(f, &self.resolved_type)?;
                write!(f, " {})", name)
            }
            ExpressionType::ArrayLiteral { elements } => {
                write!(f, "(ArrayLiteralExpr")?;
                write_type_suffix(f, &self.resolved_type)?;
                for element in elements {
                    write!(f, " {}", element)?;
                }
                write!(f, ")")
            }
            ExpressionType::ArrayIndex { array, indices } => {
                write!(f, "(ArrayIndexExpr")?;
                write_type_suffix(f, &self.resolved_type)?;
                write!(f, " {}", array)?;
                for index in indices {
                    write!(f, " {}", index)?;
                }
                write!(f, ")")
            }
            ExpressionType::Dot {
                struct_variable,
                field,
            } => {
                write!(f, "(DotExpr")?;
                write_type_suffix(f, &self.resolved_type)?;
                write!(f, " {} {})", struct_variable, field)
            }
            ExpressionType::Call {
                function,
                arguments,
            } => {
                write!(f, "(CallExpr")?;
                write_type_suffix(f, &self.resolved_type)?;
                write!(f, " {}", function)?;
                for arg in arguments {
                    write!(f, " {}", arg)?;
                }
                write!(f, ")")
            }
            ExpressionType::StructLiteral { name, fields } => {
                write!(f, "(StructLiteralExpr")?;
                write_type_suffix(f, &self.resolved_type)?;
                write!(f, " {}", name)?;
                for field in fields {
                    write!(f, " {}", field)?;
                }
                write!(f, ")")
            }
            ExpressionType::Unop {
                operator,
                expression,
            } => {
                write!(f, "(UnopExpr")?;
                write_type_suffix(f, &self.resolved_type)?;
                write!(f, " {} {})", operator, expression)
            }
            ExpressionType::Binop {
                operator,
                left,
                right,
            } => {
                write!(f, "(BinopExpr")?;
                write_type_suffix(f, &self.resolved_type)?;
                write!(f, " {} {} {})", left, operator, right)
            }
            ExpressionType::If {
                condition,
                then_branch,
                else_branch,
            } => {
                write!(f, "(IfExpr")?;
                write_type_suffix(f, &self.resolved_type)?;
                write!(f, " {} {} {})", condition, then_branch, else_branch)
            }
            ExpressionType::ArrayLoop { range, body } => {
                write!(f, "(ArrayLoopExpr")?;
                write_type_suffix(f, &self.resolved_type)?;
                for (var, expr) in range {
                    write!(f, " {} {}", var, expr)?;
                }
                write!(f, " {})", body)
            }
            ExpressionType::SumLoop { range, body } => {
                write!(f, "(SumLoopExpr")?;
                write_type_suffix(f, &self.resolved_type)?;
                for (var, expr) in range {
                    write!(f, " {} {}", var, expr)?;
                }
                write!(f, " {})", body)
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
                write!(f, "(AssertStmt {} \"{}\")", condition, message)
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

impl Type<'_> {
    pub fn isize(&self) -> isize {
        match self {
            Type::Unresolved => unreachable!(),
            Type::Int => 8,
            Type::Float => 8,
            Type::Bool => 8,
            Type::Void => 8,
            Type::Struct { name: _, elements } => {
                let mut size = 0;
                for (_, typ) in elements {
                    size += typ.isize();
                }
                size
            }
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
            Type::Struct { name: _, elements } => {
                let mut size = 0;
                for (_, typ) in elements {
                    size += typ.usize();
                }
                size
            }
            Type::Array {
                element_type: _,
                rank,
            } => 8 + 8 * *rank,
            Type::Function { .. } => unreachable!(),
        }
    }
}

fn write_type_suffix(f: &mut fmt::Formatter<'_>, typ: &Type<'_>) -> fmt::Result {
    match typ {
        Type::Unresolved => Ok(()),
        _ => write!(f, " {}", typ),
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
                write!(f, "(ArrayLValue {}", self.name)?;
                for idx in indices {
                    write!(f, " {}", idx)?;
                }
                write!(f, ")")
            }
        }
    }
}
