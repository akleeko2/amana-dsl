// src/codegen/express/static_files/app.rs
#![allow(dead_code)]

pub(crate) fn app_js() -> &'static str {
    r#"const AmanaEngine = require('./runtime/engine');
const path = require('path');

const irPath = path.join(__dirname, 'amana_ir.json');
const engine = new AmanaEngine(irPath);
engine.start().catch(err => {
  console.error('[Amana Engine Startup Error]', err);
  process.exit(1);
});
"#
}
