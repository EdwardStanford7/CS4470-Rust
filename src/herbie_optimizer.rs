use std::process::{Command as ProcessCommand, Stdio};
use std::io::Write;
use crate::ast::*;
use crate::utils::Position;

// Check if an expression is viable for Herbie optimization
pub fn is_herbie_viable(expr: &Expression) -> bool {
    let is_float = matches!(expr.resolved_type, Type::Float { .. });
    let has_viable_structure = match &*expr.node {
        ExpressionType::Int { .. }
        | ExpressionType::Float { .. }
        | ExpressionType::Variable { .. } => true,

        ExpressionType::Binop { operator, left, right } => {
            matches!(*operator, "+" | "-" | "*" | "/")
                && is_herbie_viable(left)
                && is_herbie_viable(right)
        }

        ExpressionType::Unop { operator, expression } =>
            *operator == "-" && is_herbie_viable(expression),

        ExpressionType::Call { function, arguments } =>
            matches!(function.as_ref(),
                "sin" | "cos" | "tan" | "sqrt" | "exp" |
                "log" | "asin" | "acos" | "atan" | "pow"
            ) && arguments.iter().all(|arg| is_herbie_viable(arg)),

        _ => false,
    };
    is_float && has_viable_structure
}

// Extract variables from an expression
fn extract_variables<'a>(expr: &'a Expression<'a>) -> Vec<&'a str> {
    let mut vars = Vec::new();
    match &*expr.node {
        ExpressionType::Variable { name } => vars.push(*name),
        ExpressionType::Binop { left, right, .. } => {
            vars.extend(extract_variables(left));
            vars.extend(extract_variables(right));
        }
        ExpressionType::Unop { expression, .. } => vars.extend(extract_variables(expression)),
        ExpressionType::Call { arguments, .. } => {
            for arg in arguments {
                vars.extend(extract_variables(arg));
            }
        }
        _ => {}
    }
    vars.sort();
    vars.dedup();
    vars
}

// Convert an expression to FPCore
fn expr_to_fpcore(expr: &Expression) -> String {
    match &*expr.node {
        ExpressionType::Int { value } => value.to_string(),
        ExpressionType::Float { value } => value.to_string(),
        ExpressionType::Variable { name } => name.to_string(),

        ExpressionType::Binop { operator, left, right } => {
            let op = match operator.as_ref() {
                "+" | "-" | "*" | "/" => operator.as_ref(),
                "^" => "pow",
                other => other,
            };
            format!("({} {} {})", op, expr_to_fpcore(left), expr_to_fpcore(right))
        }

        ExpressionType::Unop { operator, expression } => {
            if *operator == "-" {
                format!("(- {})", expr_to_fpcore(expression))
            } else {
                format!("({} {})", AsRef::<str>::as_ref(operator), expr_to_fpcore(expression))
            }
        }

        ExpressionType::Call { function, arguments } => {
            let args: Vec<_> = arguments.iter().map(expr_to_fpcore).collect();
            format!("({} {})", AsRef::<str>::as_ref(function), args.join(" "))
        }

        _ => String::new(),
    }
}

// Wrap expression in FPCore header
fn create_fpcore(expr: &Expression) -> String {
    let vars = extract_variables(expr).join(" ");
    let body = expr_to_fpcore(expr);
    format!("(FPCore ({}) {})", vars, body)
}

// S-expression tokenizer
#[derive(Debug, PartialEq)]
enum Token {
    OpenParen,
    CloseParen,
    Symbol(String),
    Number(f64),
}

fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&c) = chars.peek() {
        match c {
            '(' => { tokens.push(Token::OpenParen); chars.next(); }
            ')' => { tokens.push(Token::CloseParen); chars.next(); }
            ' ' | '\t' | '\n' | '\r' => { chars.next(); }

            '-' | '+' | '0'..='9' | '.' => {
                let mut s = String::new();
                if c == '-' || c == '+' {
                    s.push(c); chars.next();
                    if let Some(&n) = chars.peek() {
                        if !n.is_digit(10) && n != '.' {
                            tokens.push(Token::Symbol(s)); continue;
                        }
                    }
                }
                while let Some(&n) = chars.peek() {
                    if n.is_digit(10) || n == '.' || n == 'e' || n == 'E' ||
                       (n == '-' && (s.ends_with('e') || s.ends_with('E'))) {
                        s.push(n); chars.next();
                    } else { break; }
                }
                if let Ok(v) = s.parse() { tokens.push(Token::Number(v)); }
                else { tokens.push(Token::Symbol(s)); }
            }

            _ => {
                let mut sym = String::new();
                while let Some(&n) = chars.peek() {
                    if n != '(' && n != ')' && !n.is_whitespace() {
                        sym.push(n); chars.next();
                    } else { break; }
                }
                tokens.push(Token::Symbol(sym));
            }
        }
    }
    tokens
}

// S-expression AST
#[derive(Debug)]
enum SExpr {
    Atom(String),
    Number(f64),
    List(Vec<SExpr>),
}

fn parse_tokens(tokens: &[Token]) -> Option<(SExpr, &[Token])> {
    if tokens.is_empty() { return None; }
    match &tokens[0] {
        Token::OpenParen => {
            let mut list = Vec::new();
            let mut rest = &tokens[1..];
            while !rest.is_empty() && rest[0] != Token::CloseParen {
                let (expr, next) = parse_tokens(rest)?;
                list.push(expr);
                rest = next;
            }
            if rest.is_empty() { return None; }
            Some((SExpr::List(list), &rest[1..]))
        }
        Token::CloseParen => None,
        Token::Symbol(s) => Some((SExpr::Atom(s.clone()), &tokens[1..])),
        Token::Number(n) => Some((SExpr::Number(*n), &tokens[1..])),
    }
}

fn make_static_str(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

fn sexp_to_ast<'a>(expr: &SExpr, pos: Position) -> Option<Expression<'a>> {
    Some(match expr {
        SExpr::Number(n) => Expression {
            position: pos,
            node: Box::new(ExpressionType::Float { value: *n }),
            resolved_type: Type::Float { value: Some(*n) },
        },
        SExpr::Atom(s) => Expression {
            position: pos,
            node: Box::new(ExpressionType::Variable { name: make_static_str(s) }),
            resolved_type: Type::Float { value: None },
        },
        SExpr::List(elems) => {
            let op = if let SExpr::Atom(o) = &elems[0] { o.as_str() } else { return None; };
            match op {
                "+" | "-" | "*" | "/" if elems.len() == 3 => {
                    let l = sexp_to_ast(&elems[1], pos)?;
                    let r = sexp_to_ast(&elems[2], pos)?;
                    Expression {
                        position: pos,
                        node: Box::new(ExpressionType::Binop {
                            operator: make_static_str(op),
                            left: l,
                            right: r,
                        }),
                        resolved_type: Type::Float { value: None },
                    }
                }
                "sin" | "cos" | "tan" | "sqrt" | "exp" | "log" |
                "asin" | "acos" | "atan" if elems.len() == 2 => {
                    let a = sexp_to_ast(&elems[1], pos)?;
                    Expression {
                        position: pos,
                        node: Box::new(ExpressionType::Call {
                            function: make_static_str(op),
                            arguments: vec![a],
                        }),
                        resolved_type: Type::Float { value: None },
                    }
                }
                _ => return None,
            }
        }
    })
}

fn parse_sexp<'a>(input: &str, position: Position) -> Option<Expression<'a>> {
    if let Ok(v) = input.parse::<f64>() {
        return Some(Expression {
            position,
            node: Box::new(ExpressionType::Float { value: v }),
            resolved_type: Type::Float { value: Some(v) },
        });
    }
    if input.chars().all(|c| c.is_alphabetic() || c == '_') {
        return Some(Expression {
            position,
            node: Box::new(ExpressionType::Variable { name: make_static_str(input) }),
            resolved_type: Type::Float { value: None },
        });
    }
    let tokens = tokenize(input);
    let (sexpr, _) = parse_tokens(&tokens)?;
    sexp_to_ast(&sexpr, position)
}

// Run Herbie to optimize the expression
fn run_herbie(fpcore: &str) -> Option<String> {
    if fpcore.contains("(FPCore () ") { return None; }
    let mut child = ProcessCommand::new("racket")
        .arg("-l").arg("herbie").arg("shell")
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().ok()?;

    { let stdin = child.stdin.as_mut()?;
      stdin.write_all(fpcore.as_bytes()).ok()?;
      stdin.write_all(b"\nexit\n").ok()?; }

    let output = child.wait_with_output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines()
        .map(str::trim)
        .filter(|l| l.starts_with('(') && !l.contains("FPCore") && !l.contains("herbie>"))
        .last()
        .map(String::from)
}

// Apply Herbie optimization to a Let command
pub fn apply_herbie_optimization(command: &mut Command) {
    if let CommandType::Let { rvalue, .. } = &mut *command.node {
        if is_herbie_viable(rvalue) {
            let fpcore = create_fpcore(rvalue);
            if let Some(opt) = run_herbie(&fpcore) {
                if let Some(new_expr) = parse_sexp(&opt, rvalue.position) {
                    *rvalue = new_expr;
                }
            }
        }
    }
}
