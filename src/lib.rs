#[cfg(test)]
pub mod integration;
pub mod lexer;
pub mod parser;
pub mod renderer;
pub mod value;

pub use parser::Template;
pub use renderer::render;
pub use value::DataSource;
pub use value::Value;

#[macro_use]
pub mod macros {
    pub use crate::ctx;
    pub use crate::context;
}
