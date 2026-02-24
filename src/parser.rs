use crate::lexer::{Token, tokenize};
use std::collections::HashMap;

/// AST nodes
#[derive(Debug, Clone)]
pub enum Node {
    Text(String),
    Output(String),
    /// {{ FragmentName ctx_expr }} — render a named fragment with a scoped context
    Fragment {
        name: String,
        ctx_expr: String,
    },
    If {
        branches: Vec<(String, Vec<Node>)>,
        else_body: Option<Vec<Node>>,
    },
    For {
        item: String,
        list: String,
        body: Vec<Node>,
    },
}

/// A parsed template, ready to render.
/// Fragments can be registered and referenced via {% include "name" %}.
#[derive(Debug, Clone)]
pub struct Template {
    pub nodes: Vec<Node>,
    pub fragments: HashMap<String, Vec<Node>>,
}

impl Template {
    pub fn parse(src: &str) -> Result<Self, String> {
        let tokens = tokenize(src)?;
        let mut iter = tokens.into_iter().peekable();
        let nodes = parse_nodes(&mut iter, false, false)?;
        Ok(Template {
            nodes,
            fragments: HashMap::new(),
        })
    }

    /// Register a named fragment that can be included with {% include "name" %}
    pub fn add_fragment(&mut self, name: &str, src: &str) -> Result<(), String> {
        let tokens = tokenize(src)?;
        let mut iter = tokens.into_iter().peekable();
        let nodes = parse_nodes(&mut iter, false, false)?;
        self.fragments.insert(name.to_string(), nodes);
        Ok(())
    }
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
            Token::Output(expr) => nodes.push(Node::Output(expr)),
            Token::FragmentCall { fragment, ctx_expr } => {
                nodes.push(Node::Fragment {
                    name: fragment,
                    ctx_expr,
                });
            }

            Token::If(cond) => {
                let mut branches = vec![];
                let body = parse_nodes(iter, true, in_for)?;
                branches.push((cond, body));

                // collect elif/else
                let mut else_body = None;
                loop {
                    match iter.peek() {
                        Some(Token::Elif(_)) => {
                            if let Some(Token::Elif(cond)) = iter.next() {
                                let body = parse_nodes(iter, true, in_for)?;
                                branches.push((cond, body));
                            }
                        }
                        Some(Token::Else) => {
                            iter.next();
                            else_body = Some(parse_nodes(iter, true, in_for)?);
                            // consume endif
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
                let body = parse_nodes(iter, in_if, true)?;
                match iter.next() {
                    Some(Token::EndFor) => {}
                    _ => return Err("Expected {% endfor %}".to_string()),
                }
                nodes.push(Node::For { item, list, body });
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
            assert_eq!(branches[0].0, "a");
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
        // The parser currently allows elif after else because of the loop structure,
        // but it might lead to unexpected behavior or error.
        // Let's see what it does.
        let result = Template::parse(src);
        // In current implementation:
        // loop for elif/else:
        //   match Else -> consume endif and BREAK.
        // So anything after {% else %} but before its body is consumed...
        // wait, parse_nodes is called for else body.
        // Let's re-read parse_nodes for Else:
        /*
                        Some(Token::Else) => {
                            iter.next();
                            else_body = Some(parse_nodes(iter, true, in_for)?);
                            // consume endif
                            match iter.next() {
                                Some(Token::EndIf) => {}
                                _ => return Err("Expected {% endif %}".to_string()),
                            }
                            break;
                        }
        */
        // It calls parse_nodes for the else body. If it sees {% elif %} inside the else body,
        // it will be an error because in_if is true, and it breaks loop, then back in Else branch
        // it expects EndIf but gets Elif.
        assert!(result.is_err());
    }
}
