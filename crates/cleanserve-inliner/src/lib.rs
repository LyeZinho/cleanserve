//! CleanServe Static Environment Inliner
//! 
//! Converts .env files to PHP constants for zero-overhead environment access.

pub mod parser;
pub mod generator;

pub use parser::EnvParser;
pub use generator::PhpGenerator;
