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

/// Convenience macro for building map contexts inline.
/// Usage: context!{ "name" => Value::Str("Alice".into()), "age" => Value::Int(30) }
#[macro_export]
macro_rules! context {
    () => {{
        let map: std::collections::HashMap<&'static str, $crate::Value> =
            std::collections::HashMap::new();
        std::sync::Arc::new(map) as std::sync::Arc<dyn $crate::DataSource>
    }};
    ($($key:expr => $val:expr),* $(,)?) => {{
        let mut map: std::collections::HashMap<&'static str, $crate::Value> =
            std::collections::HashMap::new();
        $(map.insert($key, $val);)*
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
