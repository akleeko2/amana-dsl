const AmanaEngine = require('./runtime/engine');
const path = require('path');

const irPath = path.join(__dirname, 'amana_ir.json');
const engine = new AmanaEngine(irPath);
engine.start().catch(err => {
  console.error('[Amana Engine Startup Error]', err);
  process.exit(1);
});
