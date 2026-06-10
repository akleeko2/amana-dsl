const http = require('http');
const fs = require('fs');
const fsp = require('fs/promises');
const path = require('path');
const { spawn } = require('child_process');

const studioRoot = __dirname;
const repoRoot = path.resolve(studioRoot, '..');
const publicRoot = path.join(studioRoot, 'public');
const workspaceDir = path.join(studioRoot, 'workspace');
const sourcePath = path.join(workspaceDir, 'app.amana');
const distDir = path.join(studioRoot, '.amana_live_dist');

const studioPort = Number(process.env.PORT || process.env.AMANA_STUDIO_PORT || 4080);
const previewPort = Number(process.env.AMANA_PREVIEW_PORT || 4174);
const cargoBin = process.platform === 'win32' ? 'cargo.exe' : 'cargo';
const npmBin = process.platform === 'win32' ? 'npm.cmd' : 'npm';
const nodeBin = process.platform === 'win32' ? 'node.exe' : 'node';

let previewProcess = null;
let previewReady = false;
const previewLogs = [];
let operationQueue = Promise.resolve();

function pushPreviewLog(chunk) {
  const text = String(chunk || '').trimEnd();
  if (!text) return;
  previewLogs.push(text);
  while (previewLogs.length > 80) previewLogs.shift();
}

async function ensureWorkspace() {
  await fsp.mkdir(workspaceDir, { recursive: true });
}

function sendJson(res, status, payload) {
  const body = JSON.stringify(payload, null, 2);
  res.writeHead(status, {
    'content-type': 'application/json; charset=utf-8',
    'cache-control': 'no-store'
  });
  res.end(body);
}

function sendText(res, status, text, type = 'text/plain; charset=utf-8') {
  res.writeHead(status, {
    'content-type': type,
    'cache-control': 'no-store'
  });
  res.end(text);
}

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function isTransientFsError(error) {
  const code = String(error && error.code || '').toUpperCase();
  const message = String(error && error.message || '').toLowerCase();
  return ['EBUSY', 'EPERM', 'EACCES', 'UNKNOWN'].includes(code)
    || message.includes('unknown error')
    || message.includes('resource busy')
    || message.includes('being used by another process')
    || message.includes('operation not permitted');
}

async function withFsRetry(label, task, attempts = 10) {
  let lastError = null;
  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    try {
      return await task();
    } catch (error) {
      lastError = error;
      if (!isTransientFsError(error) || attempt === attempts) break;
      await delay(Math.min(1000, 80 * attempt));
    }
  }
  const code = lastError && lastError.code ? ` ${lastError.code}` : '';
  throw new Error(`${label} failed after retries:${code} ${lastError ? lastError.message : ''}`.trim());
}

function enqueueOperation(task) {
  const run = operationQueue.then(task, task);
  operationQueue = run.catch(() => {});
  return run;
}

async function readRequestBody(req) {
  const chunks = [];
  for await (const chunk of req) chunks.push(chunk);
  const raw = Buffer.concat(chunks).toString('utf8');
  if (!raw) return {};
  try {
    return JSON.parse(raw);
  } catch {
    return { raw };
  }
}

function parseJsonMaybe(stdout) {
  const text = String(stdout || '').trim();
  if (!text) return null;
  try {
    return JSON.parse(text);
  } catch {
    const start = text.indexOf('{');
    const end = text.lastIndexOf('}');
    if (start >= 0 && end > start) {
      try {
        return JSON.parse(text.slice(start, end + 1));
      } catch {
        return null;
      }
    }
    return null;
  }
}

function runProcess(command, args, options = {}) {
  const timeoutMs = options.timeoutMs || 120000;
  return new Promise((resolve) => {
    const useShell = process.platform === 'win32' && /\.(cmd|bat)$/i.test(command);
    const child = spawn(command, args, {
      cwd: options.cwd || repoRoot,
      env: options.env || process.env,
      shell: useShell,
      windowsHide: true
    });
    let stdout = '';
    let stderr = '';
    const timer = setTimeout(() => {
      child.kill();
      stderr += `\nProcess timed out after ${timeoutMs}ms.`;
    }, timeoutMs);
    child.stdout.on('data', (chunk) => {
      stdout += chunk.toString();
    });
    child.stderr.on('data', (chunk) => {
      stderr += chunk.toString();
    });
    child.on('close', (code) => {
      clearTimeout(timer);
      resolve({ code, stdout, stderr });
    });
    child.on('error', (error) => {
      clearTimeout(timer);
      resolve({ code: 1, stdout, stderr: `${stderr}\n${error.message}` });
    });
  });
}

async function runAmana(args) {
  const result = await runProcess(cargoBin, ['run', '--quiet', '--', ...args], {
    cwd: repoRoot,
    timeoutMs: 180000
  });
  return {
    ...result,
    json: parseJsonMaybe(result.stdout),
    ok: result.code === 0
  };
}

async function maybeSaveSource(body) {
  if (typeof body.source === 'string') {
    await ensureWorkspace();
    await withFsRetry('Saving workspace/app.amana', () => fsp.writeFile(sourcePath, body.source, 'utf8'));
  }
}

async function sourcePayload() {
  await ensureWorkspace();
  const source = await withFsRetry('Reading workspace/app.amana', () => fsp.readFile(sourcePath, 'utf8'));
  const stat = await withFsRetry('Stat workspace/app.amana', () => fsp.stat(sourcePath));
  return {
    path: sourcePath,
    source,
    updatedAt: stat.mtime.toISOString()
  };
}

async function fileExists(file) {
  try {
    await fsp.access(file);
    return true;
  } catch {
    return false;
  }
}

function reusableNodePath() {
  const candidates = [
    path.join(repoRoot, '.amana_tasks_dist', 'node_modules'),
    path.join(repoRoot, '.amana_design_v2_dist', 'node_modules'),
    path.join(repoRoot, '.amana_phase2_dist', 'node_modules')
  ];
  return candidates.filter((candidate) => fs.existsSync(candidate)).join(path.delimiter);
}

async function ensureNodeDependencies() {
  const nodeModules = path.join(distDir, 'node_modules');
  if (await fileExists(nodeModules)) {
    return { skipped: true, reason: 'node_modules already exists' };
  }
  if (process.env.AMANA_LIVE_SKIP_INSTALL === '1') {
    return { skipped: true, reason: 'AMANA_LIVE_SKIP_INSTALL=1' };
  }
  const result = await runProcess(npmBin, ['install'], {
    cwd: distDir,
    timeoutMs: 240000
  });
  return {
    skipped: false,
    ok: result.code === 0,
    stdout: result.stdout,
    stderr: result.stderr
  };
}

async function nodeSyntaxChecks() {
  const files = [
    'app.js',
    path.join('runtime', 'engine.js'),
    path.join('middleware', 'security.js'),
    path.join('middleware', 'hooks-worker.js')
  ];
  const results = [];
  for (const rel of files) {
    const abs = path.join(distDir, rel);
    if (!(await fileExists(abs))) continue;
    const result = await runProcess(nodeBin, ['--check', abs], {
      cwd: distDir,
      timeoutMs: 30000
    });
    results.push({ file: rel, ok: result.code === 0, stdout: result.stdout, stderr: result.stderr });
  }
  return results;
}

function previewHealthRequest(timeoutMs = 1000) {
  return new Promise((resolve) => {
    const req = http.get(`http://127.0.0.1:${previewPort}/`, (res) => {
      res.resume();
      resolve(true);
    });
    req.on('error', () => resolve(false));
    req.setTimeout(timeoutMs, () => {
      req.destroy();
      resolve(false);
    });
  });
}

async function waitForPreviewClosed(timeoutMs = 5000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (!(await previewHealthRequest(500))) return true;
    await delay(150);
  }
  return false;
}

function killProcessTree(pid) {
  if (!pid) return Promise.resolve();
  if (process.platform !== 'win32') {
    return Promise.resolve();
  }
  return new Promise((resolve) => {
    const killer = spawn('taskkill.exe', ['/PID', String(pid), '/T', '/F'], {
      windowsHide: true
    });
    killer.on('close', () => resolve());
    killer.on('error', () => resolve());
  });
}

async function stopPreview() {
  previewReady = false;
  const child = previewProcess;
  previewProcess = null;
  if (child && !child.killed) {
    await new Promise((resolve) => {
      let settled = false;
      const finish = () => {
        if (settled) return;
        settled = true;
        resolve();
      };
      child.once('exit', finish);
      child.kill();
      setTimeout(finish, 1200);
    });
    if (await previewHealthRequest(500)) {
      await killProcessTree(child.pid);
    }
  }
  await waitForPreviewClosed();
}

function waitForPreview(timeoutMs = 15000) {
  const deadline = Date.now() + timeoutMs;
  return new Promise((resolve) => {
    const tick = () => {
      const req = http.get(`http://127.0.0.1:${previewPort}/`, (res) => {
        res.resume();
        if (res.statusCode && res.statusCode < 500) {
          resolve({ ok: true, status: res.statusCode });
        } else if (Date.now() > deadline) {
          resolve({ ok: false, status: res.statusCode || 0 });
        } else {
          setTimeout(tick, 400);
        }
      });
      req.on('error', () => {
        if (Date.now() > deadline) {
          resolve({ ok: false, status: 0 });
        } else {
          setTimeout(tick, 400);
        }
      });
      req.setTimeout(2000, () => {
        req.destroy();
      });
    };
    tick();
  });
}

async function startPreview() {
  await stopPreview();
  previewLogs.length = 0;
  const env = {
    ...process.env,
    PORT: String(previewPort)
  };
  const extraNodePath = reusableNodePath();
  if (extraNodePath) {
    env.NODE_PATH = env.NODE_PATH ? `${env.NODE_PATH}${path.delimiter}${extraNodePath}` : extraNodePath;
  }
  previewProcess = spawn(nodeBin, ['app.js'], {
    cwd: distDir,
    env,
    windowsHide: true
  });
  previewProcess.stdout.on('data', pushPreviewLog);
  previewProcess.stderr.on('data', pushPreviewLog);
  previewProcess.on('exit', (code) => {
    pushPreviewLog(`[preview exited with code ${code}]`);
    previewReady = false;
  });
  const wait = await waitForPreview();
  previewReady = wait.ok;
  return wait;
}

async function buildAndPreview(body) {
  await maybeSaveSource(body);
  const build = await runAmana(['build', sourcePath, distDir, '--json']);
  if (!build.ok) {
    return { build, syntaxChecks: [], dependencyInstall: null, preview: null };
  }
  const syntaxChecks = await nodeSyntaxChecks();
  const syntaxOk = syntaxChecks.every((check) => check.ok);
  if (!syntaxOk) {
    return { build, syntaxChecks, dependencyInstall: null, preview: null };
  }
  const dependencyInstall = await ensureNodeDependencies();
  if (dependencyInstall && dependencyInstall.ok === false) {
    return { build, syntaxChecks, dependencyInstall, preview: null };
  }
  const preview = await startPreview();
  return {
    build,
    syntaxChecks,
    dependencyInstall,
    preview: {
      ...preview,
      port: previewPort,
      directUrl: `http://127.0.0.1:${previewPort}/`,
      proxiedUrl: `/preview/?t=${Date.now()}`
    }
  };
}

async function serveStatic(req, res, pathname) {
  const rel = pathname === '/' ? 'index.html' : pathname.replace(/^\/+/, '');
  const file = path.normalize(path.join(publicRoot, rel));
  if (!file.startsWith(publicRoot)) {
    sendText(res, 403, 'Forbidden');
    return;
  }
  if (!(await fileExists(file))) {
    sendText(res, 404, 'Not found');
    return;
  }
  const ext = path.extname(file).toLowerCase();
  const types = {
    '.html': 'text/html; charset=utf-8',
    '.css': 'text/css; charset=utf-8',
    '.js': 'application/javascript; charset=utf-8',
    '.json': 'application/json; charset=utf-8'
  };
  const content = await fsp.readFile(file);
  res.writeHead(200, {
    'content-type': types[ext] || 'application/octet-stream',
    'cache-control': 'no-store'
  });
  res.end(content);
}

function rewritePreviewHtml(html) {
  return html
    .replace(/(href|src|action)="\/(?!preview\/)/g, '$1="/preview/')
    .replace(/(href|src|action)='\/(?!preview\/)/g, "$1='/preview/");
}

function proxyPreview(req, res, parsedUrl) {
  if (!previewProcess || !previewReady) {
    sendText(res, 503, 'Preview is not running. Build first.');
    return;
  }
  const targetPath = parsedUrl.pathname.replace(/^\/preview/, '') || '/';
  const options = {
    hostname: '127.0.0.1',
    port: previewPort,
    method: req.method,
    path: `${targetPath}${parsedUrl.search || ''}`,
    headers: {
      ...req.headers,
      host: `127.0.0.1:${previewPort}`
    }
  };
  const proxyReq = http.request(options, (proxyRes) => {
    const chunks = [];
    proxyRes.on('data', (chunk) => chunks.push(chunk));
    proxyRes.on('end', () => {
      const headers = { ...proxyRes.headers };
      delete headers['x-frame-options'];
      delete headers['content-security-policy'];
      if (headers.location && String(headers.location).startsWith('/')) {
        headers.location = `/preview${headers.location}`;
      }
      const body = Buffer.concat(chunks);
      const contentType = String(headers['content-type'] || '');
      if (contentType.includes('text/html')) {
        const rewritten = rewritePreviewHtml(body.toString('utf8'));
        headers['content-length'] = Buffer.byteLength(rewritten);
        res.writeHead(proxyRes.statusCode || 200, headers);
        res.end(rewritten);
      } else {
        res.writeHead(proxyRes.statusCode || 200, headers);
        res.end(body);
      }
    });
  });
  proxyReq.on('error', (error) => {
    sendText(res, 502, `Preview proxy error: ${error.message}`);
  });
  req.pipe(proxyReq);
}

async function handleApi(req, res, parsedUrl) {
  try {
    if (req.method === 'GET' && parsedUrl.pathname === '/api/source') {
      sendJson(res, 200, await sourcePayload());
      return;
    }
    if (req.method === 'GET' && parsedUrl.pathname === '/api/status') {
      sendJson(res, 200, {
        studioPort,
        previewPort,
        sourcePath,
        distDir,
        previewRunning: Boolean(previewProcess && !previewProcess.killed),
        previewReady,
        previewUrl: previewReady ? `/preview/?t=${Date.now()}` : null,
        logs: previewLogs
      });
      return;
    }
    const body = await readRequestBody(req);
    if (req.method === 'POST' && parsedUrl.pathname === '/api/source') {
      sendJson(res, 200, await enqueueOperation(async () => {
        await maybeSaveSource(body);
        return sourcePayload();
      }));
      return;
    }
    if (req.method === 'POST' && parsedUrl.pathname === '/api/check') {
      sendJson(res, 200, await enqueueOperation(async () => {
        await maybeSaveSource(body);
        return runAmana(['check', sourcePath, '--json']);
      }));
      return;
    }
    if (req.method === 'POST' && parsedUrl.pathname === '/api/inspect-design') {
      sendJson(res, 200, await enqueueOperation(async () => {
        await maybeSaveSource(body);
        return runAmana(['inspect-design', sourcePath, '--json']);
      }));
      return;
    }
    if (req.method === 'POST' && parsedUrl.pathname === '/api/fmt') {
      sendJson(res, 200, await enqueueOperation(async () => {
        await maybeSaveSource(body);
        const fmt = await runAmana(['fmt', sourcePath, '--json']);
        return { fmt, ...(await sourcePayload()) };
      }));
      return;
    }
    if (req.method === 'POST' && parsedUrl.pathname === '/api/build') {
      sendJson(res, 200, await enqueueOperation(() => buildAndPreview(body)));
      return;
    }
    if (req.method === 'POST' && parsedUrl.pathname === '/api/stop-preview') {
      await stopPreview();
      sendJson(res, 200, { ok: true, previewReady: false });
      return;
    }
    sendJson(res, 404, { ok: false, error: 'Unknown API endpoint.' });
  } catch (error) {
    sendJson(res, 500, { ok: false, error: error.message, stack: error.stack });
  }
}

const server = http.createServer(async (req, res) => {
  const parsedUrl = new URL(req.url, `http://127.0.0.1:${studioPort}`);
  if (parsedUrl.pathname === '/favicon.ico') {
    res.writeHead(204, { 'cache-control': 'public, max-age=86400' });
    res.end();
    return;
  }
  if (parsedUrl.pathname.startsWith('/api/')) {
    await handleApi(req, res, parsedUrl);
    return;
  }
  if (parsedUrl.pathname.startsWith('/preview')) {
    proxyPreview(req, res, parsedUrl);
    return;
  }
  await serveStatic(req, res, parsedUrl.pathname);
});

ensureWorkspace()
  .then(() => {
    server.listen(studioPort, '127.0.0.1', () => {
      console.log(`[Amana Live Compiler] http://127.0.0.1:${studioPort}`);
      console.log(`[Amana Live Compiler] Editing ${sourcePath}`);
    });
  })
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });

process.on('SIGINT', () => {
  stopPreview().finally(() => process.exit(0));
});

process.on('SIGTERM', () => {
  stopPreview().finally(() => process.exit(0));
});
