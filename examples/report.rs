//! Employee performance report.
//!
//! Demonstrates:
//! - Implementing `DataSource` on custom Rust structs — zero-copy, no serialization
//! - Building a `Value::List` from typed struct values
//! - Fragments for reusable row rendering
//! - Comparison operators for tiered output (`>=`, `==`)
//! - The `len()` built-in function
//! - Custom function registration
//!
//! Run with: cargo run --example report

use inscenerator_template::{DataSource, Template, Value, render};
use std::sync::Arc;

// --- Domain types ---------------------------------------------------------

#[derive(Debug)]
struct Employee {
    name: &'static str,
    department: &'static str,
    years: i64,
    score: i64, // 0–100
}

impl DataSource for Employee {
    fn get(&self, key: &str) -> Option<Value> {
        match key {
            "name" => Some(Value::from(self.name)),
            "department" => Some(Value::from(self.department)),
            "years" => Some(Value::Int(self.years)),
            "score" => Some(Value::Int(self.score)),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct Report {
    year: i64,
    employees: Vec<Arc<Employee>>,
}

impl DataSource for Report {
    fn get(&self, key: &str) -> Option<Value> {
        match key {
            "year" => Some(Value::Int(self.year)),
            "employees" => Some(Value::List(Arc::new(
                self.employees
                    .iter()
                    .map(|e| Value::DataSource(Arc::clone(e) as Arc<dyn DataSource>))
                    .collect(),
            ))),
            _ => None,
        }
    }
}

// --- Templates ------------------------------------------------------------

const REPORT: &str = "\
Annual Review {{ year }}
========================
{% for emp in employees %}{{ Row emp }}
{% endfor %}
{{ len(employees) }} employee(s) reviewed.
";

// The fragment receives each Employee as its context, so fields are accessed directly.
const ROW: &str = "\
{{ tier(score) }} {{ name }} ({{ department }}, {{ years }}yr) \
— score {{ score }}/100 \
{% if score >= 90 %}— Exceeds expectations\
{% elif score >= 70 %}— Meets expectations\
{% else %}— Needs improvement\
{% endif %}";

// --- Entry point ----------------------------------------------------------

fn main() {
    let mut tmpl = Template::parse(REPORT).unwrap();
    tmpl.add_fragment("Row", ROW).unwrap();

    // Custom function: return a tier badge based on the numeric score.
    tmpl.add_function("tier", |args: &[Value]| match args {
        [Value::Int(n)] => Ok(Value::from(if *n >= 90 {
            "[★]"
        } else if *n >= 70 {
            "[+]"
        } else {
            "[-]"
        })),
        _ => Err("tier() expects one Int argument".to_string()),
    });

    let data = Report {
        year: 2026,
        employees: vec![
            Arc::new(Employee {
                name: "Alice",
                department: "Engineering",
                years: 5,
                score: 94,
            }),
            Arc::new(Employee {
                name: "Bob",
                department: "Design",
                years: 2,
                score: 78,
            }),
            Arc::new(Employee {
                name: "Carol",
                department: "Engineering",
                years: 8,
                score: 91,
            }),
            Arc::new(Employee {
                name: "Dave",
                department: "Support",
                years: 1,
                score: 65,
            }),
            Arc::new(Employee {
                name: "Evelyn",
                department: "Design",
                years: 3,
                score: 83,
            }),
        ],
    };

    let output = render(&tmpl, &data).unwrap();
    print!("{}", output);
}
