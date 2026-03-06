use crate::value::Value;
use std::fmt;

// --- Public types ---

/// A single step in a path expression.
#[derive(Debug, Clone)]
pub enum PathStep {
    /// `.ident` — strict dot access
    Dot(String),
    /// `.[expr]` — strict subscript; expr must evaluate to Value::Str at render time
    Subscript(Box<Expr>),
    /// `.?ident` — null-safe dot: propagates Null if receiver is Null or key missing
    NullDot(String),
    /// `.?[expr]` — null-safe subscript: propagates Null if receiver is Null or key missing
    NullSubscript(Box<Expr>),
}

/// The first step of a path — how to look up the root key from the context.
#[derive(Debug, Clone)]
pub enum PathStart {
    /// Plain identifier root: `foo` or `.foo` — strict lookup. Display omits the dot.
    Ident(String),
    /// `.?foo` — null-safe root lookup by identifier.
    NullSafeIdent(String),
    /// `.[expr]` — strict root subscript; expr must evaluate to Value::Str.
    Subscript(Box<Expr>),
    /// `.?[expr]` — null-safe root subscript.
    NullSafeSubscript(Box<Expr>),
}

/// A parsed expression node.
///
/// Examples of `Path`:
/// - `name`                   → `Path { start: Ident("name"), steps: [] }`
/// - `user.address.city`      → `Path { start: Ident("user"), steps: [Dot("address"), Dot("city")] }`
/// - `project.["010_intro"]`  → `Path { start: Ident("project"), steps: [Subscript(Literal("010_intro"))] }`
/// - `a.[config.key].title`   → `Path { start: Ident("a"), steps: [Subscript(Path{Ident(config),[Dot(key)]}), Dot("title")] }`
#[derive(Debug, Clone)]
pub enum Expr {
    Literal(Value),
    Path {
        start: PathStart,
        steps: Vec<PathStep>,
    },
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
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    NullCoalesce, // ??
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
            BinOp::Add => write!(f, "+"),
            BinOp::Sub => write!(f, "-"),
            BinOp::Mul => write!(f, "*"),
            BinOp::Div => write!(f, "/"),
            BinOp::Mod => write!(f, "%"),
            BinOp::NullCoalesce => write!(f, "??"),
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
            Expr::Path { start, steps } => {
                match start {
                    PathStart::Ident(s) => write!(f, "{}", s)?,
                    PathStart::NullSafeIdent(s) => write!(f, ".?{}", s)?,
                    PathStart::Subscript(e) => write!(f, ".[{}]", e)?,
                    PathStart::NullSafeSubscript(e) => write!(f, ".?[{}]", e)?,
                }
                for step in steps {
                    match step {
                        PathStep::Dot(k) => write!(f, ".{}", k)?,
                        PathStep::Subscript(e) => write!(f, ".[{}]", e)?,
                        PathStep::NullDot(k) => write!(f, ".?{}", k)?,
                        PathStep::NullSubscript(e) => write!(f, ".?[{}]", e)?,
                    }
                }
                Ok(())
            }
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
    LBracket, // [
    RBracket, // ]
    Comma,
    LParen,
    RParen,
    NullCoalesce, // ??
    DotQMark,     // .?
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
                if i + 1 < chars.len() && chars[i + 1] == '?' {
                    tokens.push(ExprToken::DotQMark);
                    i += 2;
                } else {
                    tokens.push(ExprToken::Dot);
                    i += 1;
                }
            }
            '[' => {
                tokens.push(ExprToken::LBracket);
                i += 1;
            }
            ']' => {
                tokens.push(ExprToken::RBracket);
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
            '+' => {
                tokens.push(ExprToken::Op("+".to_string()));
                i += 1;
            }
            '-' => {
                tokens.push(ExprToken::Op("-".to_string()));
                i += 1;
            }
            '*' => {
                tokens.push(ExprToken::Op("*".to_string()));
                i += 1;
            }
            '/' => {
                tokens.push(ExprToken::Op("/".to_string()));
                i += 1;
            }
            '%' => {
                tokens.push(ExprToken::Op("%".to_string()));
                i += 1;
            }
            '?' => {
                if i + 1 < chars.len() && chars[i + 1] == '?' {
                    tokens.push(ExprToken::NullCoalesce);
                    i += 2;
                } else {
                    return Err(
                        "Unexpected '?' — did you mean '??' (null-coalesce), '.?foo' (null-safe dot), or '.?[' (null-safe subscript)?".to_string()
                    );
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
        self.parse_null_coalesce()
    }

    fn parse_null_coalesce(&mut self) -> Result<Expr, String> {
        let lhs = self.parse_logical()?;
        if let Some(ExprToken::NullCoalesce) = self.peek() {
            self.next_token(); // consume ??
            let rhs = self.parse_null_coalesce()?; // right-associative recursion
            return Ok(Expr::BinOp {
                op: BinOp::NullCoalesce,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            });
        }
        Ok(lhs)
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
        let lhs = self.parse_additive()?;

        let op = match self.peek() {
            Some(ExprToken::Op(s)) => match s.as_str() {
                "==" => BinOp::Eq,
                "!=" => BinOp::Ne,
                "<=" => BinOp::Le,
                ">=" => BinOp::Ge,
                "<" => BinOp::Lt,
                ">" => BinOp::Gt,
                _ => return Ok(lhs),
            },
            _ => return Ok(lhs),
        };

        self.next_token(); // consume the op token
        let rhs = self.parse_additive()?;
        Ok(Expr::BinOp {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        })
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_multiplicative()?;
        loop {
            let op = match self.peek() {
                Some(ExprToken::Op(s)) if s == "+" => BinOp::Add,
                Some(ExprToken::Op(s)) if s == "-" => BinOp::Sub,
                _ => break,
            };
            self.next_token();
            let rhs = self.parse_multiplicative()?;
            lhs = Expr::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Some(ExprToken::Op(s)) if s == "*" => BinOp::Mul,
                Some(ExprToken::Op(s)) if s == "/" => BinOp::Div,
                Some(ExprToken::Op(s)) if s == "%" => BinOp::Mod,
                _ => break,
            };
            self.next_token();
            let rhs = self.parse_unary()?;
            lhs = Expr::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        if let Some(ExprToken::Op(s)) = self.peek()
            && s == "!"
        {
            self.next_token();
            let e = self.parse_unary()?;
            return Ok(Expr::Not(Box::new(e)));
        }
        // Unary minus: represent as (0 - x)
        if let Some(ExprToken::Op(s)) = self.peek()
            && s == "-"
        {
            self.next_token();
            let e = self.parse_unary()?;
            return Ok(Expr::BinOp {
                op: BinOp::Sub,
                lhs: Box::new(Expr::Literal(Value::Int(0))),
                rhs: Box::new(e),
            });
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            // Grouped expression: ( expr )
            Some(ExprToken::LParen) => {
                self.next_token(); // consume '('
                let e = self.parse_expr()?;
                match self.next_token() {
                    Some(ExprToken::RParen) => Ok(e),
                    other => Err(format!(
                        "Expected ')' to close grouped expression, got {:?}",
                        other
                    )),
                }
            }
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

                    // Path: ident (step)*
                    let start = PathStart::Ident(name);
                    let steps = self.parse_path_steps()?;
                    Ok(Expr::Path { start, steps })
                } else {
                    unreachable!()
                }
            }
            Some(ExprToken::Dot) => {
                self.next_token(); // consume '.'
                match self.next_token() {
                    Some(ExprToken::Ident(name)) => {
                        // .foo — strict root lookup, same AST as foo
                        let start = PathStart::Ident(name);
                        let steps = self.parse_path_steps()?;
                        Ok(Expr::Path { start, steps })
                    }
                    Some(ExprToken::LBracket) => {
                        // .[expr] — strict root subscript
                        let key_expr = self.parse_expr()?;
                        match self.next_token() {
                            Some(ExprToken::RBracket) => {}
                            other => {
                                return Err(format!(
                                    "Expected ']' after root subscript, got {:?}",
                                    other
                                ));
                            }
                        }
                        let start = PathStart::Subscript(Box::new(key_expr));
                        let steps = self.parse_path_steps()?;
                        Ok(Expr::Path { start, steps })
                    }
                    other => Err(format!(
                        "Expected identifier or '[' after '.', got {:?}",
                        other
                    )),
                }
            }
            Some(ExprToken::DotQMark) => {
                self.next_token(); // consume '.?'
                match self.next_token() {
                    Some(ExprToken::Ident(name)) => {
                        // .?foo — null-safe root lookup
                        let start = PathStart::NullSafeIdent(name);
                        let steps = self.parse_path_steps()?;
                        Ok(Expr::Path { start, steps })
                    }
                    Some(ExprToken::LBracket) => {
                        // .?[expr] — null-safe root subscript
                        let key_expr = self.parse_expr()?;
                        match self.next_token() {
                            Some(ExprToken::RBracket) => {}
                            other => {
                                return Err(format!(
                                    "Expected ']' after null-safe root subscript, got {:?}",
                                    other
                                ));
                            }
                        }
                        let start = PathStart::NullSafeSubscript(Box::new(key_expr));
                        let steps = self.parse_path_steps()?;
                        Ok(Expr::Path { start, steps })
                    }
                    other => Err(format!(
                        "Expected identifier or '[' after '.?', got {:?}",
                        other
                    )),
                }
            }
            other => Err(format!("Expected expression, got {:?}", other)),
        }
    }

    fn parse_path_steps(&mut self) -> Result<Vec<PathStep>, String> {
        let mut steps: Vec<PathStep> = Vec::new();
        loop {
            match self.peek() {
                Some(ExprToken::Dot) => {
                    self.next_token();
                    match self.next_token() {
                        Some(ExprToken::Ident(segment)) => steps.push(PathStep::Dot(segment)),
                        Some(ExprToken::LBracket) => {
                            // .[expr] — new strict subscript step syntax
                            let key_expr = self.parse_expr()?;
                            match self.next_token() {
                                Some(ExprToken::RBracket) => {}
                                other => {
                                    return Err(format!(
                                        "Expected ']' after subscript, got {:?}",
                                        other
                                    ));
                                }
                            }
                            steps.push(PathStep::Subscript(Box::new(key_expr)));
                        }
                        other => {
                            return Err(format!(
                                "Expected identifier or '[' after '.', got {:?}",
                                other
                            ));
                        }
                    }
                }
                Some(ExprToken::DotQMark) => {
                    self.next_token();
                    match self.next_token() {
                        Some(ExprToken::Ident(segment)) => steps.push(PathStep::NullDot(segment)),
                        Some(ExprToken::LBracket) => {
                            let key_expr = self.parse_expr()?;
                            match self.next_token() {
                                Some(ExprToken::RBracket) => {}
                                other => {
                                    return Err(format!(
                                        "Expected ']' after null-safe subscript, got {:?}",
                                        other
                                    ));
                                }
                            }
                            steps.push(PathStep::NullSubscript(Box::new(key_expr)));
                        }
                        other => {
                            return Err(format!(
                                "Expected identifier or '[' after '.?', got {:?}",
                                other
                            ));
                        }
                    }
                }
                _ => break,
            }
        }
        Ok(steps)
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
    fn test_path_unicode() {
        if let Expr::Path {
            start: PathStart::Ident(root),
            steps,
        } = parse("nåmë")
        {
            assert_eq!(root, "nåmë");
            assert!(steps.is_empty());
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
            assert!(matches!(*lhs, Expr::Path { .. }));
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
            assert!(matches!(*lhs, Expr::Path { .. }));
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
            assert!(
                matches!(&args[0], Expr::Path { start: PathStart::Ident(s), .. } if s == "items")
            );
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
            if let Expr::Path {
                start: PathStart::Ident(root),
                steps,
            } = &args[0]
            {
                assert_eq!(root, "user");
                assert_eq!(steps.len(), 1);
                assert!(matches!(&steps[0], PathStep::Dot(k) if k == "name"));
            } else {
                panic!("expected Path as first arg");
            }
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
        assert!(parse_expr("a @ b").is_err());
        assert!(parse_expr("a # b").is_err());
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
    fn test_error_leading_dot_bare() {
        // A bare leading dot with no ident or '[' after it is still an error
        assert!(parse_expr(".[").is_err());
        assert!(parse_expr(".").is_err());
    }

    // --- Subscript / PathStep tests ---

    #[test]
    fn test_subscript_literal_key() {
        if let Expr::Path {
            start: PathStart::Ident(root),
            steps,
        } = parse("project.[\"meta\"]")
        {
            assert_eq!(root, "project");
            assert_eq!(steps.len(), 1);
            if let PathStep::Subscript(key_expr) = &steps[0] {
                assert!(matches!(key_expr.as_ref(), Expr::Literal(Value::Str(s)) if s == "meta"));
            } else {
                panic!("expected Subscript step");
            }
        } else {
            panic!("expected Path");
        }
    }

    #[test]
    fn test_subscript_numeric_string_key() {
        if let Expr::Path {
            start: PathStart::Ident(root),
            steps,
        } = parse("project.[\"010_intro\"]")
        {
            assert_eq!(root, "project");
            assert_eq!(steps.len(), 1);
            if let PathStep::Subscript(key_expr) = &steps[0] {
                assert!(
                    matches!(key_expr.as_ref(), Expr::Literal(Value::Str(s)) if s == "010_intro")
                );
            } else {
                panic!("expected Subscript step");
            }
        } else {
            panic!("expected Path");
        }
    }

    #[test]
    fn test_subscript_dynamic_path() {
        if let Expr::Path {
            start: PathStart::Ident(root),
            steps,
        } = parse("a.[b]")
        {
            assert_eq!(root, "a");
            if let PathStep::Subscript(key_expr) = &steps[0] {
                assert!(
                    matches!(key_expr.as_ref(), Expr::Path { start: PathStart::Ident(s), .. } if s == "b")
                );
            } else {
                panic!("expected Subscript step");
            }
        } else {
            panic!("expected Path");
        }
    }

    #[test]
    fn test_subscript_dynamic_dotted_path() {
        if let Expr::Path {
            start: PathStart::Ident(root),
            steps,
        } = parse("a.[config.key_name]")
        {
            assert_eq!(root, "a");
            if let PathStep::Subscript(key_expr) = &steps[0] {
                if let Expr::Path {
                    start: PathStart::Ident(root),
                    steps,
                } = key_expr.as_ref()
                {
                    assert_eq!(root, "config");
                    assert_eq!(steps.len(), 1);
                    assert!(matches!(&steps[0], PathStep::Dot(k) if k == "key_name"));
                } else {
                    panic!("expected Path inside Subscript");
                }
            } else {
                panic!("expected Subscript step");
            }
        } else {
            panic!("expected Path");
        }
    }

    #[test]
    fn test_mixed_dot_and_subscript() {
        if let Expr::Path {
            start: PathStart::Ident(root),
            steps,
        } = parse("project.[\"010_intro\"].meta.title")
        {
            assert_eq!(root, "project");
            assert_eq!(steps.len(), 3);
            assert!(matches!(&steps[0], PathStep::Subscript(_)));
            assert!(matches!(&steps[1], PathStep::Dot(k) if k == "meta"));
            assert!(matches!(&steps[2], PathStep::Dot(k) if k == "title"));
        } else {
            panic!("expected Path");
        }
    }

    #[test]
    fn test_subscript_single_quote() {
        if let Expr::Path { steps, .. } = parse("x.['key']") {
            if let PathStep::Subscript(key_expr) = &steps[0] {
                assert!(matches!(key_expr.as_ref(), Expr::Literal(Value::Str(s)) if s == "key"));
            } else {
                panic!("expected Subscript step");
            }
        } else {
            panic!("expected Path");
        }
    }

    #[test]
    fn test_subscript_display_literal() {
        assert_eq!(
            parse("project.[\"meta\"].title").to_string(),
            "project.[\"meta\"].title"
        );
    }

    #[test]
    fn test_subscript_display_dynamic() {
        assert_eq!(parse("a.[b]").to_string(), "a.[b]");
        assert_eq!(parse("a.[config.key]").to_string(), "a.[config.key]");
    }

    #[test]
    fn test_subscript_integer_key_parses_ok() {
        // a.[42] is valid at parse time — key is Expr::Literal(Int(42))
        assert!(parse_expr("a.[42]").is_ok());
    }

    #[test]
    fn test_error_subscript_missing_bracket() {
        assert!(parse_expr("project.[\"key\"").is_err());
    }

    #[test]
    fn test_subscript_empty_string_key() {
        if let Expr::Path { steps, .. } = parse("x.[\"\"]") {
            if let PathStep::Subscript(key_expr) = &steps[0] {
                assert!(matches!(key_expr.as_ref(), Expr::Literal(Value::Str(s)) if s.is_empty()));
            } else {
                panic!("expected Subscript step");
            }
        } else {
            panic!("expected Path");
        }
    }

    #[test]
    fn test_path_simple_struct() {
        if let Expr::Path {
            start: PathStart::Ident(root),
            steps,
        } = parse("name")
        {
            assert_eq!(root, "name");
            assert!(steps.is_empty());
        } else {
            panic!("expected Path");
        }
    }

    #[test]
    fn test_path_dotted_struct() {
        if let Expr::Path {
            start: PathStart::Ident(root),
            steps,
        } = parse("user.address.city")
        {
            assert_eq!(root, "user");
            assert_eq!(steps.len(), 2);
            assert!(matches!(&steps[0], PathStep::Dot(k) if k == "address"));
            assert!(matches!(&steps[1], PathStep::Dot(k) if k == "city"));
        } else {
            panic!("expected Path");
        }
    }

    // --- Null-coalescing operator ---

    #[test]
    fn test_null_coalesce_parse() {
        if let Expr::BinOp {
            op: BinOp::NullCoalesce,
            lhs,
            rhs,
        } = parse("a ?? b")
        {
            assert!(matches!(*lhs, Expr::Path { .. }));
            assert!(matches!(*rhs, Expr::Path { .. }));
        } else {
            panic!("expected NullCoalesce BinOp");
        }
    }

    #[test]
    fn test_null_coalesce_with_literal() {
        assert!(matches!(
            parse("x ?? \"default\""),
            Expr::BinOp {
                op: BinOp::NullCoalesce,
                ..
            }
        ));
    }

    #[test]
    fn test_null_coalesce_precedence_lower_than_comparison() {
        // a == b ?? c  →  (a == b) ?? c
        if let Expr::BinOp {
            op: BinOp::NullCoalesce,
            lhs,
            ..
        } = parse("a == b ?? c")
        {
            assert!(matches!(*lhs, Expr::BinOp { op: BinOp::Eq, .. }));
        } else {
            panic!("expected NullCoalesce at top, Eq inside lhs");
        }
    }

    #[test]
    fn test_null_coalesce_display() {
        assert_eq!(parse("x ?? \"fallback\"").to_string(), "x ?? \"fallback\"");
    }

    // --- Null-safe access operators ---

    #[test]
    fn test_null_safe_dot_parse() {
        if let Expr::Path {
            start: PathStart::Ident(root),
            steps,
        } = parse("a.?b")
        {
            assert_eq!(root, "a");
            assert_eq!(steps.len(), 1);
            assert!(matches!(&steps[0], PathStep::NullDot(k) if k == "b"));
        } else {
            panic!("expected Path");
        }
    }

    #[test]
    fn test_null_safe_subscript_literal() {
        // project.?["010_intro"] — null-safe subscript with string literal key
        if let Expr::Path {
            start: PathStart::Ident(root),
            steps,
        } = parse("project.?[\"010_intro\"]")
        {
            assert_eq!(root, "project");
            if let PathStep::NullSubscript(key_expr) = &steps[0] {
                assert!(
                    matches!(key_expr.as_ref(), Expr::Literal(Value::Str(s)) if s == "010_intro")
                );
            } else {
                panic!("expected NullSubscript step");
            }
        } else {
            panic!("expected Path");
        }
    }

    #[test]
    fn test_null_safe_subscript_dynamic() {
        // project.?[key_var] — null-safe subscript with dynamic key
        if let Expr::Path {
            start: PathStart::Ident(root),
            steps,
        } = parse("project.?[key_var]")
        {
            assert_eq!(root, "project");
            if let PathStep::NullSubscript(key_expr) = &steps[0] {
                assert!(
                    matches!(key_expr.as_ref(), Expr::Path { start: PathStart::Ident(s), .. } if s == "key_var")
                );
            } else {
                panic!("expected NullSubscript step");
            }
        } else {
            panic!("expected Path");
        }
    }

    #[test]
    fn test_null_safe_chained() {
        if let Expr::Path {
            start: PathStart::Ident(root),
            steps,
        } = parse("project.?meta.?title")
        {
            assert_eq!(root, "project");
            assert_eq!(steps.len(), 2);
            assert!(matches!(&steps[0], PathStep::NullDot(k) if k == "meta"));
            assert!(matches!(&steps[1], PathStep::NullDot(k) if k == "title"));
        } else {
            panic!("expected Path");
        }
    }

    #[test]
    fn test_null_safe_mixed_with_subscript_and_coalesce() {
        // project.?["010_intro"].?meta.title ?? "Untitled"
        if let Expr::BinOp {
            op: BinOp::NullCoalesce,
            lhs,
            rhs,
        } = parse("project.?[\"010_intro\"].?meta.title ?? \"Untitled\"")
        {
            if let Expr::Path {
                start: PathStart::Ident(root),
                steps,
            } = *lhs
            {
                assert_eq!(root, "project");
                assert_eq!(steps.len(), 3);
                assert!(matches!(&steps[0], PathStep::NullSubscript(_)));
                assert!(matches!(&steps[1], PathStep::NullDot(k) if k == "meta"));
                assert!(matches!(&steps[2], PathStep::Dot(k) if k == "title"));
            } else {
                panic!("expected Path as lhs");
            }
            assert!(matches!(*rhs, Expr::Literal(Value::Str(_))));
        } else {
            panic!("expected NullCoalesce at top level");
        }
    }

    #[test]
    fn test_null_safe_dot_display() {
        assert_eq!(parse("a.?b").to_string(), "a.?b");
    }

    #[test]
    fn test_null_safe_subscript_display_literal() {
        assert_eq!(
            parse("project.?[\"key\"]").to_string(),
            "project.?[\"key\"]"
        );
    }

    #[test]
    fn test_null_safe_subscript_display_dynamic() {
        assert_eq!(
            parse("project.?[key_var]").to_string(),
            "project.?[key_var]"
        );
    }

    // --- New root-start forms ---

    #[test]
    fn test_root_subscript_literal() {
        if let Expr::Path {
            start: PathStart::Subscript(key_expr),
            steps,
        } = parse(".[\"010_intro\"]")
        {
            assert!(matches!(key_expr.as_ref(), Expr::Literal(Value::Str(s)) if s == "010_intro"));
            assert!(steps.is_empty());
        } else {
            panic!("expected Path with Subscript start");
        }
    }

    #[test]
    fn test_root_subscript_dynamic() {
        if let Expr::Path {
            start: PathStart::Subscript(key_expr),
            steps,
        } = parse(".[key]")
        {
            assert!(
                matches!(key_expr.as_ref(), Expr::Path { start: PathStart::Ident(s), .. } if s == "key")
            );
            assert!(steps.is_empty());
        } else {
            panic!("expected Path with Subscript start");
        }
    }

    #[test]
    fn test_root_null_safe_ident() {
        if let Expr::Path {
            start: PathStart::NullSafeIdent(s),
            steps,
        } = parse(".?title")
        {
            assert_eq!(s, "title");
            assert!(steps.is_empty());
        } else {
            panic!("expected Path with NullSafeIdent start");
        }
    }

    #[test]
    fn test_root_null_safe_subscript() {
        if let Expr::Path {
            start: PathStart::NullSafeSubscript(key_expr),
            steps,
        } = parse(".?[\"key\"]")
        {
            assert!(matches!(key_expr.as_ref(), Expr::Literal(Value::Str(s)) if s == "key"));
            assert!(steps.is_empty());
        } else {
            panic!("expected Path with NullSafeSubscript start");
        }
    }

    #[test]
    fn test_dot_foo_same_ast_as_foo() {
        let with_dot = parse(".foo");
        let without_dot = parse("foo");
        assert_eq!(with_dot.to_string(), without_dot.to_string());
        if let (
            Expr::Path {
                start: PathStart::Ident(a),
                steps: steps_a,
            },
            Expr::Path {
                start: PathStart::Ident(b),
                steps: steps_b,
            },
        ) = (&with_dot, &without_dot)
        {
            assert_eq!(a, b);
            assert_eq!(steps_a.len(), steps_b.len());
        } else {
            panic!("expected both to be Path with Ident start");
        }
    }

    #[test]
    fn test_leading_dot_is_valid() {
        assert!(parse_expr(".foo").is_ok());
        assert!(parse_expr(".[\"key\"]").is_ok());
        assert!(parse_expr(".?title").is_ok());
        assert!(parse_expr(".?[\"key\"]").is_ok());
    }

    #[test]
    fn test_root_subscript_display() {
        assert_eq!(parse(".[\"key\"]").to_string(), ".[\"key\"]");
    }

    #[test]
    fn test_root_null_safe_ident_display() {
        assert_eq!(parse(".?foo").to_string(), ".?foo");
    }

    #[test]
    fn test_root_null_safe_subscript_display() {
        assert_eq!(parse(".?[\"key\"]").to_string(), ".?[\"key\"]");
    }
}
