use crate::{DataSource, Template, Value, context, render};
use std::sync::Arc;

// --- A real struct implementing DataSource ---

#[derive(Debug)]
struct User {
    name: &'static str,
    age: i64,
    active: bool,
}

impl DataSource for User {
    fn get(&self, key: &str) -> Value {
        match key {
            "name" => Value::from(self.name),
            "age" => Value::Int(self.age),
            "active" => Value::Bool(self.active),
            _ => Value::Null,
        }
    }
}

#[test]
fn test_simple_output() {
    let tmpl = Template::parse("Hello, {{ name }}!").unwrap();
    let ctx = context! { "name" => Value::from("World") };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "Hello, World!");
}

#[test]
fn test_struct_datasource() {
    let tmpl = Template::parse("{{ name }} is {{ age }} years old.").unwrap();
    let user = User {
        name: "Alice",
        age: 30,
        active: true,
    };
    assert_eq!(render(&tmpl, &user).unwrap(), "Alice is 30 years old.");
}

#[test]
fn test_if_else() {
    let src = "{% if active %}on{% else %}off{% endif %}";
    let tmpl = Template::parse(src).unwrap();

    let on = User {
        name: "Alice",
        age: 30,
        active: true,
    };
    let off = User {
        name: "Bob",
        age: 25,
        active: false,
    };

    assert_eq!(render(&tmpl, &on).unwrap(), "on");
    assert_eq!(render(&tmpl, &off).unwrap(), "off");
}

#[test]
fn test_elif() {
    let src = "{% if a %}A{% elif b %}B{% else %}C{% endif %}";
    let tmpl = Template::parse(src).unwrap();

    let ctx_a = context! { "a" => Value::Bool(true),  "b" => Value::Bool(false) };
    let ctx_b = context! { "a" => Value::Bool(false), "b" => Value::Bool(true) };
    let ctx_c = context! { "a" => Value::Bool(false), "b" => Value::Bool(false) };

    assert_eq!(render(&tmpl, ctx_a.as_ref()).unwrap(), "A");
    assert_eq!(render(&tmpl, ctx_b.as_ref()).unwrap(), "B");
    assert_eq!(render(&tmpl, ctx_c.as_ref()).unwrap(), "C");
}

#[test]
fn test_for_loop() {
    let src = "{% for item in items %}[{{ item }}]{% endfor %}";
    let tmpl = Template::parse(src).unwrap();
    let ctx = context! {
        "items" => Value::from(vec![
            Value::from("a"),
            Value::from("b"),
            Value::from("c"),
        ])
    };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "[a][b][c]");
}

#[test]
fn test_nested_map_in_loop() {
    // Each item in the list is itself a Map (DataSource)
    #[derive(Debug)]
    struct Item {
        label: &'static str,
    }
    impl DataSource for Item {
        fn get(&self, key: &str) -> Value {
            match key {
                "label" => Value::from(self.label),
                _ => Value::Null,
            }
        }
    }

    let items = Value::List(Arc::new(vec![
        Value::Map(Arc::new(Item { label: "foo" })),
        Value::Map(Arc::new(Item { label: "bar" })),
    ]));

    let src = "{% for it in items %}{{ it.label }} {% endfor %}";
    let tmpl = Template::parse(src).unwrap();
    let ctx = context! { "items" => items };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "foo bar ");
}

#[test]
fn test_fragment_call_with_map_context() {
    // A fragment is called with {{ FragmentName expr }} where expr resolves to a Map.
    // The fragment's variables resolve against that map, not the outer context.
    #[derive(Debug)]
    struct Product {
        name: &'static str,
        price: i64,
    }
    impl DataSource for Product {
        fn get(&self, key: &str) -> Value {
            match key {
                "name" => Value::from(self.name),
                "price" => Value::Int(self.price),
                _ => Value::Null,
            }
        }
    }

    let mut tmpl = Template::parse("{{ ProductView product }}").unwrap();
    tmpl.add_fragment("ProductView", "{{ name }}: ${{ price }}")
        .unwrap();

    let ctx = context! {
        "product" => Value::Map(Arc::new(Product { name: "Widget", price: 9 }))
    };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "Widget: $9");
}

#[test]
fn test_fragment_call_in_loop() {
    // The canonical use case: each loop item is passed to a fragment as its context.
    #[derive(Debug)]
    struct Item {
        label: &'static str,
    }
    impl DataSource for Item {
        fn get(&self, key: &str) -> Value {
            match key {
                "label" => Value::from(self.label),
                _ => Value::Null,
            }
        }
    }

    let items = Value::List(Arc::new(vec![
        Value::Map(Arc::new(Item { label: "foo" })),
        Value::Map(Arc::new(Item { label: "bar" })),
    ]));

    let mut tmpl = Template::parse("{% for it in items %}{{ Row it }}{% endfor %}").unwrap();
    tmpl.add_fragment("Row", "[{{ label }}]").unwrap();

    let ctx = context! { "items" => items };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "[foo][bar]");
}

#[test]
fn test_fragment_unknown_errors() {
    let _tmpl = Template::parse("{{ Missing thing }}").unwrap();
    let _ctx = context! {
        "thing" => Value::Map(context! {})
    };
    // Can't easily use Value::Map(context!{}) directly here, so test via a known-missing name
    let tmpl2 = Template::parse("{{ Ghost x }}").unwrap();
    #[derive(Debug)]
    struct _Dummy;
    impl DataSource for _Dummy {
        fn get(&self, _: &str) -> Value {
            Value::Null
        }
    }
    // context expr resolving to Null should return an error
    let ctx2 = context! {};
    assert!(render(&tmpl2, ctx2.as_ref()).is_err());
}

#[test]
fn test_comment_ignored() {
    let tmpl = Template::parse("a{# this is a comment #}b").unwrap();
    let ctx = context! {};
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "ab");
}

#[test]
fn test_negation() {
    let src = "{% if !flag %}yes{% endif %}";
    let tmpl = Template::parse(src).unwrap();
    let ctx = context! { "flag" => Value::Bool(false) };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "yes");
}

#[test]
fn test_dotted_path() {
    #[derive(Debug)]
    struct Address {
        city: &'static str,
    }
    impl DataSource for Address {
        fn get(&self, key: &str) -> Value {
            match key {
                "city" => Value::from(self.city),
                _ => Value::Null,
            }
        }
    }

    #[derive(Debug)]
    struct Person {
        address: Address,
    }
    impl DataSource for Person {
        fn get(&self, key: &str) -> Value {
            match key {
                "address" => Value::Map(Arc::new(Address {
                    city: self.address.city,
                })),
                _ => Value::Null,
            }
        }
    }

    let tmpl = Template::parse("{{ address.city }}").unwrap();
    let p = Person {
        address: Address { city: "London" },
    };
    assert_eq!(render(&tmpl, &p).unwrap(), "London");
}

#[test]
fn test_complex_nesting() {
    let src = r#"
        {% for category in categories %}
            Category: {{ category.name }}
            {% for product in category.products %}
                - {{ product.name }} ({{ CurrencySymbol product }})
            {% endfor %}
        {% endfor %}
    "#;
    let mut tmpl = Template::parse(src).unwrap();
    tmpl.add_fragment("CurrencySymbol", r#"{% if is_usd %}${% else %}€{% endif %}"#).unwrap();

    let ctx = context! {
        "categories" => Value::from(vec![
            Value::Map(context! {
                "name" => Value::from("Electronics"),
                "products" => Value::from(vec![
                    Value::Map(context! { "name" => Value::from("Phone"), "is_usd" => Value::from(true) }),
                    Value::Map(context! { "name" => Value::from("Laptop"), "is_usd" => Value::from(false) }),
                ])
            })
        ])
    };

    let output = render(&tmpl, ctx.as_ref()).unwrap();
    assert!(output.contains("Category: Electronics"));
    assert!(output.contains("- Phone ($)"));
    assert!(output.contains("- Laptop (€)"));
}

#[test]
fn test_utf8_integration() {
    let src = "你好，{{ name }}！{% if happy %}😊{% endif %}";
    let tmpl = Template::parse(src).unwrap();
    let ctx = context! {
        "name" => Value::from("世界"),
        "happy" => Value::Bool(true)
    };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "你好，世界！😊");
}

#[test]
fn test_missing_variable_renders_empty() {
    let tmpl = Template::parse("Before{{ missing }}After").unwrap();
    let ctx = context! {};
    // Exposing bug: currently missing variables render as empty string,
    // but user wants them to be errors.
    assert!(render(&tmpl, ctx.as_ref()).is_err());
}

#[test]
fn test_whitespace_preservation() {
    let src = "  {% if true %}  A  {% endif %}  ";
    let tmpl = Template::parse(src).unwrap();
    let ctx = context! {};
    // current implementation preserves everything outside tags
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "    A    ");
}

#[test]
fn test_invalid_expression_renders_empty() {
    // Current eval() doesn't support errors, so it just does a lookup of the whole string.
    // This should ideally be a parsing or evaluation error.
    let tmpl = Template::parse("{{ a b c }}").unwrap();
    let ctx = context! {};
    assert!(render(&tmpl, ctx.as_ref()).is_err());
}

#[test]
fn test_utf8_variable_names() {
    let tmpl = Template::parse("{{ 名字 }}").unwrap();
    let ctx = context! { "名字" => Value::from("张三") };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "张三");
}

// #[test]
// fn test_deep_fragment_recursion() {
//     let mut tmpl = Template::parse("{{ Recurse self }}").unwrap();
//     tmpl.add_fragment("Recurse", "{{ Recurse self }}").unwrap();
//     let ctx = context! { "self" => Value::Map(context!{}) };
//     render(&tmpl, ctx.as_ref()).unwrap();
// }
