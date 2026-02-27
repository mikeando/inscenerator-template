use crate::expr::{BinOp, Expr};
use crate::parser::{Node, Template};
use crate::value::{DataSource, Value};

/// Render a template against a data source, returning the output string.
pub fn render(template: &Template, ctx: &dyn DataSource) -> Result<String, String> {
    let mut out = String::new();
    render_nodes(&template.nodes, ctx, template, &mut out, 0)?;
    Ok(out)
}

fn render_nodes(
    nodes: &[Node],
    ctx: &dyn DataSource,
    template: &Template,
    out: &mut String,
    recursion_depth: usize,
) -> Result<(), String> {
    for node in nodes {
        render_node(node, ctx, template, out, recursion_depth)?;
    }
    Ok(())
}

fn render_node(
    node: &Node,
    ctx: &dyn DataSource,
    template: &Template,
    out: &mut String,
    recursion_depth: usize,
) -> Result<(), String> {
    if recursion_depth > 20 {
        return Err(
            "Maximum recursion depth exceeded (possible infinite fragment recursion)".to_string(),
        );
    }
    match node {
        Node::Text(s) => out.push_str(s),

        Node::Output(expr) => {
            let val = eval(expr, ctx, template)?;
            out.push_str(&val.render());
        }

        Node::If {
            branches,
            else_body,
        } => {
            let mut rendered = false;
            for (cond, body) in branches {
                let cond_val = eval(cond, ctx, template)?;
                match &cond_val {
                    Value::Bool(b) => {
                        if *b {
                            render_nodes(body, ctx, template, out, recursion_depth + 1)?;
                            rendered = true;
                            break;
                        }
                    }
                    _ => {
                        return Err(format!(
                            "Conditional expression `{}` must evaluate to a Bool, got `{}`",
                            cond,
                            cond_val.render()
                        ));
                    }
                }
            }
            if !rendered && let Some(body) = else_body {
                render_nodes(body, ctx, template, out, recursion_depth + 1)?;
            }
        }

        Node::For { item, list, body } => {
            let list_val = eval(list, ctx, template)?;
            let items = match list_val {
                Value::List(l) => l,
                other => {
                    return Err(format!(
                        "For loop `{}` must be a List, got `{}`",
                        list,
                        other.render()
                    ));
                }
            };

            for val in items.iter() {
                let child = LoopContext {
                    item_name: item.as_str(),
                    item_val: val.clone(),
                    parent: ctx,
                };
                render_nodes(body, &child, template, out, recursion_depth + 1)?;
            }
        }

        Node::Fragment { name, ctx_expr } => {
            let ctx_val = eval(ctx_expr, ctx, template)?;
            let frag_ctx: &dyn DataSource = match &ctx_val {
                Value::Map(map) => map,
                Value::DataSource(src) => src.as_ref(),
                _ => {
                    return Err(format!(
                        "Fragment `{}`: context must be a Map, got `{}`",
                        name,
                        ctx_val.render()
                    ));
                }
            };

            match template.fragments.get(name) {
                Some(nodes) => render_nodes(nodes, frag_ctx, template, out, recursion_depth + 1)?,
                None => return Err(format!("Unknown fragment: `{}`", name)),
            }
        }
    }
    Ok(())
}

/// Evaluate an `Expr` against a context and template, returning a `Value`.
fn eval(expr: &Expr, ctx: &dyn DataSource, template: &Template) -> Result<Value, String> {
    match expr {
        Expr::Literal(v) => Ok(v.clone()),

        Expr::Path(p) => {
            let root_key = p.split('.').next().unwrap();
            let root_val = ctx
                .get(root_key)
                .ok_or_else(|| format!("Variable not found: `{}` ", p))?;

            if p.contains('.') {
                root_val
                    .get_path(&p[root_key.len() + 1..])
                    .ok_or_else(|| format!("Variable not found: `{}` ", p))
            } else {
                Ok(root_val)
            }
        }

        Expr::Not(inner) => {
            let val = eval(inner, ctx, template)?;
            match val {
                Value::Bool(b) => Ok(Value::Bool(!b)),
                other => Err(format!(
                    "Negation operator `!` requires a Bool operand, got `{}`",
                    other.render()
                )),
            }
        }

        Expr::BinOp { op, lhs, rhs } => {
            let l = eval(lhs, ctx, template)?;
            let r = eval(rhs, ctx, template)?;
            apply_binop(op, l, r)
        }

        Expr::Call { name, args } => {
            let vals: Vec<Value> = args
                .iter()
                .map(|a| eval(a, ctx, template))
                .collect::<Result<_, _>>()?;
            match template.functions.get(name) {
                Some(f) => f.call(&vals),
                None => Err(format!("Unknown function: `{}`", name)),
            }
        }
    }
}

fn apply_binop(op: &BinOp, lhs: Value, rhs: Value) -> Result<Value, String> {
    match op {
        BinOp::Eq | BinOp::Ne => {
            let equal = match (&lhs, &rhs) {
                (Value::Null, Value::Null) => true,
                (Value::Bool(a), Value::Bool(b)) => a == b,
                (Value::Str(a), Value::Str(b)) => a == b,
                (Value::Int(a), Value::Int(b)) => a == b,
                (Value::Float(a), Value::Float(b)) => a == b,
                (Value::Int(a), Value::Float(b)) => (*a as f64) == *b,
                (Value::Float(a), Value::Int(b)) => *a == (*b as f64),
                _ => {
                    return Err(format!(
                        "Cannot compare `{}` and `{}`",
                        lhs.render(),
                        rhs.render()
                    ));
                }
            };
            Ok(Value::Bool(if matches!(op, BinOp::Eq) {
                equal
            } else {
                !equal
            }))
        }

        BinOp::And | BinOp::Or => match (&lhs, &rhs) {
            (Value::Bool(a), Value::Bool(b)) => {
                let result = if matches!(op, BinOp::And) {
                    *a && *b
                } else {
                    *a || *b
                };
                Ok(Value::Bool(result))
            }
            _ => Err(format!(
                "`{}` operator requires Bool operands, got `{}` and `{}`",
                op,
                lhs.render(),
                rhs.render()
            )),
        },

        BinOp::Add => match (&lhs, &rhs) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + *b as f64)),
            (Value::Str(a), Value::Str(b)) => Ok(Value::Str(format!("{}{}", a, b))),
            _ => Err(format!(
                "`+` operator: incompatible types `{}` and `{}`",
                lhs.render(),
                rhs.render()
            )),
        },

        BinOp::Sub => match (&lhs, &rhs) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a - *b as f64)),
            _ => Err(format!(
                "`-` operator requires numeric operands, got `{}` and `{}`",
                lhs.render(),
                rhs.render()
            )),
        },

        BinOp::Mul => match (&lhs, &rhs) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a * *b as f64)),
            _ => Err(format!(
                "`*` operator requires numeric operands, got `{}` and `{}`",
                lhs.render(),
                rhs.render()
            )),
        },

        BinOp::Div => match (&lhs, &rhs) {
            (Value::Int(_), Value::Int(0)) => Err("Division by zero".to_string()),
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a / b)),
            (Value::Float(a), Value::Float(b)) => {
                if *b == 0.0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(Value::Float(a / b))
                }
            }
            (Value::Int(a), Value::Float(b)) => {
                if *b == 0.0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(Value::Float(*a as f64 / b))
                }
            }
            (Value::Float(_), Value::Int(0)) => Err("Division by zero".to_string()),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a / *b as f64)),
            _ => Err(format!(
                "`/` operator requires numeric operands, got `{}` and `{}`",
                lhs.render(),
                rhs.render()
            )),
        },

        BinOp::Mod => match (&lhs, &rhs) {
            (Value::Int(_), Value::Int(0)) => Err("Modulo by zero".to_string()),
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a % b)),
            _ => Err(format!(
                "`%` operator requires integer operands, got `{}` and `{}`",
                lhs.render(),
                rhs.render()
            )),
        },

        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
            let cmp = match (&lhs, &rhs) {
                (Value::Int(a), Value::Int(b)) => a.cmp(b),
                (Value::Float(a), Value::Float(b)) => a
                    .partial_cmp(b)
                    .ok_or_else(|| "Cannot compare NaN values".to_string())?,
                (Value::Int(a), Value::Float(b)) => (*a as f64)
                    .partial_cmp(b)
                    .ok_or_else(|| "Cannot compare NaN values".to_string())?,
                (Value::Float(a), Value::Int(b)) => a
                    .partial_cmp(&(*b as f64))
                    .ok_or_else(|| "Cannot compare NaN values".to_string())?,
                _ => {
                    return Err(format!(
                        "Ordering operators require numeric operands, got `{}` and `{}`",
                        lhs.render(),
                        rhs.render()
                    ));
                }
            };
            let result = match op {
                BinOp::Lt => cmp.is_lt(),
                BinOp::Le => cmp.is_le(),
                BinOp::Gt => cmp.is_gt(),
                BinOp::Ge => cmp.is_ge(),
                _ => unreachable!(),
            };
            Ok(Value::Bool(result))
        }
    }
}

/// A context that overlays one variable on top of a parent context.
/// Used to introduce the loop variable inside {% for %} blocks.
#[derive(Debug)]
struct LoopContext<'a> {
    item_name: &'a str,
    item_val: Value,
    parent: &'a dyn DataSource,
}

impl<'a> DataSource for LoopContext<'a> {
    fn get(&self, key: &str) -> Option<Value> {
        if key == self.item_name {
            Some(self.item_val.clone())
        } else {
            self.parent.get(key)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ctx;
    use crate::expr::parse_expr;

    fn make_tmpl() -> Template {
        Template::parse("").unwrap()
    }

    fn ev(s: &str, ctx: &dyn DataSource) -> Result<Value, String> {
        let expr = parse_expr(s).unwrap();
        eval(&expr, ctx, &make_tmpl())
    }

    #[test]
    fn test_eval_literals() -> Result<(), String> {
        let ctx = ctx! {};
        assert_eq!(ev("true", ctx.as_ref())?.render(), "true");
        assert_eq!(ev("false", ctx.as_ref())?.render(), "false");
        assert_eq!(ev("null", ctx.as_ref())?.render(), "");
        assert_eq!(ev("123", ctx.as_ref())?.render(), "123");
        assert_eq!(ev("1.23", ctx.as_ref())?.render(), "1.23");
        assert_eq!(ev("\"hello\"", ctx.as_ref())?.render(), "hello");
        assert_eq!(ev("'world'", ctx.as_ref())?.render(), "world");
        Ok(())
    }

    #[test]
    fn test_eval_negation() -> Result<(), String> {
        let ctx = ctx! { "flag": true };
        assert_eq!(ev("!flag", ctx.as_ref())?.render(), "false");
        assert_eq!(ev("!!flag", ctx.as_ref())?.render(), "true");
        assert_eq!(ev("!false", ctx.as_ref())?.render(), "true");
        Ok(())
    }

    #[test]
    fn test_eval_dotted_path() -> Result<(), String> {
        let ctx = ctx! { "inner": { "a": 1 } };
        assert_eq!(ev("inner.a", ctx.as_ref())?.render(), "1");
        assert!(ev("inner.b", ctx.as_ref()).is_err());
        Ok(())
    }

    // --- apply_binop tests ---

    #[test]
    fn test_binop_eq_int() {
        assert_eq!(
            apply_binop(&BinOp::Eq, Value::Int(5), Value::Int(5)).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            apply_binop(&BinOp::Eq, Value::Int(5), Value::Int(6)).unwrap(),
            Value::Bool(false)
        );
    }

    #[test]
    fn test_binop_eq_float_int_coerce() {
        assert_eq!(
            apply_binop(&BinOp::Eq, Value::Int(3), Value::Float(3.0)).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            apply_binop(&BinOp::Eq, Value::Float(2.5), Value::Int(2)).unwrap(),
            Value::Bool(false)
        );
    }

    #[test]
    fn test_binop_eq_str() {
        assert_eq!(
            apply_binop(&BinOp::Eq, Value::from("hello"), Value::from("hello")).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            apply_binop(&BinOp::Eq, Value::from("hello"), Value::from("world")).unwrap(),
            Value::Bool(false)
        );
    }

    #[test]
    fn test_binop_eq_bool() {
        assert_eq!(
            apply_binop(&BinOp::Eq, Value::Bool(true), Value::Bool(true)).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            apply_binop(&BinOp::Eq, Value::Bool(true), Value::Bool(false)).unwrap(),
            Value::Bool(false)
        );
    }

    #[test]
    fn test_binop_eq_null() {
        assert_eq!(
            apply_binop(&BinOp::Eq, Value::Null, Value::Null).unwrap(),
            Value::Bool(true)
        );
    }

    #[test]
    fn test_binop_ne() {
        assert_eq!(
            apply_binop(&BinOp::Ne, Value::Int(1), Value::Int(2)).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            apply_binop(&BinOp::Ne, Value::Int(1), Value::Int(1)).unwrap(),
            Value::Bool(false)
        );
    }

    #[test]
    fn test_binop_eq_mixed_type_error() {
        assert!(apply_binop(&BinOp::Eq, Value::Bool(true), Value::Int(1)).is_err());
        assert!(apply_binop(&BinOp::Eq, Value::from("1"), Value::Int(1)).is_err());
    }

    #[test]
    fn test_binop_lt_int() {
        assert_eq!(
            apply_binop(&BinOp::Lt, Value::Int(3), Value::Int(5)).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            apply_binop(&BinOp::Lt, Value::Int(5), Value::Int(3)).unwrap(),
            Value::Bool(false)
        );
    }

    #[test]
    fn test_binop_le_ge() {
        assert_eq!(
            apply_binop(&BinOp::Le, Value::Int(5), Value::Int(5)).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            apply_binop(&BinOp::Ge, Value::Int(5), Value::Int(5)).unwrap(),
            Value::Bool(true)
        );
    }

    #[test]
    fn test_binop_ordering_float_int_coerce() {
        assert_eq!(
            apply_binop(&BinOp::Lt, Value::Int(3), Value::Float(3.5)).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            apply_binop(&BinOp::Gt, Value::Float(4.0), Value::Int(3)).unwrap(),
            Value::Bool(true)
        );
    }

    #[test]
    fn test_binop_ordering_str_error() {
        assert!(apply_binop(&BinOp::Lt, Value::from("a"), Value::from("b")).is_err());
    }

    #[test]
    fn test_binop_ordering_bool_error() {
        assert!(apply_binop(&BinOp::Lt, Value::Bool(true), Value::Bool(false)).is_err());
    }

    #[test]
    fn test_eval_binop_end_to_end() -> Result<(), String> {
        let ctx = ctx! { "age": 25 };
        assert_eq!(ev("age >= 18", ctx.as_ref())?.render(), "true");
        assert_eq!(ev("age < 18", ctx.as_ref())?.render(), "false");
        Ok(())
    }

    #[test]
    fn test_eval_unknown_function() {
        let ctx = ctx! {};
        assert!(ev("unknown_fn()", ctx.as_ref()).is_err());
    }

    #[test]
    fn test_render_if() {
        let tmpl = Template::parse("{% if a %}A{% elif b %}B{% else %}C{% endif %}").unwrap();

        let ctx_a = ctx! { "a": true };
        assert_eq!(render(&tmpl, ctx_a.as_ref()).unwrap(), "A");

        let ctx_b = ctx! { "a": false, "b": true };
        assert_eq!(render(&tmpl, ctx_b.as_ref()).unwrap(), "B");

        let ctx_c = ctx! { "a": false, "b": false };
        assert_eq!(render(&tmpl, ctx_c.as_ref()).unwrap(), "C");
    }

    #[test]
    fn test_render_if_non_bool_error() {
        let tmpl = Template::parse("{% if a %}A{% endif %}").unwrap();
        let ctx = ctx! { "a": "not a bool" };
        let result = render(&tmpl, ctx.as_ref());
        assert!(
            result.is_err(),
            "Conditionals should require boolean values"
        );
        assert_eq!(
            result.unwrap_err(),
            "Conditional expression `a` must evaluate to a Bool, got `not a bool`"
        );
    }

    #[test]
    fn test_render_for_non_list() {
        let tmpl = Template::parse("{% for i in items %}loop{% endfor %}").unwrap();
        let ctx = ctx! { "items": 123 };
        assert!(render(&tmpl, ctx.as_ref()).is_err());
    }

    #[test]
    fn test_render_fragment_missing() {
        let tmpl = Template::parse("{{ Missing ctx }}").unwrap();
        let ctx = ctx! { "ctx": {} };
        assert!(render(&tmpl, ctx.as_ref()).is_err());
    }

    #[test]
    fn test_render_fragment_wrong_ctx_type() {
        let mut tmpl = Template::parse("{{ Frag ctx }}").unwrap();
        tmpl.add_fragment("Frag", "hi").unwrap();
        let ctx = ctx! { "ctx": 123 };
        assert!(render(&tmpl, ctx.as_ref()).is_err());
    }

    #[test]
    fn test_loop_scoping() {
        let src = "{% for i in items %}{{ i }}{{ outer }}{% endfor %}";
        let tmpl = Template::parse(src).unwrap();
        let ctx = ctx! {
            "outer": "!",
            "items": [1, 2]
        };
        assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "1!2!");
    }
}
