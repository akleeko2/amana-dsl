// src/codegen/express/static_files/default_hooks.rs
#![allow(dead_code)]

pub(crate) fn default_hooks_js() -> &'static str {
    r#"// Amana Custom Hooks
// This file is NOT overwritten on recompilation. Add your custom middlewares or route controllers here.
module.exports = {
  // beforeAll: (req, res, next) => { console.log(`Custom log: ${req.method} ${req.url}`); next(); }
};"#
}
