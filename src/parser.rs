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
