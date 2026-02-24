use crate::parser::{Node, Template};
use crate::value::{DataSource, Value};
use std::sync::Arc;

/// Render a template against a data source, returning the output string.
pub fn render(template: &Template, ctx: &dyn DataSource) -> Result<String, String> {
    let mut out = String::new();
    render_nodes(&template.nodes, ctx, template, &mut out)?;
    Ok(out)
}

fn render_nodes(
    nodes: &[Node],
    ctx: &dyn DataSource,
    template: &Template,
    out: &mut String,
) -> Result<(), String> {
    for node in nodes {
        render_node(node, ctx, template, out)?;
    }
    Ok(())
}

fn render_node(
    node: &Node,
    ctx: &dyn DataSource,
    template: &Template,
    out: &mut String,
) -> Result<(), String> {
    match node {
        Node::Text(s) => out.push_str(s),

        Node::Output(expr) => {
            let val = eval(expr, ctx);
            out.push_str(&val.render());
        }

        Node::If {
            branches,
            else_body,
        } => {
            let mut rendered = false;
            for (cond, body) in branches {
                if eval(cond, ctx).is_truthy() {
                    render_nodes(body, ctx, template, out)?;
                    rendered = true;
                    break;
                }
            }
            if !rendered && let Some(body) = else_body {
                render_nodes(body, ctx, template, out)?;
            }
        }

        Node::For { item, list, body } => {
            let list_val = eval(list, ctx);
            let items = match list_val {
                Value::List(l) => l,
                _ => return Ok(()), // non-list → skip
            };

            for val in items.iter() {
                // Build a child context that overlays the loop variable
                let child = LoopContext {
                    item_name: item.as_str(),
                    item_val: val.clone(),
                    parent: ctx,
                };
                render_nodes(body, &child, template, out)?;
            }
        }

        Node::Fragment { name, ctx_expr } => {
            // Resolve the context expression — must evaluate to a Map
            let ctx_val = eval(ctx_expr, ctx);
            let frag_ctx: &dyn DataSource = match &ctx_val {
                Value::Map(src) => src.as_ref(),
                Value::Null => {
                    return Err(format!(
                        "Fragment `{}`: context expression `{}` resolved to null",
                        name, ctx_expr
                    ));
                }
                _ => {
                    return Err(format!(
                        "Fragment `{}`: context must be a Map, got `{}`",
                        name,
                        ctx_val.render()
                    ));
                }
            };

            match template.fragments.get(name) {
                Some(nodes) => render_nodes(nodes, frag_ctx, template, out)?,
                None => return Err(format!("Unknown fragment: `{}`", name)),
            }
        }
    }
    Ok(())
}

/// Evaluate an expression string against a context.
///
/// 'eval' here refers to the process of resolving an expression (like "user.name" or "!flag")
/// into a `Value` by looking it up in the provided `DataSource` or parsing it as a literal.
/// Currently, this is a simple lookup/literal parser and doesn't support full arithmetic.
fn eval(expr: &str, ctx: &dyn DataSource) -> Value {
    match expr {
        "true" => return Value::Bool(true),
        "false" => return Value::Bool(false),
        "null" => return Value::Null,
        _ => {}
    }

    // integer literal
    if let Ok(n) = expr.parse::<i64>() {
        return Value::Int(n);
    }

    // float literal
    if let Ok(f) = expr.parse::<f64>() {
        return Value::Float(f);
    }

    // string literal "..." or '...'
    if (expr.starts_with('"') && expr.ends_with('"'))
        || (expr.starts_with('\'') && expr.ends_with('\''))
    {
        let inner = &expr[1..expr.len() - 1];
        return Value::Str(Arc::from(inner));
    }

    // negation: !expr
    if let Some(rest) = expr.strip_prefix('!') {
        return Value::Bool(!eval(rest.trim(), ctx).is_truthy());
    }

    // dotted path lookup
    let root_key = expr.split('.').next().unwrap();
    let root_val = ctx.get(root_key);

    if expr.contains('.') {
        root_val.get_path(&expr[root_key.len() + 1..])
    } else {
        root_val
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
    fn get(&self, key: &str) -> Value {
        if key == self.item_name {
            self.item_val.clone()
        } else {
            self.parent.get(key)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{context, ctx};

    #[test]
    fn test_eval_literals() {
        let ctx = context! {};
        assert_eq!(eval("true", ctx.as_ref()).render(), "true");
        assert_eq!(eval("false", ctx.as_ref()).render(), "false");
        assert_eq!(eval("null", ctx.as_ref()).render(), "");
        assert_eq!(eval("123", ctx.as_ref()).render(), "123");
        assert_eq!(eval("1.23", ctx.as_ref()).render(), "1.23");
        assert_eq!(eval("\"hello\"", ctx.as_ref()).render(), "hello");
        assert_eq!(eval("'world'", ctx.as_ref()).render(), "world");
    }

    #[test]
    fn test_eval_negation() {
        let ctx = context! { "flag" => Value::Bool(true) };
        assert_eq!(eval("!flag", ctx.as_ref()).render(), "false");
        assert_eq!(eval("!!flag", ctx.as_ref()).render(), "true");
        assert_eq!(eval("!false", ctx.as_ref()).render(), "true");
    }

    #[test]
    fn test_eval_dotted_path() {
        let inner = context! { "a" => Value::Int(1) };
        let ctx = context! { "inner" => Value::Map(inner) };
        assert_eq!(eval("inner.a", ctx.as_ref()).render(), "1");
        assert!(matches!(eval("inner.b", ctx.as_ref()), Value::Null));
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
        // Exposing bug: currently it uses truthiness, but should be an error
        let result = render(&tmpl, ctx.as_ref());
        assert!(result.is_err(), "Conditionals should require boolean values");
        assert_eq!(result.unwrap_err(), "Conditional expression `a` must evaluate to a Bool, got `not a bool` ");
    }

    #[test]
    fn test_render_for_non_list() {
        let tmpl = Template::parse("{% for i in items %}loop{% endfor %}").unwrap();
        let ctx = context! { "items" => Value::Int(123) };
        // Exposing bug: currently it skips silently, but should probably be an error.
        assert!(render(&tmpl, ctx.as_ref()).is_err());
    }

    #[test]
    fn test_render_fragment_missing() {
        let tmpl = Template::parse("{{ Missing ctx }}").unwrap();
        let ctx = context! { "ctx" => Value::Map(context!{}) };
        assert!(render(&tmpl, ctx.as_ref()).is_err());
    }

    #[test]
    fn test_render_fragment_wrong_ctx_type() {
        let mut tmpl = Template::parse("{{ Frag ctx }}").unwrap();
        tmpl.add_fragment("Frag", "hi").unwrap();
        let ctx = context! { "ctx" => Value::Int(123) };
        assert!(render(&tmpl, ctx.as_ref()).is_err());
    }

    #[test]
    fn test_loop_scoping() {
        let src = "{% for i in items %}{{ i }}{{ outer }}{% endfor %}";
        let tmpl = Template::parse(src).unwrap();
        let ctx = context! {
            "outer" => Value::from("!"),
            "items" => Value::from(vec![Value::Int(1), Value::Int(2)])
        };
        assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "1!2!");
    }
}
