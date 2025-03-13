use crate::lexer::Position;
use core::str;
use std::{cell::RefCell, collections::HashMap, fmt::Display, rc::Rc};

// -------------------------------------------------------------------------------------------- Command Nodes -----------------------------------------------------------------------------------------------

pub struct Command<'a> {
    pub position: Position,
    pub node: CommandType<'a>,
}

pub enum CommandType<'a> {
    Read {
        source: &'a str,
        destination: Box<LValue<'a>>,
    },
    Write {
        source: Box<Expression<'a>>,
        destination: &'a str,
    },
    Let {
        variable: Box<LValue<'a>>,
        rvalue: Box<Expression<'a>>,
    },
    Assert {
        condition: Box<Expression<'a>>,
        message: &'a str,
    },
    Print {
        message: &'a str,
    },
    Show {
        expression: Box<Expression<'a>>,
    },
    Time {
        command: Box<Command<'a>>,
    },
    Function {
        name: &'a str,
        parameters: Vec<(LValue<'a>, Type<'a>)>,
        return_type: Box<Type<'a>>,
        statements: Vec<Statement<'a>>,
        has_return: bool,
        local_scope: Option<HashMap<&'a str, Type<'a>>>,
    },
    Struct {
        name: &'a str,
        elements: Vec<(&'a str, Type<'a>)>,
    },
}

impl<'a> Display for Command<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.node {
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
                local_scope: _,
            } => {
                let mut result = format!("(FnCmd {} ((", name);
                let mut first = true;
                for (var, typ) in parameters {
                    if !first {
                        result.push(' ');
                    }
                    result.push_str(&format!("{} {}", var, typ));
                    first = false;
                }
                result.push_str(&format!(")) {}", return_type));
                for stmt in statements {
                    result.push_str(&format!(" {}", stmt));
                }
                result.push(')');
                write!(f, "{}", result)
            }
            CommandType::Struct { name, elements } => {
                let mut result = format!("(StructCmd {}", name);
                for (field, typ) in elements {
                    result.push_str(&format!(" {} {}", field, typ));
                }
                result.push(')');
                write!(f, "{}", result)
            }
        }
    }
}

// -------------------------------------------------------------------------------------------- Expression Nodes -----------------------------------------------------------------------------------------------

pub enum Unop {
    Negative,
    Not,
}

impl Display for Unop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Unop::Negative => write!(f, "-"),
            Unop::Not => write!(f, "!"),
        }
    }
}

pub enum Binop {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Less,
    Greater,
    Equals,
    NotEquals,
    LessEquals,
    GreaterEquals,
    And,
    Or,
}

impl Display for Binop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Binop::Add => write!(f, "+"),
            Binop::Subtract => write!(f, "-"),
            Binop::Multiply => write!(f, "*"),
            Binop::Divide => write!(f, "/"),
            Binop::Modulo => write!(f, "%"),
            Binop::Less => write!(f, "<"),
            Binop::Greater => write!(f, ">"),
            Binop::Equals => write!(f, "=="),
            Binop::NotEquals => write!(f, "!="),
            Binop::LessEquals => write!(f, "<="),
            Binop::GreaterEquals => write!(f, ">="),
            Binop::And => write!(f, "&&"),
            Binop::Or => write!(f, "||"),
        }
    }
}

impl Binop {
    pub fn from_str(s: &str) -> Binop {
        match s {
            "+" => Binop::Add,
            "-" => Binop::Subtract,
            "*" => Binop::Multiply,
            "/" => Binop::Divide,
            "%" => Binop::Modulo,
            "<" => Binop::Less,
            ">" => Binop::Greater,
            "==" => Binop::Equals,
            "!=" => Binop::NotEquals,
            "<=" => Binop::LessEquals,
            ">=" => Binop::GreaterEquals,
            "&&" => Binop::And,
            "||" => Binop::Or,
            _ => panic!("Invalid binary operator: {}", s),
        }
    }
}

pub struct Expression<'a> {
    pub position: Position,
    pub node: ExpressionType<'a>,
    pub resolved_type: Rc<RefCell<Option<Type<'a>>>>,
}

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
        array: Box<Expression<'a>>,
        indices: Vec<Expression<'a>>,
    },
    Dot {
        struct_variable: Box<Expression<'a>>,
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
        operator: Unop,
        expression: Box<Expression<'a>>,
    },
    Binop {
        operator: Binop,
        left: Box<Expression<'a>>,
        right: Box<Expression<'a>>,
    },
    If {
        condition: Box<Expression<'a>>,
        then_branch: Box<Expression<'a>>,
        else_branch: Box<Expression<'a>>,
    },
    ArrayLoop {
        range: Vec<(&'a str, Expression<'a>)>,
        body: Box<Expression<'a>>,
    },
    SumLoop {
        range: Vec<(&'a str, Expression<'a>)>,
        body: Box<Expression<'a>>,
    },
}

impl<'a> Display for Expression<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let type_str = if let Some(typ) = self.resolved_type.borrow().as_ref() {
            format!(" {}", typ)
        } else {
            String::new()
        };

        match &self.node {
            ExpressionType::Int { value } => write!(f, "(IntExpr{} {})", type_str, value),
            ExpressionType::Float { value } => {
                write!(f, "(FloatExpr{} {})", type_str, *value as i64)
            }
            ExpressionType::True => write!(f, "(TrueExpr{})", type_str),
            ExpressionType::False => write!(f, "(FalseExpr{})", type_str),
            ExpressionType::Void => write!(f, "(VoidExpr{})", type_str),
            ExpressionType::Variable { name } => write!(f, "(VarExpr{} {})", type_str, name),
            ExpressionType::ArrayLiteral { elements } => {
                let mut result = format!("(ArrayLiteralExpr{}", type_str);
                for element in elements {
                    result.push_str(&format!(" {}", element));
                }
                result.push(')');
                write!(f, "{}", result)
            }
            ExpressionType::ArrayIndex { array, indices } => {
                let mut result = format!("(ArrayIndexExpr{} {}", type_str, array);
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
                write!(f, "(DotExpr{} {} {})", type_str, struct_variable, field)
            }
            ExpressionType::Call {
                function,
                arguments,
            } => {
                let mut result = format!("(CallExpr{} {}", type_str, function);
                for arg in arguments {
                    result.push_str(&format!(" {}", arg));
                }
                result.push(')');
                write!(f, "{}", result)
            }
            ExpressionType::StructLiteral { name, fields } => {
                let mut result = format!("(StructLiteralExpr{} {}", type_str, name);
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
                write!(f, "(UnopExpr{} {} {})", type_str, operator, expression)
            }
            ExpressionType::Binop {
                operator,
                left,
                right,
            } => write!(f, "(BinopExpr{} {} {} {})", type_str, left, operator, right),
            ExpressionType::If {
                condition,
                then_branch,
                else_branch,
            } => write!(
                f,
                "(IfExpr{} {} {} {})",
                type_str, condition, then_branch, else_branch
            ),
            ExpressionType::ArrayLoop { range, body } => {
                let mut result = format!("(ArrayLoopExpr{}", type_str);
                for (var, expr) in range {
                    result.push_str(&format!(" {} {}", var, expr));
                }
                result.push_str(&format!(" {})", body));
                write!(f, "{}", result)
            }
            ExpressionType::SumLoop { range, body } => {
                let mut result = format!("(SumLoopExpr{}", type_str);
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
    pub position: Position,
    pub node: StatementType<'a>,
}

pub enum StatementType<'a> {
    Let {
        variable: Box<LValue<'a>>,
        rvalue: Box<Expression<'a>>,
    },
    Assert {
        condition: Box<Expression<'a>>,
        message: &'a str,
    },
    Return {
        value: Box<Expression<'a>>,
    },
}

impl<'a> Display for Statement<'a> {
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

#[derive(Debug)]
pub struct Type<'a> {
    pub position: Position,
    pub node: TypeValue<'a>,
}

#[derive(Debug)]
pub enum TypeValue<'a> {
    Int,
    Float,
    Bool,
    Void,
    Struct {
        name: &'a str,
        elements: Option<Vec<(&'a str, Type<'a>)>>, // Only exists once resolved.
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

impl<'a> Display for Type<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.node {
            TypeValue::Int => write!(f, "(IntType)"),
            TypeValue::Float => write!(f, "(FloatType)"),
            TypeValue::Bool => write!(f, "(BoolType)"),
            TypeValue::Void => write!(f, "(VoidType)"),
            TypeValue::Struct { name, elements: _ } => write!(f, "(StructType {})", name),
            TypeValue::Array { element_type, rank } => {
                write!(f, "(ArrayType {} {})", element_type, rank)
            }
            TypeValue::Function {
                param_types: _,
                return_type,
            } => write!(f, "{}", return_type),
        }
    }
}

// -------------------------------------------------------------------------------------------- LValue Nodes -----------------------------------------------------------------------------------------------

pub struct LValue<'a> {
    pub position: Position,
    pub node: LValueType<'a>,
}

pub enum LValueType<'a> {
    Variable {
        name: &'a str,
    },
    Array {
        name: &'a str,
        indices: Vec<&'a str>,
    },
}

impl<'a> Display for LValue<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.node {
            LValueType::Variable { name } => write!(f, "(VarLValue {})", name),
            LValueType::Array { name, indices } => {
                let mut result = format!("(ArrayLValue {}", name);
                for idx in indices {
                    result.push_str(&format!(" {}", idx));
                }
                result.push(')');
                write!(f, "{}", result)
            }
        }
    }
}
