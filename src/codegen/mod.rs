// src/codegen/mod.rs
//! Codegen module for producing production-ready SQL database schemas,
//! HTML/EJS dynamic layout templates, and Express.js web applications.

use crate::semantic::ir::AmanaIR;

pub mod express;
pub mod html;
pub mod sql;

/// A backend target that can turn Amana IR into a runnable project.
///
/// Keeping this boundary small makes adding new targets straightforward:
/// a backend receives validated target-independent IR and owns every
/// framework-specific file it emits.
pub trait CodegenBackend {
    fn name(&self) -> &'static str;
    fn generate(&self, dest_dir: &str, ir: &AmanaIR) -> Result<(), String>;
}

pub struct ExpressNodeBackend;

impl CodegenBackend for ExpressNodeBackend {
    fn name(&self) -> &'static str {
        "express-node"
    }

    fn generate(&self, dest_dir: &str, ir: &AmanaIR) -> Result<(), String> {
        express::generate_project(dest_dir, ir)
    }
}

pub fn default_backend() -> ExpressNodeBackend {
    ExpressNodeBackend
}

pub fn available_backends() -> &'static [&'static str] {
    &["express-node"]
}

pub fn generate_project(dest_dir: &str, ir: &AmanaIR) -> Result<(), String> {
    default_backend().generate(dest_dir, ir)
}
