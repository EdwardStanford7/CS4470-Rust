use std::process::{Command as ProcessCommand, Stdio};
use std::io::Write;
use crate::ast::*;
use crate::utils::Position;
use sexp::{parse, Atom, Sexp};

/// Very verbose debug wrapper for Herbie integration
pub fn is_herbie_viable(expr: &Expression) -> bool {
    println!("[VIABLE] checking expr: {:?}", expr);
    let is_float = matches!(expr.resolved_type, Type::Float { .. });
    println!("[VIABLE] is_float = {}", is_float);

    let has_structure = match &*expr.node {
        ExpressionType::Int { .. } => { println!("[VIABLE] Int node → true"); true }
        ExpressionType::Float { .. } => { println!("[VIABLE] Float node → true"); true }
        ExpressionType::Variable { .. } => { println!("[VIABLE] Var node → true"); true }

        ExpressionType::Binop { operator, left, right } => {
            println!("[VIABLE] Binop '{}' → checking children", operator);
            let op_ok = matches!(*operator, "+"|"-"|"*"|"/");
            let left_ok = is_herbie_viable(left);
            let right_ok = is_herbie_viable(right);
            println!("[VIABLE] op_ok={}, left_ok={}, right_ok={}", op_ok, left_ok, right_ok);
            op_ok && left_ok && right_ok
        }

        ExpressionType::Unop { operator, expression } => {
            println!("[VIABLE] Unop '{}' → checking child", operator);
            let op_ok = *operator == "-";
            let child_ok = is_herbie_viable(expression);
            println!("[VIABLE] op_ok={}, child_ok={}", op_ok, child_ok);
            op_ok && child_ok
        }

        ExpressionType::Call { function, arguments } => {
            println!("[VIABLE] Call '{}' with {} args", function, arguments.len());
            let func_ok = matches!(function.as_ref(),
                "sin"|"cos"|"tan"|"sqrt"|"exp"|"log"|"asin"|"acos"|"atan"|"pow");
            let mut args_ok = true;
            for (i, arg) in arguments.iter().enumerate() {
                let ok = is_herbie_viable(arg);
                println!("[VIABLE]  arg[{}] ok={}", i, ok);
                args_ok &= ok;
            }
            println!("[VIABLE] func_ok={}, args_ok={}", func_ok, args_ok);
            func_ok && args_ok
        }

        _ => {
            println!("[VIABLE] Node type not supported → false");
            false
        }
    };

    let result = is_float && has_structure;
    println!("[VIABLE] FINAL = {}", result);
    result
}

fn extract_variables<'a>(expr: &'a Expression<'a>) -> Vec<&'a str> {
    let mut vars = Vec::new();
    fn collect<'a>(e: &'a Expression<'a>, out: &mut Vec<&'a str>) {
        match &*e.node {
            ExpressionType::Variable { name } => out.push(*name),
            ExpressionType::Binop { left, right, .. } => { collect(left, out); collect(right, out); }
            ExpressionType::Unop { expression, .. } => collect(expression, out),
            ExpressionType::Call { arguments, .. } => arguments.iter().for_each(|a| collect(a, out)),
            _ => {}
        }
    }
    collect(expr, &mut vars);
    vars.sort(); vars.dedup();
    println!("[VARS] extracted = {:?}", vars);
    vars
}

fn create_fpcore(expr: &Expression) -> String {
    let vars = extract_variables(expr).join(" ");
    let body = {
        fn to_core(e: &Expression) -> String {
            match &*e.node {
                ExpressionType::Int { value } => value.to_string(),
                ExpressionType::Float { value } => value.to_string(),
                ExpressionType::Variable { name } => name.to_string(),
                ExpressionType::Binop { operator, left, right } => {
                    let op = match operator.as_ref() {
                        "+"|"-"|"*"|"/" => operator.as_ref(), "^" => "pow", other=>other
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
        to_core(expr)
    };
    let fpcore = format!("(FPCore ({}) {})", vars, body);
    println!("[FPCORE] header+body = {}", fpcore);
    fpcore
}

fn parse_sexp<'a>(input: &str, pos: Position) -> Option<Expression<'a>> {
    println!("[PARSE] input = '{}'", input);
    if let Ok(v) = input.parse::<f64>() {
        println!("[PARSE] → Float({})", v);
        return Some(Expression {
            position: pos,
            node: Box::new(ExpressionType::Float { value: v }),
            resolved_type: Type::Float { value: Some(v) },
        });
    }
    if input.chars().all(|c| c.is_alphanumeric() || c=='_') {
        let name = Box::leak(input.to_string().into_boxed_str());
        println!("[PARSE] → Var({})", name);
        return Some(Expression {
            position: pos,
            node: Box::new(ExpressionType::Variable { name }),
            resolved_type: Type::Float { value: None },
        });
    }
    let sexp = match parse(input) {
        Ok(s) => { println!("[PARSE] Sexp = {:?}", s); s }
        Err(e) => { println!("[PARSE] parse error: {}", e); return None }
    };
    sexp_to_ast(&sexp, pos)
}

fn sexp_to_ast<'a>(sexp: &Sexp, pos: Position) -> Option<Expression<'a>> {
    println!("[AST] processing Sexp = {:?}", sexp);
    match sexp {
        Sexp::Atom(Atom::I(i)) => {
            let v = *i as f64;
            println!("[AST] Atom::I → Float({})", v);
            Some(Expression {
                position: pos,
                node: Box::new(ExpressionType::Float { value: v }),
                resolved_type: Type::Float { value: Some(v) },
            })
        }
        Sexp::Atom(Atom::F(f)) => {
            println!("[AST] Atom::F → Float({})", f);
            Some(Expression {
                position: pos,
                node: Box::new(ExpressionType::Float { value: *f }),
                resolved_type: Type::Float { value: Some(*f) },
            })
        }
        Sexp::Atom(Atom::S(s)) => {
            println!("[AST] Atom::S → Var({})", s);
            let name = Box::leak(s.clone().into_boxed_str());
            Some(Expression {
                position: pos,
                node: Box::new(ExpressionType::Variable { name }),
                resolved_type: Type::Float { value: None },
            })
        }
        Sexp::List(list) => {
            println!("[AST] List(len={})", list.len());
            if list.is_empty() { return None }
            let op = match &list[0] {
                Sexp::Atom(Atom::S(sym)) => sym.as_str(),
                _ => { println!("[AST] head not symbol"); return None }
            };
            match (op, list.len()) {
                ("+"|"-"|"*"|"/", 3) => {
                    println!("[AST] Binop '{}'", op);
                    let l = sexp_to_ast(&list[1], pos)?;
                    let r = sexp_to_ast(&list[2], pos)?;
                    Some(Expression {
                        position: pos,
                        node: Box::new(ExpressionType::Binop {
                            operator: Box::leak(op.to_string().into_boxed_str()),
                            left: l, right: r
                        }),
                        resolved_type: Type::Float { value: None },
                    })
                }
                ("-", 2) => {
                    println!("[AST] Unop '-'");
                    let inner = sexp_to_ast(&list[1], pos)?;
                    Some(Expression {
                        position: pos,
                        node: Box::new(ExpressionType::Unop {
                            operator: Box::leak("-".to_string().into_boxed_str()),
                            expression: inner
                        }),
                        resolved_type: Type::Float { value: None },
                    })
                }
                ("sin"|"cos"|"tan"|"sqrt"|"exp"|"log"|"asin"|"acos"|"atan"|"pow", 2) => {
                    println!("[AST] Call '{}'", op);
                    let arg = sexp_to_ast(&list[1], pos)?;
                    Some(Expression {
                        position: pos,
                        node: Box::new(ExpressionType::Call {
                            function: Box::leak(op.to_string().into_boxed_str()),
                            arguments: vec![arg]
                        }),
                        resolved_type: Type::Float { value: None },
                    })
                }
                _ => {
                    println!("[AST] unhandled '{}'/arity {}", op, list.len());
                    None
                }
            }
        }
    }
}
fn run_herbie(fpcore: &str) -> Option<String> {
    // 1) launch Herbie
    let mut child = ProcessCommand::new("racket")
        .arg("-l").arg("herbie").arg("shell")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn herbie");

    // 2) feed it the FPCore and exit
    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(fpcore.as_bytes()).unwrap();
        stdin.write_all(b"\nexit\n").unwrap();
    }

    // 3) collect stdout lines
    let output = child.wait_with_output().unwrap();
    let binding = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = binding
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with('(') && !l.contains("FPCore") && !l.contains("herbie>"))
        .collect();

    // 4) pick the last candidate (whatever it is)
    let mut cand = lines.last()?.to_string();

    // 5) trim trailing ')' until opens == closes
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
        println!("[APPLY] before rvalue={:?}", rvalue);
        if is_herbie_viable(rvalue) {
            println!("[APPLY] viable!");
            let fpcore = create_fpcore(rvalue);
            if let Some(opt) = run_herbie(&fpcore) {
                println!("[APPLY] raw opt='{}'", opt);
                if let Some(new_expr) = parse_sexp(&opt, rvalue.position) {
                    println!("[APPLY] parsed new_expr={:?}", new_expr);
                    *rvalue = new_expr;
                    println!("[APPLY] after rvalue={:?}", rvalue);
                } else {
                    println!("[APPLY] parse_sexp failed");
                }
            } else {
                println!("[APPLY] run_herbie returned None");
            }
        } else {
            println!("[APPLY] not viable");
        }
    } else {
        println!("[APPLY] not a Let command");
    }
}