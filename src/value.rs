use std::collections::HashMap;
use std::sync::Arc;

/// The value type returned by DataSource::get.
/// Arc<str> throughout means no lifetime parameters anywhere.
#[derive(Clone, Debug)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Arc<str>),
    List(Arc<Vec<Value>>),
    Map(Arc<dyn DataSource>),
}

impl Value {
    /// Truthiness for use in conditionals.
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::Str(s) => !s.is_empty(),
            Value::List(l) => !l.is_empty(),
            Value::Map(_) => true,
        }
    }

    /// Render to a string for output.
    pub fn render(&self) -> String {
        match self {
            Value::Null => String::new(),
            Value::Bool(b) => b.to_string(),
            Value::Int(n) => n.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Str(s) => s.to_string(),
            Value::List(l) => {
                let items: Vec<_> = l.iter().map(|v| v.render()).collect();
                items.join(", ")
            }
            Value::Map(_) => "[object]".to_string(),
        }
    }

    /// Navigate a dotted path like "user.address.city"
    pub fn get_path(&self, path: &str) -> Value {
        let mut current = self.clone();
        for segment in path.split('.') {
            current = match &current {
                Value::Map(src) => src.get(segment),
                _ => return Value::Null,
            };
        }
        current
    }
}

/// Implement this trait to make any type usable as template context.
pub trait DataSource: Send + Sync + std::fmt::Debug {
    fn get(&self, key: &str) -> Value;
}

// --- Blanket implementations for common types ---

impl DataSource for HashMap<String, Value> {
    fn get(&self, key: &str) -> Value {
        self.get(key).cloned().unwrap_or(Value::Null)
    }
}

impl DataSource for HashMap<&'static str, Value> {
    fn get(&self, key: &str) -> Value {
        self.get(key).cloned().unwrap_or(Value::Null)
    }
}

/// A powerful macro for building nested map contexts inline.
/// Supports nesting with { ... } and lists with [ ... ].
/// Usage:
/// ctx! {
///     "name": "Alice",
///     "age": 30,
///     "address": { "city": "London" },
///     "tags": ["rust", "template"]
/// }
#[macro_export]
macro_rules! ctx {
    // Helper to convert values, including nested maps and lists
    (@val { }) => {
        $crate::Value::Map($crate::ctx!())
    };
    (@val { $($k:tt : $v:tt),* $(,)? }) => {
        $crate::Value::Map($crate::ctx! { $($k : $v),* })
    };
    (@val [ $($v:tt),* $(,)? ]) => {
        $crate::Value::List(std::sync::Arc::new(vec![ $($crate::ctx!(@val $v)),* ]))
    };
    (@val $v:expr) => {
        $crate::Value::from($v)
    };

    // Empty map
    () => {{
        let map: std::collections::HashMap<&'static str, $crate::Value> =
            std::collections::HashMap::new();
        std::sync::Arc::new(map) as std::sync::Arc<dyn $crate::DataSource>
    }};

    // Map with entries
    ($($k:tt : $v:tt),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut map: std::collections::HashMap<&'static str, $crate::Value> =
            std::collections::HashMap::new();
        $(
            map.insert($k, $crate::ctx!(@val $v));
        )*
        std::sync::Arc::new(map) as std::sync::Arc<dyn $crate::DataSource>
    }};
}

/// Backward-compatible alias for ctx! using => syntax.
#[macro_export]
macro_rules! context {
    ($($key:expr => $val:expr),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut map: std::collections::HashMap<&'static str, $crate::Value> =
            std::collections::HashMap::new();
        $(map.insert($key, $crate::Value::from($val));)*
        std::sync::Arc::new(map) as std::sync::Arc<dyn $crate::DataSource>
    }};
    () => {{
        let map: std::collections::HashMap<&'static str, $crate::Value> =
            std::collections::HashMap::new();
        std::sync::Arc::new(map) as std::sync::Arc<dyn $crate::DataSource>
    }};
}

/// Helper to convert a &str into Value::Str
impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::Str(Arc::from(s))
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::Str(Arc::from(s.as_str()))
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Value::Int(n)
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<Vec<Value>> for Value {
    fn from(v: Vec<Value>) -> Self {
        Value::List(Arc::new(v))
    }
}

impl From<Arc<dyn DataSource>> for Value {
    fn from(src: Arc<dyn DataSource>) -> Self {
        Value::Map(src)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_truthy() {
        assert!(!Value::Null.is_truthy());
        assert!(Value::Bool(true).is_truthy());
        assert!(!Value::Bool(false).is_truthy());
        assert!(Value::Int(1).is_truthy());
        assert!(Value::Int(-1).is_truthy());
        assert!(!Value::Int(0).is_truthy());
        assert!(Value::Float(1.0).is_truthy());
        assert!(Value::Float(-0.5).is_truthy());
        assert!(!Value::Float(0.0).is_truthy());
        assert!(Value::from("abc").is_truthy());
        assert!(!Value::from("").is_truthy());
        assert!(Value::from(vec![Value::Int(1)]).is_truthy());
        assert!(!Value::from(vec![]).is_truthy());
        assert!(Value::Map(ctx!()).is_truthy());
    }

    #[test]
    fn test_render() {
        assert_eq!(Value::Null.render(), "");
        assert_eq!(Value::Bool(true).render(), "true");
        assert_eq!(Value::Bool(false).render(), "false");
        assert_eq!(Value::Int(123).render(), "123");
        assert_eq!(Value::Float(1.23).render(), "1.23");
        assert_eq!(Value::from("hello").render(), "hello");
        assert_eq!(
            Value::from(vec![Value::Int(1), Value::from("two")]).render(),
            "1, two"
        );
        assert_eq!(Value::Map(ctx!()).render(), "[object]");
    }

    #[test]
    fn test_get_path() {
        let inner = ctx! { "a": 1 };
        let outer = ctx! { "inner": inner };
        let val = Value::Map(outer);

        assert_eq!(val.get_path("inner.a").render(), "1");
        assert!(matches!(val.get_path("inner.b"), Value::Null));
        assert!(matches!(val.get_path("missing.a"), Value::Null));
        assert!(matches!(val.get_path("inner.a.nothing"), Value::Null));
    }

    #[test]
    fn test_from_impls() {
        assert!(matches!(Value::from("hi"), Value::Str(_)));
        assert!(matches!(Value::from("hi".to_string()), Value::Str(_)));
        assert!(matches!(Value::from(10i64), Value::Int(10)));
        assert!(matches!(Value::from(1.5f64), Value::Float(_)));
        assert!(matches!(Value::from(true), Value::Bool(true)));
        assert!(matches!(Value::from(vec![]), Value::List(_)));
    }
}
