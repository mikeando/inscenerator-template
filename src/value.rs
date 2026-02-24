use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// The value type used throughout the template engine.
#[derive(Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Arc<Vec<Value>>),
    /// An owned, transparent map of key→value pairs.
    /// Structural equality; cheap to construct and return from functions.
    Map(HashMap<String, Value>),
    /// An opaque Rust data source. Exists so large Rust structs can be used
    /// as template contexts without copying their data into `Value`s up front.
    DataSource(Arc<dyn DataSource>),
    /// A callable value.
    Fn(Arc<dyn Function>),
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "Null"),
            Value::Bool(b) => write!(f, "Bool({:?})", b),
            Value::Int(n) => write!(f, "Int({:?})", n),
            Value::Float(n) => write!(f, "Float({:?})", n),
            Value::Str(s) => write!(f, "Str({:?})", s),
            Value::List(l) => write!(f, "List({:?})", l),
            Value::Map(m) => write!(f, "Map({:?})", m),
            Value::DataSource(s) => write!(f, "DataSource({:?})", s),
            Value::Fn(_) => write!(f, "Fn(<function>)"),
        }
    }
}

impl Value {
    /// Render to a string for output.
    pub fn render(&self) -> String {
        match self {
            Value::Null => String::new(),
            Value::Bool(b) => b.to_string(),
            Value::Int(n) => n.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Str(s) => s.clone(),
            Value::List(l) => {
                let items: Vec<_> = l.iter().map(|v| v.render()).collect();
                items.join(", ")
            }
            Value::Map(_) | Value::DataSource(_) => "[object]".to_string(),
            Value::Fn(_) => "<function>".to_string(),
        }
    }

    /// Navigate a dotted path like "user.address.city".
    /// Returns None if any segment along the path is missing.
    pub fn get_path(&self, path: &str) -> Option<Value> {
        let mut current = self.clone();
        for segment in path.split('.') {
            current = match &current {
                Value::Map(map) => map.get(segment).cloned()?,
                Value::DataSource(src) => src.get(segment)?,
                _ => return None,
            };
        }
        Some(current)
    }
}

/// Implement this trait to make any type usable as template context.
/// Return `Some(value)` when the key exists, `None` when it does not.
/// Use `Some(Value::Null)` to represent a key that is explicitly null.
pub trait DataSource: Send + Sync + std::fmt::Debug {
    fn get(&self, key: &str) -> Option<Value>;
}

/// Implement this trait to register a callable as a template function.
/// The blanket impl allows `Fn(&[Value]) -> Result<Value, String>` closures to be used directly.
pub trait Function: Send + Sync {
    fn call(&self, args: &[Value]) -> Result<Value, String>;
}

impl<F> Function for F
where
    F: Fn(&[Value]) -> Result<Value, String> + Send + Sync,
{
    fn call(&self, args: &[Value]) -> Result<Value, String> {
        self(args)
    }
}

// --- Blanket implementations for common types ---

impl DataSource for HashMap<String, Value> {
    fn get(&self, key: &str) -> Option<Value> {
        self.get(key).cloned()
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
    // Helper: nested empty map → Value::Map
    (@val { }) => {
        $crate::Value::Map(std::collections::HashMap::new())
    };
    // Helper: nested map with entries → Value::Map
    (@val { $($k:tt : $v:tt),* $(,)? }) => {{
        #[allow(unused_mut)]
        let mut map: std::collections::HashMap<String, $crate::Value> =
            std::collections::HashMap::new();
        $(
            map.insert($k.to_string(), $crate::ctx!(@val $v));
        )*
        $crate::Value::Map(map)
    }};
    // Helper: list → Value::List
    (@val [ $($v:tt),* $(,)? ]) => {
        $crate::Value::List(std::sync::Arc::new(vec![ $($crate::ctx!(@val $v)),* ]))
    };
    // Helper: any other expression → Value::from
    (@val $v:expr) => {
        $crate::Value::from($v)
    };

    // Root: empty map → Arc<dyn DataSource>
    () => {{
        let map: std::collections::HashMap<String, $crate::Value> =
            std::collections::HashMap::new();
        std::sync::Arc::new(map) as std::sync::Arc<dyn $crate::DataSource>
    }};

    // Root: map with entries → Arc<dyn DataSource>
    ($($k:tt : $v:tt),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut map: std::collections::HashMap<String, $crate::Value> =
            std::collections::HashMap::new();
        $(
            map.insert($k.to_string(), $crate::ctx!(@val $v));
        )*
        std::sync::Arc::new(map) as std::sync::Arc<dyn $crate::DataSource>
    }};
}

/// Backward-compatible alias for ctx! using => syntax.
#[macro_export]
macro_rules! context {
    ($($key:expr => $val:expr),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut map: std::collections::HashMap<String, $crate::Value> =
            std::collections::HashMap::new();
        $(map.insert($key.to_string(), $crate::Value::from($val));)*
        std::sync::Arc::new(map) as std::sync::Arc<dyn $crate::DataSource>
    }};
    () => {{
        let map: std::collections::HashMap<String, $crate::Value> =
            std::collections::HashMap::new();
        std::sync::Arc::new(map) as std::sync::Arc<dyn $crate::DataSource>
    }};
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::List(a), Value::List(b)) => a == b,
            (Value::Map(a), Value::Map(b)) => a == b,
            // Opaque sources are compared by identity (same Arc allocation).
            (Value::DataSource(a), Value::DataSource(b)) => Arc::ptr_eq(a, b),
            // Functions are equal only if they are the same Arc allocation.
            (Value::Fn(a), Value::Fn(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::Str(s.to_string())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::Str(s)
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

impl From<HashMap<String, Value>> for Value {
    fn from(m: HashMap<String, Value>) -> Self {
        Value::Map(m)
    }
}

impl From<Arc<dyn DataSource>> for Value {
    fn from(src: Arc<dyn DataSource>) -> Self {
        Value::DataSource(src)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(Value::DataSource(ctx!()).render(), "[object]");
        assert_eq!(
            Value::Map(std::collections::HashMap::new()).render(),
            "[object]"
        );
    }

    #[test]
    fn test_get_path() {
        let inner = ctx! { "a": 1 };
        let outer = ctx! { "inner": inner };
        let val = Value::DataSource(outer);

        assert_eq!(val.get_path("inner.a").unwrap().render(), "1");
        assert!(val.get_path("inner.b").is_none());
        assert!(val.get_path("missing.a").is_none());
        assert!(val.get_path("inner.a.nothing").is_none());
    }

    #[test]
    fn test_get_path_owned_map() {
        let mut inner = std::collections::HashMap::new();
        inner.insert("x".to_string(), Value::Int(42));
        let mut outer = std::collections::HashMap::new();
        outer.insert("inner".to_string(), Value::Map(inner));
        let val = Value::Map(outer);

        assert_eq!(val.get_path("inner.x").unwrap().render(), "42");
        assert!(val.get_path("inner.y").is_none());
    }

    #[test]
    fn test_function_closure() {
        let f = |args: &[Value]| -> Result<Value, String> { Ok(Value::Int(args.len() as i64)) };
        assert_eq!(Function::call(&f, &[]).unwrap().render(), "0");
        assert_eq!(Function::call(&f, &[Value::Int(1)]).unwrap().render(), "1");
        assert!(
            Function::call(&f, &[Value::Bool(false), Value::Null])
                .unwrap()
                .render()
                == "2"
        );
    }

    #[test]
    fn test_function_custom_struct() {
        struct Double;
        impl Function for Double {
            fn call(&self, args: &[Value]) -> Result<Value, String> {
                match args {
                    [Value::Int(n)] => Ok(Value::Int(n * 2)),
                    _ => Err("expected one Int".to_string()),
                }
            }
        }
        assert_eq!(Double.call(&[Value::Int(5)]).unwrap().render(), "10");
        assert!(Double.call(&[]).is_err());
        assert!(Double.call(&[Value::Bool(true)]).is_err());
    }

    #[test]
    fn test_datasource_equality() {
        #[derive(Debug)]
        struct Dummy;
        impl DataSource for Dummy {
            fn get(&self, _: &str) -> Option<Value> {
                None
            }
        }
        let a: Arc<dyn DataSource> = Arc::new(Dummy);
        let b = Arc::clone(&a);
        let c: Arc<dyn DataSource> = Arc::new(Dummy);
        assert_eq!(Value::DataSource(Arc::clone(&a)), Value::DataSource(b));
        assert_ne!(Value::DataSource(a), Value::DataSource(c));
    }

    #[test]
    fn test_fn_equality() {
        let f: Arc<dyn Function> = Arc::new(|_args: &[Value]| Ok(Value::Null));
        let f2 = Arc::clone(&f);
        let g: Arc<dyn Function> = Arc::new(|_args: &[Value]| Ok(Value::Null));
        assert_eq!(Value::Fn(Arc::clone(&f)), Value::Fn(f2));
        assert_ne!(Value::Fn(f), Value::Fn(g));
    }

    #[test]
    fn test_map_equality() {
        let mut a = std::collections::HashMap::new();
        a.insert("x".to_string(), Value::Int(1));
        let mut b = std::collections::HashMap::new();
        b.insert("x".to_string(), Value::Int(1));
        let mut c = std::collections::HashMap::new();
        c.insert("x".to_string(), Value::Int(2));
        assert_eq!(Value::Map(a.clone()), Value::Map(b));
        assert_ne!(Value::Map(a), Value::Map(c));
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
