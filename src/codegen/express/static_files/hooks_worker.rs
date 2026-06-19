// src/codegen/express/static_files/hooks_worker.rs

pub(crate) fn hooks_worker_js() -> &'static str {
    r#"const fs = require('fs');
const vm = require('vm');
const path = require('path');

let sandboxHook = null;

try {
  const hookFilePath = path.resolve(__dirname, '../custom/hooks.js');
  if (fs.existsSync(hookFilePath)) {
    const hookCode = fs.readFileSync(hookFilePath, 'utf8');
    const scriptCode = `
      (function() {
        const exports = {};
        const module = { exports };
        ${hookCode}
        return module.exports;
      })()
    `;
    const script = new vm.Script(scriptCode, { filename: 'hooks.js' });
    const context = vm.createContext({
      console: {
        log: (...args) => console.log('[Sandbox Log]', ...args),
        error: (...args) => console.error('[Sandbox Error]', ...args)
      }
    });
    const exportsObj = script.runInContext(context, { timeout: 1000 });
    if (exportsObj && typeof exportsObj.beforeAll === 'function') {
      sandboxHook = exportsObj.beforeAll;
    }
  }
} catch (e) {
  console.error('[Amana Sandbox Compile Error]', e);
}

function validateExecuteRequest(msg) {
  if (!msg || typeof msg !== 'object') return false;
  if (msg.type !== 'EXECUTE_HOOK') return false;
  if (typeof msg.reqId !== 'number' || msg.reqId <= 0) return false;
  if (!msg.req || typeof msg.req !== 'object') return false;
  
  const req = msg.req;
  if (typeof req.method !== 'string') return false;
  if (typeof req.url !== 'string') return false;
  if (typeof req.headers !== 'object' || req.headers === null) return false;
  if (typeof req.body !== 'object' || req.body === null) return false;
  if (typeof req.query !== 'object' || req.query === null) return false;
  if (typeof req.params !== 'object' || req.params === null) return false;
  
  return true;
}

process.on('message', async (msg) => {
  if (!validateExecuteRequest(msg)) {
    console.warn('[Security Warning] IPC Request contract violation - message discarded:', msg);
    return;
  }

  const { reqId, req } = msg;
  if (!sandboxHook) {
    process.send({ type: 'HOOK_RESPONSE', reqId, action: 'next' });
    return;
  }

  try {
    let sent = false;
    const safeReq = {
      method: req.method,
      url: req.url,
      headers: { ...req.headers },
      query: { ...req.query },
      body: { ...req.body },
      params: { ...req.params }
    };

    const safeRes = {
      status: (code) => {
        safeRes.statusCode = code;
        return safeRes;
      },
      send: (body) => {
        if (!sent) {
          sent = true;
          process.send({
            type: 'HOOK_RESPONSE',
            reqId,
            action: 'send',
            status: safeRes.statusCode || 200,
            body
          });
        }
      },
      redirect: (url) => {
        if (!sent) {
          sent = true;
          process.send({
            type: 'HOOK_RESPONSE',
            reqId,
            action: 'redirect',
            url
          });
        }
      }
    };

    const result = sandboxHook(safeReq, safeRes, (err) => {
      if (err) {
        console.error('[Amana Hook Error] beforeAll passed an error:', err);
        process.send({ type: 'HOOK_RESPONSE', reqId, action: 'error', error: err.toString() });
      } else if (!sent) {
        process.send({ type: 'HOOK_RESPONSE', reqId, action: 'next' });
      }
    });

    if (result instanceof Promise) {
      await result;
    }
  } catch (err) {
    console.error('[Amana Hook Exception] beforeAll crashed:', err);
    process.send({ type: 'HOOK_RESPONSE', reqId, action: 'crash', error: err.toString() });
  }
});"#
}
