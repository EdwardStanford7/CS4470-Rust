use std::process::{Command as ProcessCommand, Stdio};
use std::io::Write;
use crate::ast::*;
use crate::utils::Position;
use sexp::{parse, Atom, Sexp};

pub fn is_herbie_viable(expr: &Expression) -> bool {
    let is_float = matches!(expr.resolved_type, Type::Float { .. });

    let has_structure = match &*expr.node {
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

        ExpressionType::Call { function, arguments } => {
            let func_ok = matches!(function.as_ref(),
                "sin"|"cos"|"tan"|"sqrt"|"exp"|"log"|"asin"|"acos"|"atan"|"pow");
            func_ok && arguments.iter().all(|arg| is_herbie_viable(arg))
        }

        _ => false,
    };

    is_float && has_structure
}

fn extract_variables<'a>(expr: &'a Expression<'a>) -> Vec<&'a str> {
    let mut vars = Vec::new();
    fn collect<'a>(e: &'a Expression<'a>, out: &mut Vec<&'a str>) {
        match &*e.node {
            ExpressionType::Variable { name } => out.push(*name),
            ExpressionType::Binop { left, right, .. } => {
                collect(left, out);
                collect(right, out);
            }
            ExpressionType::Unop { expression, .. } => collect(expression, out),
            ExpressionType::Call { arguments, .. } => {
                for arg in arguments {
                    collect(arg, out);
                }
            }
            _ => {}
        }
    }
    collect(expr, &mut vars);
    vars.sort();
    vars.dedup();
    vars
}

fn create_fpcore(expr: &Expression) -> String {
    fn to_core(e: &Expression) -> String {
        match &*e.node {
            ExpressionType::Int { value } => value.to_string(),
            ExpressionType::Float { value } => value.to_string(),
            ExpressionType::Variable { name } => name.to_string(),
            ExpressionType::Binop { operator, left, right } => {
                let op = match operator.as_ref() {
                    "+" | "-" | "*" | "/" => operator.as_ref(),
                    "^" => "pow",
                    other => other,
                };
                format!("({} {} {})", op, to_core(left), to_core(right))
            }
            ExpressionType::Unop { operator, expression } =>
                format!("({} {})", operator, to_core(expression)),
            ExpressionType::Call { function, arguments } => {
                let args = arguments.iter().map(to_core).collect::<Vec<_>>().join(" ");
                format!("({} {})", function, args)
            }
            _ => String::new(),
        }
    }

    let vars = extract_variables(expr).join(" ");
    let body = to_core(expr);
    format!("(FPCore ({}) {})", vars, body)
}

fn parse_sexp<'a>(input: &str, pos: Position) -> Option<Expression<'a>> {
    if let Ok(v) = input.parse::<f64>() {
        return Some(Expression {
            position: pos,
            node: Box::new(ExpressionType::Float { value: v }),
            resolved_type: Type::Float { value: Some(v) },
        });
    }
    if input.chars().all(|c| c.is_alphanumeric() || c == '_') {
        let name = Box::leak(input.to_string().into_boxed_str());
        return Some(Expression {
            position: pos,
            node: Box::new(ExpressionType::Variable { name }),
            resolved_type: Type::Float { value: None },
        });
    }
    let sexp = parse(input).ok()?;
    sexp_to_ast(&sexp, pos)
}

fn sexp_to_ast<'a>(sexp: &Sexp, pos: Position) -> Option<Expression<'a>> {
    match sexp {
        Sexp::Atom(Atom::I(i)) => {
            let v = *i as f64;
            Some(Expression {
                position: pos,
                node: Box::new(ExpressionType::Float { value: v }),
                resolved_type: Type::Float { value: Some(v) },
            })
        }
        Sexp::Atom(Atom::F(f)) => {
            Some(Expression {
                position: pos,
                node: Box::new(ExpressionType::Float { value: *f }),
                resolved_type: Type::Float { value: Some(*f) },
            })
        }
        Sexp::Atom(Atom::S(s)) => {
            let name = Box::leak(s.clone().into_boxed_str());
            Some(Expression {
                position: pos,
                node: Box::new(ExpressionType::Variable { name }),
                resolved_type: Type::Float { value: None },
            })
        }
        Sexp::List(list) => {
            if list.is_empty() { return None }
            let op = match &list[0] {
                Sexp::Atom(Atom::S(sym)) => sym.as_str(),
                _ => return None,
            };
            match (op, list.len()) {
                ("+"|"-"|"*"|"/", 3) => {
                    let l = sexp_to_ast(&list[1], pos)?;
                    let r = sexp_to_ast(&list[2], pos)?;
                    Some(Expression {
                        position: pos,
                        node: Box::new(ExpressionType::Binop {
                            operator: Box::leak(op.to_string().into_boxed_str()),
                            left: l,
                            right: r,
                        }),
                        resolved_type: Type::Float { value: None },
                    })
                }
                ("-", 2) => {
                    let inner = sexp_to_ast(&list[1], pos)?;
                    Some(Expression {
                        position: pos,
                        node: Box::new(ExpressionType::Unop {
                            operator: Box::leak("-".to_string().into_boxed_str()),
                            expression: inner,
                        }),
                        resolved_type: Type::Float { value: None },
                    })
                }
                ("sin"|"cos"|"tan"|"sqrt"|"exp"|"log"|"asin"|"acos"|"atan"|"pow", 2) => {
                    let arg = sexp_to_ast(&list[1], pos)?;
                    Some(Expression {
                        position: pos,
                        node: Box::new(ExpressionType::Call {
                            function: Box::leak(op.to_string().into_boxed_str()),
                            arguments: vec![arg],
                        }),
                        resolved_type: Type::Float { value: None },
                    })
                }
                ("fma", 4) => {
                    let a = sexp_to_ast(&list[1], pos)?;
                    let b = sexp_to_ast(&list[2], pos)?;
                    let c = sexp_to_ast(&list[3], pos)?;
                    let mul = Expression {
                        position: pos,
                        node: Box::new(ExpressionType::Binop {
                            operator: Box::leak("*".to_string().into_boxed_str()),
                            left: a,
                            right: b,
                        }),
                        resolved_type: Type::Float { value: None },
                    };
                    Some(Expression {
                        position: pos,
                        node: Box::new(ExpressionType::Binop {
                            operator: Box::leak("+".to_string().into_boxed_str()),
                            left: mul,
                            right: c,
                        }),
                        resolved_type: Type::Float { value: None },
                    })
                }
                _ => None,
            }
        }
    }
}

fn run_herbie(fpcore: &str) -> Option<String> {
    let mut child = ProcessCommand::new("racket")
        .arg("-l").arg("herbie").arg("shell")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    {
        let stdin = child.stdin.as_mut()?;
        stdin.write_all(fpcore.as_bytes()).ok()?;
        stdin.write_all(b"\nexit\n").ok()?;
    }

    let output = child.wait_with_output().ok()?;
    let binding = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = binding
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with('(') && !l.contains("FPCore") && !l.contains("herbie>"))
        .collect();

    let mut cand = lines.last()?.to_string();

    loop {
        let opens = cand.matches('(').count();
        let closes = cand.matches(')').count();
        if opens == closes { break; }
        if cand.ends_with(')') {
            cand.pop();
        } else {
            break;
        }
    }

    Some(cand)
}

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