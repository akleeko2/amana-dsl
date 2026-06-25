import { spawn, execSync } from 'child_process';
import http from 'http';

const PORT = 3210;
const APP_PATH = 'target/baselines/test_modal_grid/app.js';
const SOURCE_PATH = 'component_test/visual_baseline.amana';
const BUILD_OUTPUT = 'target/baselines/test_modal_grid';

function wait(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function isServerReady() {
  return new Promise((resolve) => {
    const req = http.get(`http://localhost:${PORT}/`, (res) => {
      resolve(res.statusCode < 400);
    });
    req.on('error', () => {
      resolve(false);
    });
    req.end();
  });
}

async function main() {
  console.log('[Visual Smoke] Building test Amana application...');
  execSync(`cargo run -- build ${SOURCE_PATH} ${BUILD_OUTPUT}`, { stdio: 'inherit' });

  console.log('[Visual Smoke] Starting Express server on port 3210...');
  const serverProcess = spawn('node', [APP_PATH], {
    env: {
      ...process.env,
      PORT: String(PORT),
      SESSION_SECRET: 'visual_smoke_session_secret_123456789_long_secret',
      NODE_ENV: 'production',
      AMANA_FORCE_HTTPS: 'false'
    },
    stdio: 'inherit'
  });

  serverProcess.on('error', (err) => {
    console.error('[Visual Smoke] Failed to start server:', err);
    process.exit(1);
  });

  // Wait for the server to be ready
  let ready = false;
  for (let i = 0; i < 20; i++) {
    await wait(500);
    ready = await isServerReady();
    if (ready) {
      break;
    }
  }

  if (!ready) {
    console.error('[Visual Smoke] Express server failed to become ready in time.');
    serverProcess.kill();
    process.exit(1);
  }

  console.log('[Visual Smoke] Express server is ready. Running Playwright tests...');
  let testFailed = false;
  try {
    execSync('npx playwright test', { stdio: 'inherit' });
    console.log('[Visual Smoke] Playwright tests passed successfully!');
  } catch (err) {
    console.error('[Visual Smoke] Playwright tests failed.');
    testFailed = true;
  } finally {
    console.log('[Visual Smoke] Stopping Express server...');
    serverProcess.kill();
  }

  if (testFailed) {
    process.exit(1);
  }
}

main().catch((err) => {
  console.error('[Visual Smoke] Script error:', err);
  process.exit(1);
});
