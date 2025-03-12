use crate::lexer::Position;
use core::str;
use std::fmt::Display;

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
        parameters: Vec<(LValue<'a>, Box<Type<'a>>)>,
        return_type: Box<Type<'a>>,
        statements: Vec<Statement<'a>>,
        has_return: bool,
    },
    Struct {
        name: &'a str,
        elements: Vec<(&'a str, Box<Type<'a>>)>,
    },
}

impl<'a> Display for Command<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.node {
            CommandType::Read {
                source,
                destination,
            } => write!(f, "(ReadCmd {} {})", source, destination),
            CommandType::Write {
                source,
                destination,
            } => {
                todo!()
            }
            CommandType::Let { variable, rvalue } => {
                todo!()
            }
            CommandType::Assert { condition, message } => {
                todo!()
            }
            CommandType::Print { message } => {
                todo!()
            }

            CommandType::Show { expression } => {
                todo!()
            }
            CommandType::Time { command } => {
                todo!()
            }
            CommandType::Function {
                name,
                parameters,
                return_type,
                statements,
                has_return,
            } => {
                todo!()
            }
            CommandType::Struct { name, elements } => {
                todo!()
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
        todo!();
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
        todo!();
    }
}

// -------------------------------------------------------------------------------------------- Type Nodes -----------------------------------------------------------------------------------------------

pub enum Type<'a> {
    IntType {
        resolved: bool,
        value: i64,
    },
    FloatType {
        resolved: bool,
        value: f64,
    },
    BoolType {
        resolved: bool,
    },
    VoidType {
        resolved: bool,
    },
    StructType {
        resolved: bool,
        name: &'a str,
        value: Vec<(&'a str, Type<'a>)>,
    },
    ArrayType {
        resolved: bool,
        element_type: Box<Type<'a>>,
        rank: usize,
    },
    FunctionType {
        resolved: bool,
        param_types: Vec<Type<'a>>,
        return_type: Box<Type<'a>>,
    },
}

impl<'a> Display for Type<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!();
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
                todo!();
            }
        }
    }
}
