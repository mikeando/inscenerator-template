use crate::value::Value;
use std::fmt;

// --- Public types ---

/// A parsed expression node.
#[derive(Debug, Clone)]
pub enum Expr {
    Literal(Value),
    Path(String),
    Not(Box<Expr>),
    BinOp {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone)]
pub enum BinOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinOp::Eq => write!(f, "=="),
            BinOp::Ne => write!(f, "!="),
            BinOp::Lt => write!(f, "<"),
            BinOp::Le => write!(f, "<="),
            BinOp::Gt => write!(f, ">"),
            BinOp::Ge => write!(f, ">="),
            BinOp::And => write!(f, "and"),
            BinOp::Or => write!(f, "or"),
        }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Literal(v) => match v {
                Value::Bool(b) => write!(f, "{}", b),
                Value::Int(n) => write!(f, "{}", n),
                Value::Float(n) => write!(f, "{}", n),
                Value::Str(s) => write!(f, "\"{}\"", s),
                Value::Null => write!(f, "null"),
                _ => write!(f, "[expr]"),
            },
            Expr::Path(p) => write!(f, "{}", p),
            Expr::Not(e) => write!(f, "!{}", e),
            Expr::BinOp { op, lhs, rhs } => write!(f, "{} {} {}", lhs, op, rhs),
            Expr::Call { name, args } => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
        }
    }
}

// --- Internal lexer ---

#[derive(Debug, Clone, PartialEq)]
enum ExprToken {
    Ident(String),
    Int(i64),
    Float(f64),
    Str(String),
    /// Two-char ops: "==", "!=", "<=", ">="  or single-char: "<", ">", "!"
    Op(String),
    Dot,
    Comma,
    LParen,
    RParen,
}

fn tokenize_expr(input: &str) -> Result<Vec<ExprToken>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if c.is_whitespace() {
            i += 1;
            continue;
        }

        // String literal
        if c == '"' || c == '\'' {
            let quote = c;
            i += 1;
            let mut s = String::new();
            while i < chars.len() && chars[i] != quote {
                s.push(chars[i]);
                i += 1;
            }
            if i >= chars.len() {
                return Err("Unterminated string literal in expression".to_string());
            }
            i += 1; // consume closing quote
            tokens.push(ExprToken::Str(s));
            continue;
        }

        // Number
        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            // Float if followed by '.' and a digit
            if i < chars.len()
                && chars[i] == '.'
                && i + 1 < chars.len()
                && chars[i + 1].is_ascii_digit()
            {
                i += 1; // consume '.'
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                let f: f64 = s.parse().map_err(|_| format!("Invalid float: {}", s))?;
                tokens.push(ExprToken::Float(f));
            } else {
                let s: String = chars[start..i].iter().collect();
                let n: i64 = s.parse().map_err(|_| format!("Invalid integer: {}", s))?;
                tokens.push(ExprToken::Int(n));
            }
            continue;
        }

        // Identifier (includes unicode letters)
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            tokens.push(ExprToken::Ident(s));
            continue;
        }

        match c {
            '.' => {
                tokens.push(ExprToken::Dot);
                i += 1;
            }
            ',' => {
                tokens.push(ExprToken::Comma);
                i += 1;
            }
            '(' => {
                tokens.push(ExprToken::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(ExprToken::RParen);
                i += 1;
            }
            '!' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(ExprToken::Op("!=".to_string()));
                    i += 2;
                } else {
                    tokens.push(ExprToken::Op("!".to_string()));
                    i += 1;
                }
            }
            '=' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(ExprToken::Op("==".to_string()));
                    i += 2;
                } else {
                    return Err("Unexpected '=' — did you mean '=='?".to_string());
                }
            }
            '<' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(ExprToken::Op("<=".to_string()));
                    i += 2;
                } else {
                    tokens.push(ExprToken::Op("<".to_string()));
                    i += 1;
                }
            }
            '>' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(ExprToken::Op(">=".to_string()));
                    i += 2;
                } else {
                    tokens.push(ExprToken::Op(">".to_string()));
                    i += 1;
                }
            }
            '&' => {
                if i + 1 < chars.len() && chars[i + 1] == '&' {
                    tokens.push(ExprToken::Op("&&".to_string()));
                    i += 2;
                } else {
                    return Err("Unexpected '&' — did you mean '&&'?".to_string());
                }
            }
            '|' => {
                if i + 1 < chars.len() && chars[i + 1] == '|' {
                    tokens.push(ExprToken::Op("||".to_string()));
                    i += 2;
                } else {
                    return Err("Unexpected '|' — did you mean '||'?".to_string());
                }
            }
            other => return Err(format!("Unexpected character '{}' in expression", other)),
        }
    }

    Ok(tokens)
}

// --- Recursive-descent parser ---

struct Parser {
    tokens: Vec<ExprToken>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<ExprToken>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&ExprToken> {
        self.tokens.get(self.pos)
    }

    fn next_token(&mut self) -> Option<ExprToken> {
        if self.pos < self.tokens.len() {
            let t = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(t)
        } else {
            None
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_logical()
    }

    fn parse_logical(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_comparison()?;

        loop {
            let op = match self.peek() {
                Some(ExprToken::Op(s)) if s == "&&" => BinOp::And,
                Some(ExprToken::Op(s)) if s == "||" => BinOp::Or,
                Some(ExprToken::Ident(s)) if s == "and" => BinOp::And,
                Some(ExprToken::Ident(s)) if s == "or" => BinOp::Or,
                _ => break,
            };
            self.next_token(); // consume the operator
            let rhs = self.parse_comparison()?;
            lhs = Expr::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }

        Ok(lhs)
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let lhs = self.parse_unary()?;

        let op = match self.peek() {
            Some(ExprToken::Op(s)) => match s.as_str() {
                "==" => BinOp::Eq,
                "!=" => BinOp::Ne,
                "<=" => BinOp::Le,
                ">=" => BinOp::Ge,
                "<" => BinOp::Lt,
                ">" => BinOp::Gt,
                _ => return Ok(lhs), // "!" is unary, not binary
            },
            _ => return Ok(lhs),
        };

        self.next_token(); // consume the op token
        let rhs = self.parse_unary()?;
        Ok(Expr::BinOp {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        })
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        if let Some(ExprToken::Op(s)) = self.peek()
            && s == "!"
        {
            self.next_token();
            let e = self.parse_unary()?;
            return Ok(Expr::Not(Box::new(e)));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some(ExprToken::Int(_)) => {
                if let Some(ExprToken::Int(n)) = self.next_token() {
                    Ok(Expr::Literal(Value::Int(n)))
                } else {
                    unreachable!()
                }
            }
            Some(ExprToken::Float(_)) => {
                if let Some(ExprToken::Float(f)) = self.next_token() {
                    Ok(Expr::Literal(Value::Float(f)))
                } else {
                    unreachable!()
                }
            }
            Some(ExprToken::Str(_)) => {
                if let Some(ExprToken::Str(s)) = self.next_token() {
                    Ok(Expr::Literal(Value::Str(s)))
                } else {
                    unreachable!()
                }
            }
            Some(ExprToken::Ident(_)) => {
                if let Some(ExprToken::Ident(name)) = self.next_token() {
                    // Boolean, null, and operator keywords
                    match name.as_str() {
                        "true" => return Ok(Expr::Literal(Value::Bool(true))),
                        "false" => return Ok(Expr::Literal(Value::Bool(false))),
                        "null" => return Ok(Expr::Literal(Value::Null)),
                        "and" | "or" => {
                            return Err(format!(
                                "`{}` is a reserved keyword and cannot be used as an identifier",
                                name
                            ));
                        }
                        _ => {}
                    }

                    // Function call: ident '(' ... ')'
                    if let Some(ExprToken::LParen) = self.peek() {
                        self.next_token(); // consume '('
                        let mut args = Vec::new();
                        if self.peek() != Some(&ExprToken::RParen) {
                            args.push(self.parse_expr()?);
                            while let Some(ExprToken::Comma) = self.peek() {
                                self.next_token(); // consume ','
                                args.push(self.parse_expr()?);
                            }
                        }
                        match self.next_token() {
                            Some(ExprToken::RParen) => {}
                            other => {
                                return Err(format!(
                                    "Expected ')' to close function call, got {:?}",
                                    other
                                ));
                            }
                        }
                        return Ok(Expr::Call { name, args });
                    }

                    // Dotted path: ident ('.' ident)*
                    let mut path = name;
                    while let Some(ExprToken::Dot) = self.peek() {
                        self.next_token(); // consume '.'
                        match self.next_token() {
                            Some(ExprToken::Ident(segment)) => {
                                path.push('.');
                                path.push_str(&segment);
                            }
                            other => {
                                return Err(format!(
                                    "Expected identifier after '.', got {:?}",
                                    other
                                ));
                            }
                        }
                    }
                    Ok(Expr::Path(path))
                } else {
                    unreachable!()
                }
            }
            other => Err(format!("Expected expression, got {:?}", other)),
        }
    }
}

/// Parse an expression string into an `Expr` AST node.
/// Returns an error if the string is not a valid expression.
pub fn parse_expr(input: &str) -> Result<Expr, String> {
    let tokens = tokenize_expr(input.trim())?;
    if tokens.is_empty() {
        return Err("Empty expression".to_string());
    }
    let mut parser = Parser::new(tokens);
    let expr = parser.parse_expr()?;
    if parser.pos < parser.tokens.len() {
        return Err(format!(
            "Unexpected token after expression: {:?}",
            parser.tokens[parser.pos]
        ));
    }
    Ok(expr)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Expr {
        parse_expr(s).unwrap_or_else(|e| panic!("parse_expr({:?}) failed: {}", s, e))
    }

    fn parse_err(s: &str) -> String {
        parse_expr(s).unwrap_err()
    }

    // --- Literals ---

    #[test]
    fn test_literal_true() {
        assert!(matches!(parse("true"), Expr::Literal(Value::Bool(true))));
    }

    #[test]
    fn test_literal_false() {
        assert!(matches!(parse("false"), Expr::Literal(Value::Bool(false))));
    }

    #[test]
    fn test_literal_null() {
        assert!(matches!(parse("null"), Expr::Literal(Value::Null)));
    }

    #[test]
    fn test_literal_int() {
        assert!(matches!(parse("42"), Expr::Literal(Value::Int(42))));
        assert!(matches!(parse("0"), Expr::Literal(Value::Int(0))));
    }

    #[test]
    fn test_literal_float() {
        assert!(matches!(parse("1.5"), Expr::Literal(Value::Float(_))));
        if let Expr::Literal(Value::Float(f)) = parse("1.5") {
            assert!((f - 1.5).abs() < 1e-10);
        }
    }

    #[test]
    fn test_literal_string_double_quote() {
        if let Expr::Literal(Value::Str(s)) = parse("\"hello\"") {
            assert_eq!(s, "hello");
        } else {
            panic!("expected Str literal");
        }
    }

    #[test]
    fn test_literal_string_single_quote() {
        if let Expr::Literal(Value::Str(s)) = parse("'world'") {
            assert_eq!(s, "world");
        } else {
            panic!("expected Str literal");
        }
    }

    // --- Paths ---

    #[test]
    fn test_path_simple() {
        if let Expr::Path(p) = parse("name") {
            assert_eq!(p, "name");
        } else {
            panic!("expected Path");
        }
    }

    #[test]
    fn test_path_dotted() {
        if let Expr::Path(p) = parse("user.address.city") {
            assert_eq!(p, "user.address.city");
        } else {
            panic!("expected Path");
        }
    }

    #[test]
    fn test_path_unicode() {
        if let Expr::Path(p) = parse("nåmë") {
            assert_eq!(p, "nåmë");
        } else {
            panic!("expected Path");
        }
    }

    // --- Negation ---

    #[test]
    fn test_negation() {
        assert!(matches!(parse("!flag"), Expr::Not(_)));
    }

    #[test]
    fn test_double_negation() {
        if let Expr::Not(inner) = parse("!!flag") {
            assert!(matches!(*inner, Expr::Not(_)));
        } else {
            panic!("expected Not(Not(...))");
        }
    }

    #[test]
    fn test_negation_literal() {
        assert!(matches!(parse("!true"), Expr::Not(_)));
    }

    // --- Binary operators ---

    #[test]
    fn test_binop_eq() {
        if let Expr::BinOp { op: BinOp::Eq, .. } = parse("a == b") {
        } else {
            panic!("expected BinOp Eq");
        }
    }

    #[test]
    fn test_binop_ne() {
        assert!(matches!(parse("a != b"), Expr::BinOp { op: BinOp::Ne, .. }));
    }

    #[test]
    fn test_binop_lt() {
        assert!(matches!(parse("a < b"), Expr::BinOp { op: BinOp::Lt, .. }));
    }

    #[test]
    fn test_binop_le() {
        assert!(matches!(parse("a <= b"), Expr::BinOp { op: BinOp::Le, .. }));
    }

    #[test]
    fn test_binop_gt() {
        assert!(matches!(parse("a > b"), Expr::BinOp { op: BinOp::Gt, .. }));
    }

    #[test]
    fn test_binop_ge() {
        assert!(matches!(parse("a >= b"), Expr::BinOp { op: BinOp::Ge, .. }));
    }

    #[test]
    fn test_binop_with_literal() {
        // age < 18
        if let Expr::BinOp {
            op: BinOp::Lt,
            lhs,
            rhs,
        } = parse("age < 18")
        {
            assert!(matches!(*lhs, Expr::Path(_)));
            assert!(matches!(*rhs, Expr::Literal(Value::Int(18))));
        } else {
            panic!("expected BinOp Lt");
        }
    }

    #[test]
    fn test_binop_str_comparison() {
        if let Expr::BinOp {
            op: BinOp::Eq,
            lhs,
            rhs,
        } = parse("name == \"Alice\"")
        {
            assert!(matches!(*lhs, Expr::Path(_)));
            assert!(matches!(*rhs, Expr::Literal(Value::Str(_))));
        } else {
            panic!("expected BinOp Eq");
        }
    }

    // --- Function calls ---

    #[test]
    fn test_call_no_args() {
        if let Expr::Call { name, args } = parse("foo()") {
            assert_eq!(name, "foo");
            assert!(args.is_empty());
        } else {
            panic!("expected Call");
        }
    }

    #[test]
    fn test_call_one_arg() {
        if let Expr::Call { name, args } = parse("len(items)") {
            assert_eq!(name, "len");
            assert_eq!(args.len(), 1);
            assert!(matches!(&args[0], Expr::Path(p) if p == "items"));
        } else {
            panic!("expected Call");
        }
    }

    #[test]
    fn test_call_two_args() {
        if let Expr::Call { name, args } = parse("ends_with(name, '.rs')") {
            assert_eq!(name, "ends_with");
            assert_eq!(args.len(), 2);
        } else {
            panic!("expected Call");
        }
    }

    #[test]
    fn test_call_nested_expr_arg() {
        // starts_with(user.name, "A")
        if let Expr::Call { args, .. } = parse("starts_with(user.name, \"A\")") {
            assert!(matches!(&args[0], Expr::Path(p) if p == "user.name"));
        } else {
            panic!("expected Call");
        }
    }

    #[test]
    fn test_call_in_comparison() {
        // ends_with(name, ".rs") == true
        assert!(matches!(
            parse("ends_with(name, '.rs') == true"),
            Expr::BinOp { op: BinOp::Eq, .. }
        ));
    }

    // --- Display ---

    #[test]
    fn test_display_path() {
        assert_eq!(parse("user.name").to_string(), "user.name");
    }

    #[test]
    fn test_display_binop() {
        assert_eq!(parse("age >= 18").to_string(), "age >= 18");
    }

    #[test]
    fn test_display_not() {
        assert_eq!(parse("!flag").to_string(), "!flag");
    }

    #[test]
    fn test_display_call() {
        assert_eq!(parse("len(items)").to_string(), "len(items)");
    }

    // --- Error cases ---

    #[test]
    fn test_error_empty() {
        assert!(parse_expr("").is_err());
        assert!(parse_expr("   ").is_err());
    }

    #[test]
    fn test_error_trailing_tokens() {
        let err = parse_err("a b");
        assert!(err.contains("Unexpected token"), "error was: {}", err);
    }

    #[test]
    fn test_error_unclosed_string() {
        assert!(parse_expr("\"unclosed").is_err());
    }

    #[test]
    fn test_error_unclosed_paren() {
        assert!(parse_expr("foo(a, b").is_err());
    }

    #[test]
    fn test_error_bad_char() {
        assert!(parse_expr("a + b").is_err());
    }

    #[test]
    fn test_error_bare_equals() {
        assert!(parse_expr("a = b").is_err());
    }

    #[test]
    fn test_error_dot_at_end() {
        assert!(parse_expr("a.").is_err());
    }

    #[test]
    fn test_error_leading_dot() {
        assert!(parse_expr(".a").is_err());
    }
}
