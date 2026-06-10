const editor = document.getElementById('editor');
const diagnostics = document.getElementById('diagnostics');
const logs = document.getElementById('logs');
const saveState = document.getElementById('saveState');
const previewState = document.getElementById('previewState');
const compilerState = document.getElementById('compilerState');
const lastAction = document.getElementById('lastAction');
const preview = document.getElementById('preview');
const openPreview = document.getElementById('openPreview');
const autoPreview = document.getElementById('autoPreview');

const buttons = {
  save: document.getElementById('saveBtn'),
  check: document.getElementById('checkBtn'),
  format: document.getElementById('formatBtn'),
  inspect: document.getElementById('inspectBtn'),
  build: document.getElementById('buildBtn'),
  stop: document.getElementById('stopBtn')
};

let saveTimer = null;
let buildTimer = null;
let busy = false;
let dirty = false;

function setBusy(value, label = 'ready') {
  busy = value;
  compilerState.textContent = label;
  Object.values(buttons).forEach((button) => {
    button.disabled = value;
  });
}

function showJson(label, payload) {
  lastAction.textContent = label;
  diagnostics.textContent = JSON.stringify(payload, null, 2);
  const ok = payload && (payload.ok === true || payload.code === 0 || (payload.build && payload.build.ok));
  diagnostics.className = ok ? 'ok' : 'error';
}

async function api(path, body = null) {
  const response = await fetch(path, {
    method: body ? 'POST' : 'GET',
    headers: body ? { 'content-type': 'application/json' } : undefined,
    body: body ? JSON.stringify(body) : undefined
  });
  const text = await response.text();
  let data = null;
  try {
    data = text ? JSON.parse(text) : {};
  } catch {
    data = { ok: false, error: text || `HTTP ${response.status}` };
  }
  if (!response.ok) {
    throw new Error(data.error || `HTTP ${response.status}`);
  }
  return data;
}

async function loadSource() {
  const data = await api('/api/source');
  editor.value = data.source;
  saveState.textContent = `loaded ${new Date(data.updatedAt).toLocaleTimeString()}`;
  dirty = false;
}

async function saveSource() {
  try {
    const data = await api('/api/source', { source: editor.value });
    saveState.textContent = `saved ${new Date(data.updatedAt).toLocaleTimeString()}`;
    dirty = false;
    return data;
  } catch (error) {
    showError('save', error);
    throw error;
  }
}

function scheduleSave() {
  dirty = true;
  saveState.textContent = 'unsaved';
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    saveSource().catch((error) => {
      saveState.textContent = error.message;
    });
  }, 450);
}

function scheduleBuild() {
  if (!autoPreview.checked) return;
  clearTimeout(buildTimer);
  buildTimer = setTimeout(() => {
    buildPreview();
  }, 1600);
}

async function checkSource() {
  setBusy(true, 'checking');
  try {
    const result = await api('/api/check', { source: editor.value });
    showJson('check', result.json || result);
  } catch (error) {
    showError('check', error);
  } finally {
    setBusy(false);
  }
}

async function inspectDesign() {
  setBusy(true, 'inspecting');
  try {
    const result = await api('/api/inspect-design', { source: editor.value });
    showJson('inspect-design', result.json || result);
  } catch (error) {
    showError('inspect-design', error);
  } finally {
    setBusy(false);
  }
}

async function formatSource() {
  setBusy(true, 'formatting');
  try {
    const result = await api('/api/fmt', { source: editor.value });
    if (typeof result.source === 'string') {
      editor.value = result.source;
    }
    showJson('fmt', result.fmt && result.fmt.json ? result.fmt.json : result);
    saveState.textContent = 'formatted';
    dirty = false;
  } catch (error) {
    showError('fmt', error);
  } finally {
    setBusy(false);
  }
}

async function buildPreview() {
  if (busy) return;
  clearTimeout(saveTimer);
  saveTimer = null;
  setBusy(true, 'building');
  try {
    const result = await api('/api/build', { source: editor.value });
    showJson('build', result);
    if (result.preview && result.preview.ok) {
      const url = result.preview.proxiedUrl;
      previewState.textContent = `running on ${result.preview.port}`;
      preview.src = url;
      openPreview.href = url;
    } else {
      previewState.textContent = 'build failed';
    }
    await refreshStatus();
  } catch (error) {
    showError('build', error);
  } finally {
    setBusy(false);
  }
}

async function stopPreview() {
  try {
    const result = await api('/api/stop-preview', {});
    showJson('stop-preview', result);
    previewState.textContent = 'stopped';
    preview.removeAttribute('src');
    await refreshStatus();
  } catch (error) {
    showError('stop-preview', error);
  }
}

async function refreshStatus() {
  const status = await api('/api/status');
  logs.textContent = (status.logs || []).join('\n');
  if (status.previewReady) {
    previewState.textContent = `running on ${status.previewPort}`;
    openPreview.href = status.previewUrl || '/preview/';
  }
}

function showError(label, error) {
  lastAction.textContent = label;
  diagnostics.className = 'error';
  diagnostics.textContent = error.stack || error.message || String(error);
}

editor.addEventListener('input', () => {
  scheduleSave();
  scheduleBuild();
});

editor.addEventListener('keydown', (event) => {
  if (event.key === 'Tab') {
    event.preventDefault();
    const start = editor.selectionStart;
    const end = editor.selectionEnd;
    editor.value = `${editor.value.slice(0, start)}    ${editor.value.slice(end)}`;
    editor.selectionStart = editor.selectionEnd = start + 4;
    scheduleSave();
    scheduleBuild();
  }
  if ((event.ctrlKey || event.metaKey) && event.key === 's') {
    event.preventDefault();
    saveSource();
  }
  if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') {
    event.preventDefault();
    buildPreview();
  }
  if ((event.ctrlKey || event.metaKey) && event.shiftKey && event.key.toLowerCase() === 'f') {
    event.preventDefault();
    formatSource();
  }
});

buttons.save.addEventListener('click', () => saveSource());
buttons.check.addEventListener('click', checkSource);
buttons.format.addEventListener('click', formatSource);
buttons.inspect.addEventListener('click', inspectDesign);
buttons.build.addEventListener('click', buildPreview);
buttons.stop.addEventListener('click', stopPreview);

autoPreview.addEventListener('change', () => {
  localStorage.setItem('amana:auto-preview', autoPreview.checked ? '1' : '0');
});

if (localStorage.getItem('amana:auto-preview') === '0') {
  autoPreview.checked = false;
}

loadSource()
  .then(() => refreshStatus())
  .then(() => {
    if (autoPreview.checked) buildPreview();
  })
  .catch((error) => {
    diagnostics.textContent = error.stack || error.message;
    diagnostics.className = 'error';
  });

setInterval(() => {
  if (!dirty) refreshStatus().catch(() => {});
}, 2500);
