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
