/// Template syntax:
///   {{ expr }}              — output expression (dotted paths ok: user.name)
///   {{ FragmentName expr }} — render named fragment with expr as context
///   {% if expr %}           — conditional
///   {% elif expr %}         — else-if branch
///   {% else %}              — else branch
///   {% endif %}             — end conditional
///   {% for item in list %}  — loop
///   {% endfor %}            — end loop
///   {# comment #}           — ignored
///
/// Fragment calls are distinguished from plain output by the first token
/// starting with an ASCII uppercase letter.

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Raw text to emit as-is
    Text(String),
    /// {{ expr }}  (first word is lowercase / path / literal)
    Output(String),
    /// {{ FragmentName expr }}  (first word starts with uppercase)
    FragmentCall { fragment: String, ctx_expr: String },
    /// {% if expr %}
    If(String),
    /// {% elif expr %}
    Elif(String),
    /// {% else %}
    Else,
    /// {% endif %}
    EndIf,
    /// {% for item in list %}
    For { item: String, list: String },
    /// {% endfor %}
    EndFor,
}

pub fn tokenize(src: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = src.char_indices().peekable();
    let mut text_buf = String::new();

    while let Some((_i, c)) = chars.next() {
        if c == '{' {
            match chars.peek().map(|(_, c)| *c) {
                Some('{') => {
                    chars.next();
                    if !text_buf.is_empty() {
                        tokens.push(Token::Text(std::mem::take(&mut text_buf)));
                    }
                    let inner = read_until(&mut chars, "}}")?;
                    let trimmed = inner.trim();
                    tokens.push(parse_output(trimmed));
                }
                Some('%') => {
                    chars.next();
                    if !text_buf.is_empty() {
                        tokens.push(Token::Text(std::mem::take(&mut text_buf)));
                    }
                    let inner = read_until(&mut chars, "%}")?;
                    tokens.push(parse_block(inner.trim())?);
                }
                Some('#') => {
                    chars.next();
                    // comment — consume until #}
                    read_until(&mut chars, "#}")?;
                }
                _ => {
                    text_buf.push(c);
                }
            }
        } else {
            text_buf.push(c);
        }
    }

    if !text_buf.is_empty() {
        tokens.push(Token::Text(text_buf));
    }

    Ok(tokens)
}

fn read_until(
    chars: &mut std::iter::Peekable<std::str::CharIndices>,
    end: &str,
) -> Result<String, String> {
    let mut buf = String::new();

    loop {
        match chars.next() {
            None => return Err(format!("Unexpected end of template, expected `{}`", end)),
            Some((_, c)) => {
                buf.push(c);
                if buf.ends_with(end) {
                    let len = buf.len() - end.len();
                    buf.truncate(len);
                    return Ok(buf);
                }
            }
        }
    }
}

/// Decide whether an output block is a fragment call or a plain expression.
/// A fragment call starts with an uppercase ASCII letter followed by whitespace.
fn parse_output(inner: &str) -> Token {
    let first_word_end = inner
        .find(|c: char| c.is_whitespace())
        .unwrap_or(inner.len());
    let first_word = &inner[..first_word_end];

    if first_word.starts_with(|c: char| c.is_ascii_uppercase()) && first_word_end < inner.len() {
        Token::FragmentCall {
            fragment: first_word.to_string(),
            ctx_expr: inner[first_word_end..].trim().to_string(),
        }
    } else {
        Token::Output(inner.to_string())
    }
}

fn parse_block(inner: &str) -> Result<Token, String> {
    if inner == "else" {
        return Ok(Token::Else);
    }
    if inner == "endif" {
        return Ok(Token::EndIf);
    }
    if inner == "endfor" {
        return Ok(Token::EndFor);
    }

    if let Some(rest) = inner.strip_prefix("if ") {
        return Ok(Token::If(rest.trim().to_string()));
    }

    if let Some(rest) = inner.strip_prefix("elif ") {
        return Ok(Token::Elif(rest.trim().to_string()));
    }

    if let Some(rest) = inner.strip_prefix("for ") {
        // "item in list"
        let parts: Vec<&str> = rest.splitn(3, ' ').collect();
        if parts.len() == 3 && parts[1] == "in" {
            return Ok(Token::For {
                item: parts[0].trim().to_string(),
                list: parts[2].trim().to_string(),
            });
        }
        return Err(format!("Invalid for syntax: `{}`", inner));
    }

    Err(format!("Unknown block tag: `{}`", inner))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_basic() {
        let src = "Hello {{ name }}!";
        let tokens = tokenize(src).unwrap();
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0], Token::Text("Hello ".to_string()));
        assert_eq!(tokens[1], Token::Output("name".to_string()));
        assert_eq!(tokens[2], Token::Text("!".to_string()));
    }

    #[test]
    fn test_tokenize_fragment() {
        let src = "{{ Fragment arg }}";
        let tokens = tokenize(src).unwrap();
        assert_eq!(
            tokens[0],
            Token::FragmentCall {
                fragment: "Fragment".to_string(),
                ctx_expr: "arg".to_string()
            }
        );
    }

    #[test]
    fn test_tokenize_blocks() {
        let src = "{% if a %}A{% elif b %}B{% else %}C{% endif %}";
        let tokens = tokenize(src).unwrap();
        assert_eq!(tokens[0], Token::If("a".to_string()));
        assert_eq!(tokens[1], Token::Text("A".to_string()));
        assert_eq!(tokens[2], Token::Elif("b".to_string()));
        assert_eq!(tokens[3], Token::Text("B".to_string()));
        assert_eq!(tokens[4], Token::Else);
        assert_eq!(tokens[5], Token::Text("C".to_string()));
        assert_eq!(tokens[6], Token::EndIf);
    }

    #[test]
    fn test_tokenize_for() {
        let src = "{% for item in items %}...{% endfor %}";
        let tokens = tokenize(src).unwrap();
        assert_eq!(
            tokens[0],
            Token::For {
                item: "item".to_string(),
                list: "items".to_string()
            }
        );
        assert_eq!(tokens[2], Token::EndFor);
    }

    #[test]
    fn test_tokenize_unclosed() {
        assert!(tokenize("{{ unclosed").is_err());
        assert!(tokenize("{% unclosed").is_err());
        assert!(tokenize("{# unclosed").is_err());
    }

    #[test]
    fn test_tokenize_utf8() {
        let src = "你好 {{ 名字 }}！";
        let tokens = tokenize(src).unwrap();
        assert_eq!(tokens[0], Token::Text("你好 ".to_string()));
        assert_eq!(tokens[1], Token::Output("名字".to_string()));
        assert_eq!(tokens[2], Token::Text("！".to_string()));
    }

    #[test]
    fn test_parse_output_edge_cases() {
        // Starts with uppercase but no space -> Output
        assert!(matches!(parse_output("Fragment"), Token::Output(_)));
        // Starts with uppercase and space -> FragmentCall
        assert!(matches!(
            parse_output("Fragment "),
            Token::FragmentCall { .. }
        ));
        // Lowercase start -> Output
        assert!(matches!(parse_output("fragment arg"), Token::Output(_)));
    }

    #[test]
    fn test_parse_block_invalid() {
        assert!(parse_block("unknown").is_err());
        assert!(parse_block("for item items").is_err()); // missing 'in'
    }

    #[test]
    fn test_comment() {
        let src = "A{# comment #}B";
        let tokens = tokenize(src).unwrap();
        // Comment should be skipped, and text around it can be merged
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::Text("AB".to_string()));
    }
}
