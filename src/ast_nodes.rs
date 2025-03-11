use crate::lexer::Position;
use std::fmt::Display;

pub enum ASTNode<'a> {
    Command(Command<'a>),
    Expression(Expression<'a>),
    Statement(Statement<'a>),
    LValue(LValue<'a>),
    Type(Type<'a>),
}

impl<'a> Display for ASTNode<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ASTNode::Command(c) => write!(f, "{}", c),
            ASTNode::Expression(e) => write!(f, "{}", e),
            ASTNode::Statement(s) => write!(f, "{}", s),
            ASTNode::LValue(l) => write!(f, "{}", l),
            ASTNode::Type(t) => write!(f, "{}", t),
        }
    }
}

pub trait ASTVisitor<'a> {
    fn visit_command(&mut self, command: &Command<'a>);
    fn visit_expression(&mut self, expression: &Expression<'a>);
    fn visit_statement(&mut self, statement: &Statement<'a>);
    fn visit_lvalue(&mut self, lvalue: &LValue<'a>);
    fn visit_type(&mut self, ty: &Type<'a>);
}

// -------------------------------------------------------------------------------------------- Command Nodes -----------------------------------------------------------------------------------------------

pub enum Command<'a> {
    Read {
        position: Position,
        source: &'a str,
        destination: Box<LValue<'a>>,
    },
    Write {
        position: Position,
        source: Box<Expression<'a>>,
        destination: &'a str,
    },
    Let {
        position: Position,
        variable: Box<LValue<'a>>,
        expression: Box<Expression<'a>>,
    },
    Assert {
        position: Position,
        expression: Box<Expression<'a>>,
        message: &'a str,
    },
    Print {
        position: Position,
        message: &'a str,
    },
    Show {
        position: Position,
        expression: Box<Expression<'a>>,
    },
    Time {
        position: Position,
        command: Box<Command<'a>>,
    },
    Function {
        position: Position,
        name: &'a str,
        parameters: Vec<(LValue<'a>, Box<Type<'a>>)>,
        return_type: Box<Type<'a>>,
        statements: Vec<Statement<'a>>,
        has_return: bool,
    },
    Struct {
        position: Position,
        name: &'a str,
        elements: Vec<(&'a str, Box<Type<'a>>)>,
    },
}

impl<'a> Display for Command<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Read {
                position,
                source,
                destination,
            } => write!(f, "Read {} into {}", source, destination),
            Command::Write {
                position,
                source,
                destination,
            } => write!(f, "Write {} to {}", source, destination),
            Command::Let {
                position,
                variable,
                expression,
            } => write!(f, "Let {} = {}", variable, expression),
            Command::Assert {
                position,
                expression,
                message,
            } => write!(f, "Assert {} with message {}", expression, message),
            Command::Print { position, message } => write!(f, "Print {}", message),
            Command::Show {
                position,
                expression,
            } => write!(f, "Show {}", expression),
            Command::Time { position, command } => write!(f, "Time {}", command),
            Command::Function {
                position,
                name,
                parameters,
                return_type,
                statements,
                has_return,
            } => write!(
                f,
                "Function {}({}) -> {} {{\n{}\n}}",
                name,
                parameters
                    .iter()
                    .map(|(l, t)| format!("{}: {}", l, t))
                    .collect::<Vec<String>>()
                    .join(", "),
                return_type,
                statements
                    .iter()
                    .map(|s| format!("{}", s))
                    .collect::<Vec<String>>()
                    .join("\n")
            ),
            Command::Struct {
                position,
                name,
                elements,
            } => write!(
                f,
                "Struct {} {{\n{}\n}}",
                name,
                elements
                    .iter()
                    .map(|(n, t)| format!("{}: {}", n, t))
                    .collect::<Vec<String>>()
                    .join("\n")
            ),
        }
    }
}

impl<'a> Command<'a> {
    pub fn accept<V: ASTVisitor<'a>>(&self, visitor: &mut V) {
        visitor.visit_command(self);
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

pub enum Expression<'a> {
    IntExpression {
        position: Position,
        value: i64,
    },
    FloatExpression {
        position: Position,
        value: f64,
    },
    TrueExpression {
        position: Position,
    },
    FalseExpression {
        position: Position,
    },
    VoidExpression {
        position: Position,
    },
    VariableExpression {
        position: Position,
        name: &'a str,
    },
    ArrayLiteralExpression {
        position: Position,
        elements: Vec<Expression<'a>>,
    },
    ArrayIndexExpression {
        position: Position,
        array: Box<Expression<'a>>,
        indices: Vec<Expression<'a>>,
    },
    DotExpression {
        position: Position,
        struct_variable: Box<Expression<'a>>,
        field: String,
    },
    CallExpression {
        position: Position,
        function: &'a str,
        arguments: Vec<Expression<'a>>,
    },
    StructLiteralExpression {
        position: Position,
        struct_name: &'a str,
        fields: Vec<Expression<'a>>,
    },
    UnopExpression {
        position: Position,
        operator: Unop,
        expression: Box<Expression<'a>>,
    },
    BinopExpression {
        position: Position,
        operator: Binop,
        left: Box<Expression<'a>>,
        right: Box<Expression<'a>>,
    },
    IfExpression {
        position: Position,
        condition: Box<Expression<'a>>,
        then_branch: Box<Expression<'a>>,
        else_branch: Box<Expression<'a>>,
    },
    ArrayLoopExpression {
        position: Position,
        range: Vec<(&'a str, Expression<'a>)>,
        body: Box<Expression<'a>>,
    },
    SumLoopExpression {
        position: Position,
        range: Box<Expression<'a>>,
        body: Box<Expression<'a>>,
    },
}

impl<'a> Display for Expression<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expression::IntExpression { position, value } => write!(f, "{}", value),
            Expression::FloatExpression { position, value } => write!(f, "{}", value),
            Expression::TrueExpression { position } => write!(f, "true"),
            Expression::FalseExpression { position } => write!(f, "false"),
            Expression::VoidExpression { position } => write!(f, "void"),
            Expression::VariableExpression { position, name } => write!(f, "{}", name),
            Expression::ArrayLiteralExpression { position, elements } => write!(
                f,
                "[{}]",
                elements
                    .iter()
                    .map(|e| format!("{}", e))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Expression::ArrayIndexExpression {
                position,
                array,
                indices,
            } => write!(
                f,
                "{}[{}]",
                array,
                indices
                    .iter()
                    .map(|i| format!("{}", i))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Expression::DotExpression {
                position,
                struct_variable,
                field,
            } => write!(f, "{}.{}", struct_variable, field),
            Expression::CallExpression {
                position,
                function,
                arguments,
            } => write!(
                f,
                "{}({})",
                function,
                arguments
                    .iter()
                    .map(|a| format!("{}", a))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Expression::StructLiteralExpression {
                position,
                struct_name,
                fields,
            } => write!(
                f,
                "{} {{\n{}\n}}",
                struct_name,
                fields
                    .iter()
                    .map(|e| format!("{}", e))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Expression::UnopExpression {
                position,
                operator,
                expression,
            } => write!(f, "{}{}", operator, expression),
            Expression::BinopExpression {
                position,
                operator,
                left,
                right,
            } => write!(f, "{} {} {}", left, operator, right),
            Expression::IfExpression {
                position,
                condition,
                then_branch,
                else_branch,
            } => write!(
                f,
                "if {} then {} else {}",
                condition, then_branch, else_branch
            ),
            Expression::ArrayLoopExpression {
                position,
                range,
                body,
            } => write!(
                f,
                "for {} in {}",
                body,
                range
                    .iter()
                    .map(|(n, e)| format!("{} in {}", n, e))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Expression::SumLoopExpression {
                position,
                range,
                body,
            } => write!(f, "sum {} {}", range, body),
        }
    }
}

impl<'a> Expression<'a> {
    pub fn accept<V: ASTVisitor<'a>>(&self, visitor: &mut V) {
        visitor.visit_expression(self);
    }
}

// -------------------------------------------------------------------------------------------- Statement Nodes -----------------------------------------------------------------------------------------------

pub enum Statement<'a> {
    LetStatement {
        position: Position,
        variable: Box<LValue<'a>>,
        value: Box<Expression<'a>>,
    },
    AssertStatement {
        position: Position,
        condition: Box<Expression<'a>>,
        message: &'a str,
    },
    ReturnStatement {
        position: Position,
        return_val: Box<Expression<'a>>,
    },
}

impl<'a> Display for Statement<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Statement::LetStatement {
                position,
                variable,
                value,
            } => write!(f, "Let {} = {}", variable, value),
            Statement::AssertStatement {
                position,
                condition,
                message,
            } => write!(f, "Assert {} with message {}", condition, message),
            Statement::ReturnStatement {
                position,
                return_val,
            } => write!(f, "Return {}", return_val),
        }
    }
}

impl<'a> Statement<'a> {
    pub fn accept<V: ASTVisitor<'a>>(&self, visitor: &mut V) {
        visitor.visit_statement(self);
    }
}

// -------------------------------------------------------------------------------------------- Type Nodes -----------------------------------------------------------------------------------------------

pub enum Type<'a> {
    IntType {
        position: Position,
        resolved: bool,
        value: i64,
    },
    FloatType {
        position: Position,
        resolved: bool,
        value: f64,
    },
    BoolType {
        position: Position,
        resolved: bool,
    },
    VoidType {
        position: Position,
        resolved: bool,
    },
    StructType {
        position: Position,
        resolved: bool,
        name: &'a str,
        value: Vec<(&'a str, Type<'a>)>,
    },
    ArrayType {
        position: Position,
        resolved: bool,
        element_type: Box<Type<'a>>,
        rank: usize,
    },
    FunctionType {
        position: Position,
        resolved: bool,
        param_types: Vec<Type<'a>>,
        return_type: Box<Type<'a>>,
    },
}

impl<'a> Display for Type<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::IntType {
                position,
                resolved,
                value,
            } => write!(f, "int"),
            Type::FloatType {
                position,
                resolved,
                value,
            } => write!(f, "float"),
            Type::BoolType { position, resolved } => write!(f, "bool"),
            Type::VoidType { position, resolved } => write!(f, "void"),
            Type::StructType {
                position,
                resolved,
                name,
                value,
            } => write!(f, "{}", name),
            Type::ArrayType {
                position,
                resolved,
                element_type,
                rank,
            } => write!(f, "{}[{}]", element_type, rank),
            Type::FunctionType {
                position,
                resolved,
                param_types,
                return_type,
            } => write!(
                f,
                "({}) -> {}",
                param_types
                    .iter()
                    .map(|t| format!("{}", t))
                    .collect::<Vec<String>>()
                    .join(", "),
                return_type
            ),
        }
    }
}

impl<'a> Type<'a> {
    pub fn accept<V: ASTVisitor<'a>>(&self, visitor: &mut V) {
        visitor.visit_type(self);
    }
}

// -------------------------------------------------------------------------------------------- LValue Nodes -----------------------------------------------------------------------------------------------

pub enum LValue<'a> {
    VariableLValue {
        name: &'a str,
    },
    ArrayLValue {
        name: &'a str,
        indices: Vec<&'a str>,
    },
}

impl<'a> Display for LValue<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LValue::VariableLValue { name } => write!(f, "{}", name),
            LValue::ArrayLValue { name, indices } => write!(
                f,
                "{}[{}]",
                name,
                indices
                    .iter()
                    .map(|i| format!("{}", i))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
        }
    }
}

impl<'a> LValue<'a> {
    pub fn accept<V: ASTVisitor<'a>>(&self, visitor: &mut V) {
        visitor.visit_lvalue(self);
    }
}
