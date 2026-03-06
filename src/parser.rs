use crate::expr::{Expr, parse_expr};
use crate::lexer::{Token, tokenize};
use crate::value::{Function, Value};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// AST nodes
#[derive(Debug, Clone)]
pub enum Node {
    Text(String),
    Output(Expr),
    /// {{ FragmentName ctx_expr }} — render a named fragment with a scoped context
    Fragment {
        name: String,
        ctx_expr: Expr,
    },
    If {
        branches: Vec<(Expr, Vec<Node>)>,
        else_body: Option<Vec<Node>>,
    },
    For {
        item: String,
        list: Expr,
        body: Vec<Node>,
    },
}

/// A parsed template, ready to render.
#[derive(Clone)]
pub struct Template {
    pub nodes: Vec<Node>,
    pub fragments: HashMap<String, Vec<Node>>,
    pub(crate) functions: HashMap<String, Arc<dyn Function>>,
}

impl fmt::Debug for Template {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Template")
            .field("nodes", &self.nodes)
            .field("fragments", &self.fragments)
            .field("functions", &self.functions.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl Template {
    pub fn parse(src: &str) -> Result<Self, String> {
        let tokens = tokenize(src)?;
        let mut iter = tokens.into_iter().peekable();
        let nodes = parse_nodes(&mut iter, false, false)?;
        Ok(Template {
            nodes,
            fragments: HashMap::new(),
            functions: make_builtins(),
        })
    }

    /// Register a named fragment that can be called with {{ FragmentName expr }}
    pub fn add_fragment(&mut self, name: &str, src: &str) -> Result<(), String> {
        let tokens = tokenize(src)?;
        let mut iter = tokens.into_iter().peekable();
        let nodes = parse_nodes(&mut iter, false, false)?;
        self.fragments.insert(name.to_string(), nodes);
        Ok(())
    }

    /// Register a named function callable from templates.
    /// Overwrites any existing function (including built-ins) with the same name.
    pub fn add_function(&mut self, name: &str, f: impl Function + 'static) -> &mut Self {
        self.functions.insert(name.to_string(), Arc::new(f));
        self
    }
}

fn make_builtins() -> HashMap<String, Arc<dyn Function>> {
    let mut fns: HashMap<String, Arc<dyn Function>> = HashMap::new();

    fns.insert(
        "len".to_string(),
        Arc::new(|args: &[Value]| -> Result<Value, String> {
            match args {
                [Value::Str(s)] => Ok(Value::Int(s.chars().count() as i64)),
                [Value::List(l)] => Ok(Value::Int(l.len() as i64)),
                [_] => Err("len() expects a Str or List argument".to_string()),
                _ => Err(format!("len() expects 1 argument, got {}", args.len())),
            }
        }),
    );

    fns.insert(
        "starts_with".to_string(),
        Arc::new(|args: &[Value]| -> Result<Value, String> {
            match args {
                [Value::Str(s), Value::Str(prefix)] => {
                    Ok(Value::Bool(s.starts_with(prefix.as_str())))
                }
                [_, _] => Err("starts_with() expects two Str arguments".to_string()),
                _ => Err(format!(
                    "starts_with() expects 2 arguments, got {}",
                    args.len()
                )),
            }
        }),
    );

    fns.insert(
        "ends_with".to_string(),
        Arc::new(|args: &[Value]| -> Result<Value, String> {
            match args {
                [Value::Str(s), Value::Str(suffix)] => {
                    Ok(Value::Bool(s.ends_with(suffix.as_str())))
                }
                [_, _] => Err("ends_with() expects two Str arguments".to_string()),
                _ => Err(format!(
                    "ends_with() expects 2 arguments, got {}",
                    args.len()
                )),
            }
        }),
    );

    fns.insert(
        "contains".to_string(),
        Arc::new(|args: &[Value]| -> Result<Value, String> {
            match args {
                [Value::Str(s), Value::Str(needle)] => Ok(Value::Bool(s.contains(needle.as_str()))),
                [_, _] => Err("contains() expects two Str arguments".to_string()),
                _ => Err(format!(
                    "contains() expects 2 arguments, got {}",
                    args.len()
                )),
            }
        }),
    );

    fns.insert(
        "upper".to_string(),
        Arc::new(|args: &[Value]| -> Result<Value, String> {
            match args {
                [Value::Str(s)] => Ok(Value::Str(s.to_uppercase())),
                [_] => Err("upper() expects a Str argument".to_string()),
                _ => Err(format!("upper() expects 1 argument, got {}", args.len())),
            }
        }),
    );

    fns.insert(
        "lower".to_string(),
        Arc::new(|args: &[Value]| -> Result<Value, String> {
            match args {
                [Value::Str(s)] => Ok(Value::Str(s.to_lowercase())),
                [_] => Err("lower() expects a Str argument".to_string()),
                _ => Err(format!("lower() expects 1 argument, got {}", args.len())),
            }
        }),
    );

    fns.insert(
        "trim".to_string(),
        Arc::new(|args: &[Value]| -> Result<Value, String> {
            match args {
                [Value::Str(s)] => Ok(Value::Str(s.trim().into())),
                [_] => Err("trim() expects a Str argument".to_string()),
                _ => Err(format!("trim() expects 1 argument, got {}", args.len())),
            }
        }),
    );

    fns.insert(
        "replace".to_string(),
        Arc::new(|args: &[Value]| -> Result<Value, String> {
            match args {
                [Value::Str(s), Value::Str(from), Value::Str(to)] => {
                    Ok(Value::Str(s.replace(from.as_str(), to.as_str())))
                }
                [_, _, _] => Err("replace() expects three Str arguments".to_string()),
                _ => Err(format!("replace() expects 3 arguments, got {}", args.len())),
            }
        }),
    );

    fns.insert(
        "enumerate".to_string(),
        Arc::new(|args: &[Value]| -> Result<Value, String> {
            match args {
                [Value::List(list)] => {
                    let result: Vec<Value> = list
                        .iter()
                        .enumerate()
                        .map(|(i, v)| {
                            let mut map = HashMap::new();
                            map.insert("index".to_string(), Value::Int(i as i64));
                            map.insert("value".to_string(), v.clone());
                            Value::Map(map)
                        })
                        .collect();
                    Ok(Value::List(Arc::new(result)))
                }
                [_] => Err("enumerate() expects a List argument".to_string()),
                _ => Err(format!(
                    "enumerate() expects 1 argument, got {}",
                    args.len()
                )),
            }
        }),
    );

    fns
}

fn parse_nodes(
    iter: &mut std::iter::Peekable<std::vec::IntoIter<Token>>,
    in_if: bool,
    in_for: bool,
) -> Result<Vec<Node>, String> {
    let mut nodes = Vec::new();

    loop {
        match iter.peek() {
            None => break,
            Some(Token::EndIf) if in_if => break,
            Some(Token::Else) if in_if => break,
            Some(Token::Elif(_)) if in_if => break,
            Some(Token::EndFor) if in_for => break,
            _ => {}
        }

        let token = match iter.next() {
            None => break,
            Some(t) => t,
        };

        match token {
            Token::Text(s) => nodes.push(Node::Text(s)),
            Token::Output(expr_str) => {
                let expr = parse_expr(&expr_str)
                    .map_err(|e| format!("Invalid expression `{}`: {}", expr_str, e))?;
                nodes.push(Node::Output(expr));
            }
            Token::FragmentCall { fragment, ctx_expr } => {
                let expr = parse_expr(&ctx_expr)
                    .map_err(|e| format!("Invalid expression `{}`: {}", ctx_expr, e))?;
                nodes.push(Node::Fragment {
                    name: fragment,
                    ctx_expr: expr,
                });
            }

            Token::If(cond_str) => {
                let cond = parse_expr(&cond_str)
                    .map_err(|e| format!("Invalid expression `{}`: {}", cond_str, e))?;
                let mut branches = vec![];
                let body = parse_nodes(iter, true, in_for)?;
                branches.push((cond, body));

                // collect elif/else
                let mut else_body = None;
                loop {
                    match iter.peek() {
                        Some(Token::Elif(_)) => {
                            if let Some(Token::Elif(elif_str)) = iter.next() {
                                let elif_cond = parse_expr(&elif_str).map_err(|e| {
                                    format!("Invalid expression `{}`: {}", elif_str, e)
                                })?;
                                let body = parse_nodes(iter, true, in_for)?;
                                branches.push((elif_cond, body));
                            }
                        }
                        Some(Token::Else) => {
                            iter.next();
                            else_body = Some(parse_nodes(iter, true, in_for)?);
                            match iter.next() {
                                Some(Token::EndIf) => {}
                                _ => return Err("Expected {% endif %}".to_string()),
                            }
                            break;
                        }
                        Some(Token::EndIf) => {
                            iter.next();
                            break;
                        }
                        _ => return Err("Expected {% endif %} or {% else %}".to_string()),
                    }
                }

                nodes.push(Node::If {
                    branches,
                    else_body,
                });
            }

            Token::For { item, list } => {
                let list_expr = parse_expr(&list)
                    .map_err(|e| format!("Invalid expression `{}`: {}", list, e))?;
                let body = parse_nodes(iter, in_if, true)?;
                match iter.next() {
                    Some(Token::EndFor) => {}
                    _ => return Err("Expected {% endfor %}".to_string()),
                }
                nodes.push(Node::For {
                    item,
                    list: list_expr,
                    body,
                });
            }

            Token::EndIf | Token::EndFor | Token::Else | Token::Elif(_) => {
                return Err(format!("Unexpected token: {:?}", token));
            }
        }
    }

    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let src = "Hello {{ name }}!";
        let tmpl = Template::parse(src).unwrap();
        assert_eq!(tmpl.nodes.len(), 3);
    }

    #[test]
    fn test_parse_if_nested() {
        let src = "{% if a %}{% if b %}inner{% endif %}{% else %}outer{% endif %}";
        let tmpl = Template::parse(src).unwrap();
        assert_eq!(tmpl.nodes.len(), 1);
        if let Node::If {
            branches,
            else_body,
        } = &tmpl.nodes[0]
        {
            assert_eq!(branches.len(), 1);
            assert_eq!(branches[0].0.to_string(), "a");
            assert!(else_body.is_some());
        } else {
            panic!("Expected If node");
        }
    }

    #[test]
    fn test_parse_for() {
        let src = "{% for i in items %}{{ i }}{% endfor %}";
        let tmpl = Template::parse(src).unwrap();
        assert_eq!(tmpl.nodes.len(), 1);
        assert!(matches!(tmpl.nodes[0], Node::For { .. }));
    }

    #[test]
    fn test_parse_mismatched_if_for() {
        let src = "{% if a %}{% endfor %}";
        assert!(Template::parse(src).is_err());
    }

    #[test]
    fn test_parse_mismatched_for_if() {
        let src = "{% for i in items %}{% endif %}";
        assert!(Template::parse(src).is_err());
    }

    #[test]
    fn test_parse_unexpected_else() {
        let src = "{% else %}";
        assert!(Template::parse(src).is_err());
    }

    #[test]
    fn test_parse_unexpected_endif() {
        let src = "{% endif %}";
        assert!(Template::parse(src).is_err());
    }

    #[test]
    fn test_add_fragment() {
        let mut tmpl = Template::parse("").unwrap();
        tmpl.add_fragment("Frag", "inside").unwrap();
        assert!(tmpl.fragments.contains_key("Frag"));
    }

    #[test]
    fn test_elif_after_else() {
        let src = "{% if a %}A{% else %}B{% elif c %}C{% endif %}";
        let result = Template::parse(src);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_comparison_expression() {
        let src = "{% if age >= 18 %}adult{% endif %}";
        let tmpl = Template::parse(src).unwrap();
        assert!(matches!(&tmpl.nodes[0], Node::If { .. }));
        if let Node::If { branches, .. } = &tmpl.nodes[0] {
            assert_eq!(branches[0].0.to_string(), "age >= 18");
        }
    }

    #[test]
    fn test_parse_function_call_expression() {
        let src = "{% if ends_with(name, '.rs') %}yes{% endif %}";
        let tmpl = Template::parse(src).unwrap();
        assert!(matches!(&tmpl.nodes[0], Node::If { .. }));
    }

    #[test]
    fn test_parse_invalid_expression_is_error() {
        let src = "{{ a b c }}";
        assert!(Template::parse(src).is_err());
    }

    #[test]
    fn test_builtins_registered() {
        let tmpl = Template::parse("").unwrap();
        assert!(tmpl.functions.contains_key("len"));
        assert!(tmpl.functions.contains_key("starts_with"));
        assert!(tmpl.functions.contains_key("ends_with"));
        assert!(tmpl.functions.contains_key("contains"));
    }

    #[test]
    fn test_add_function() {
        let mut tmpl = Template::parse("").unwrap();
        tmpl.add_function("double", |args: &[Value]| match args {
            [Value::Int(n)] => Ok(Value::Int(n * 2)),
            _ => Err("expected Int".to_string()),
        });
        assert!(tmpl.functions.contains_key("double"));
    }
}
