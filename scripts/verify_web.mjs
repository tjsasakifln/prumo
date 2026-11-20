#!/usr/bin/env node
// Administrative browser evidence. Not shipped as product logic.
// Uses Node 22+ built-ins and an installed Chrome/Chromium, without npm packages.
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile, writeFile, mkdir, mkdtemp, rm, access } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import { tmpdir } from 'node:os';
import { resolve, join, extname, sep } from 'node:path';

const [directoryArg, fixtureArg, evidenceArg, nativeEvidenceArg] = process.argv.slice(2);
assert(directoryArg && fixtureArg && evidenceArg,
  'Usage: node scripts/verify_web.mjs dist/prumo-web fixture.json evidence-directory [native-capture.json]');
assert.equal(typeof WebSocket, 'function', 'Node 22+ with built-in WebSocket is required.');
const directory = resolve(directoryArg), fixturePath = resolve(fixtureArg), evidence = resolve(evidenceArg);
const fixtureText = await readFile(fixturePath, 'utf8');
const fixture = JSON.parse(fixtureText);
const nativeSnapshot = JSON.parse(await readFile(`${fixturePath}.snapshot.json`, 'utf8'));
let nativeCapture;
if (nativeEvidenceArg) {
  const bytes = await readFile(resolve(nativeEvidenceArg));
  const record = JSON.parse(bytes);
  assert.equal(record.native_backend_executed, true);
  assert.equal(record.view_report.snapshot_sha256, nativeSnapshot.sha256);
  assert.equal(record.view_report.project_id, fixture.id);
  assert.equal(record.view_report.source_revision, fixture.revision);
  const png = await readFile(resolve(nativeEvidenceArg).replace(/\.json$/, '.png'));
  assert.equal(createHash('sha256').update(png).digest('hex'), record.image_sha256);
  nativeCapture = { report_sha256: createHash('sha256').update(bytes).digest('hex'), image_sha256: record.image_sha256, snapshot_sha256: nativeSnapshot.sha256 };
}
const manifest = JSON.parse(await readFile(join(directory, 'manifest.json'), 'utf8'));
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
await mkdir(evidence, { recursive: true });
for (const [file, expected] of Object.entries(manifest.files)) {
  assert.equal(hash(await readFile(join(directory, file))), expected, `Package hash mismatch: ${file}`);
}

let browser;
for (const candidate of [process.env.CHROME_BIN, '/usr/bin/google-chrome', '/usr/bin/google-chrome-stable', '/usr/bin/chromium', '/usr/bin/chromium-browser'].filter(Boolean)) {
  try { await access(candidate); browser = candidate; break; } catch { /* try next installed browser */ }
}
assert(browser, 'Install Chrome/Chromium or set CHROME_BIN.');
const temporary = await mkdtemp(join(tmpdir(), 'prumo-web-ci-'));
const mime = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript', '.wasm': 'application/wasm', '.json': 'application/json' };
const server = createServer(async (request, response) => {
  try {
    const path = resolve(directory, '.' + decodeURIComponent(new URL(request.url, 'http://localhost').pathname));
    assert(path === directory || path.startsWith(directory + sep), 'Invalid path');
    const file = path === directory ? join(path, 'index.html') : path;
    const bytes = await readFile(file);
    response.writeHead(200, { 'Content-Type': mime[extname(file)] ?? 'application/octet-stream', 'Cache-Control': 'no-store' });
    response.end(bytes);
  } catch {
    if (response.headersSent) { response.destroy(); return; }
    response.writeHead(404); response.end('Not found');
  }
});
await new Promise((ok, fail) => { server.once('error', fail); server.listen(0, '127.0.0.1', ok); });
const origin = `http://127.0.0.1:${server.address().port}`;
const child = spawn(browser, [
  '--headless=new', '--no-sandbox', '--disable-dev-shm-usage', '--no-first-run',
  '--no-default-browser-check', '--remote-debugging-port=0', `--user-data-dir=${temporary}`,
  '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--window-size=1600,1800', 'about:blank',
], { stdio: ['ignore', 'ignore', 'pipe'] });
let stderr = '', processError;
child.stderr.on('data', data => { stderr += data.toString(); });
child.on('error', error => { processError = error; });
const delay = milliseconds => new Promise(ok => setTimeout(ok, milliseconds));
async function until(callback, description, timeout = 20000) {
  const end = Date.now() + timeout;
  let last;
  while (Date.now() < end) {
    if (processError) throw processError;
    if (child.exitCode !== null) throw new Error(`Browser exited (${child.exitCode}): ${stderr}`);
    try { const result = await callback(); if (result) return result; } catch (error) { last = error; }
    await delay(100);
  }
  throw new Error(`Timed out: ${description}${last ? `: ${last.message}` : ''}`);
}

let socket;
let importActivation;
const checks = [], screenshots = {};
const diagnostics = [];
const locations = trace => (trace?.callFrames ?? []).slice(0, 3).map(frame => ({
  line: frame.lineNumber, column: frame.columnNumber,
}));
try {
  const port = await until(async () => (await readFile(join(temporary, 'DevToolsActivePort'), 'utf8')).split('\n')[0], 'CDP port');
  const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
  const page = targets.find(target => target.type === 'page');
  assert(page, 'Browser did not expose a page');
  socket = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((ok, fail) => { socket.addEventListener('open', ok, { once: true }); socket.addEventListener('error', fail, { once: true }); });
  let sequence = 0;
  const pending = new Map();
  socket.addEventListener('message', event => {
    const message = JSON.parse(event.data);
    if (message.method === 'Runtime.exceptionThrown') {
      const details = message.params.exceptionDetails;
      diagnostics.push({ kind: 'exception', line: details.lineNumber, column: details.columnNumber, locations: locations(details.stackTrace) });
    } else if (message.method === 'Runtime.consoleAPICalled' && message.params.type === 'error') {
      diagnostics.push({ kind: 'console.error', argument_count: message.params.args.length, locations: locations(message.params.stackTrace) });
    }
    if (diagnostics.length > 8) diagnostics.shift();
    const item = pending.get(message.id);
    if (item) { clearTimeout(item.timer); pending.delete(message.id); message.error ? item.fail(new Error(JSON.stringify(message.error))) : item.ok(message.result); }
  });
  const call = (method, params = {}) => new Promise((ok, fail) => {
    const id = ++sequence;
    const timer = setTimeout(() => { pending.delete(id); fail(new Error(`CDP timeout: ${method}`)); }, 30000);
    pending.set(id, { ok, fail, timer });
    socket.send(JSON.stringify({ id, method, params }));
  });
  const evaluate = async expression => {
    const result = await call('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
    if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
    return result.result.value;
  };
  const key = async (keyName, code, windowsVirtualKeyCode) => {
    const text = keyName === 'Enter' ? '\r' : '';
    await call('Input.dispatchKeyEvent', {
      type: text ? 'keyDown' : 'rawKeyDown', key: keyName, code, windowsVirtualKeyCode,
      text, unmodifiedText: text,
    });
    await call('Input.dispatchKeyEvent', { type: 'keyUp', key: keyName, code, windowsVirtualKeyCode });
  };
  const enter = () => key('Enter', 'Enter', 13);
  const tab = () => key('Tab', 'Tab', 9);
  const report = () => evaluate('JSON.parse(window.prumoTestModule.view_report())');
  const command = command => evaluate(`JSON.parse(window.prumoTestModule.view_command(${JSON.stringify(JSON.stringify(command))}))`);
  await call('Page.enable'); await call('Runtime.enable'); await call('Accessibility.enable');
  await call('Emulation.setDeviceMetricsOverride', { width: 1600, height: 1800, deviceScaleFactor: 1, mobile: false });
  await call('Page.navigate', { url: `${origin}/index.html` });
  await until(() => evaluate('document.body?.getAttribute("data-prumo-ready")==="true"'), 'Rust WebRunner initialized', 120000);
  await evaluate('(async () => { window.prumoTestModule = await import("./pkg/prumo_web.js"); return true; })()');
  await evaluate(`(() => {
    const button = document.getElementById('load-project');
    window.prumoImportProbe = { keydown: 0, keypress: 0, keyup: 0, clicks: 0, trusted_clicks: 0, focused_on_keydown: null };
    for (const type of ['keydown', 'keypress', 'keyup']) {
      button.addEventListener(type, event => {
        if (event.key !== 'Enter') return;
        window.prumoImportProbe[type]++;
        if (type === 'keydown') window.prumoImportProbe.focused_on_keydown = document.activeElement === button;
      }, { passive: true });
    }
    button.addEventListener('click', event => {
      window.prumoImportProbe.clicks++;
      if (event.isTrusted) window.prumoImportProbe.trusted_clicks++;
    }, { passive: true });
    return true;
  })()`);
  await evaluate(`document.getElementById('project-json').value=${JSON.stringify(fixtureText)};document.getElementById('load-project').focus();true`);
  importActivation = { focused_before: await evaluate('document.activeElement === document.getElementById("load-project")') };
  assert.equal(importActivation.focused_before, true, 'Import button must have focus before Enter');
  await enter();
  Object.assign(importActivation, await evaluate(`(() => {
    const status = document.getElementById('project-status').textContent;
    return {
      ...window.prumoImportProbe,
      focused_after: document.activeElement === document.getElementById('load-project'),
      input_present: document.getElementById('project-json').value.length > 0,
      status_kind: status === 'Projeto aberto. Consulta de modelagem, sem cálculo.' ? 'success' : status ? 'message' : 'empty',
    };
  })()`));
  console.log(JSON.stringify({ suite: 'PRU-011-web', phase: 'import-activation', import_activation: importActivation }, null, 2));
  assert.equal(importActivation.clicks, 1, 'Enter must activate the import button exactly once');
  assert.equal(importActivation.trusted_clicks, 1, 'Import activation must come from browser keyboard input');
  const imported = await until(async () => { const r = await report(); return r.project_id === fixture.id ? r : false; }, 'native project imported');
  assert.equal(imported.source_revision, fixture.revision);
  assert.equal(imported.snapshot_sha256, nativeSnapshot.sha256);
  assert.deepEqual(imported.objects.map(object => object.id), fixture.objects.map(object => object.id));
  checks.push('Native JSON identity, revision, object UUIDs and canonical snapshot match in Rust/WASM.');

  await evaluate('document.getElementById("previous-object").focus(); true');
  await tab();
  assert.equal(await evaluate('document.activeElement.id'), 'next-object');
  const focus = await evaluate('getComputedStyle(document.activeElement).outlineWidth');
  assert(parseFloat(focus) >= 3, `Keyboard focus outline missing: ${focus}`);
  await enter();
  const first = (await report()).state.selected_id;
  await enter();
  const second = (await report()).state.selected_id;
  assert(first && second && first !== second);
  await tab();
  assert.equal(await evaluate('document.activeElement.id'), 'hide-object');
  await enter();
  assert((await report()).state.hidden_ids.includes(second));
  const elevated = fixture.objects.find(object => object.name === 'Viga superior');
  assert(elevated, 'Gallery must contain the elevated beam');
  await command({ action: 'select', object_id: elevated.id, reveal: true });
  await command({ action: 'hide_selected' });
  await command({ action: 'set_cut', cut: { axis: 'z', min_m: -0.2, max_m: 1 } });
  const other = fixture.objects.find(object => object.id !== elevated.id);
  assert(other, 'Gallery must contain another object for pending DOM selection');
  await command({ action: 'select', object_id: other.id, reveal: false });
  await evaluate(`document.getElementById('object-list').value=${JSON.stringify(elevated.id)};document.getElementById('select-object').focus();true`);
  await delay(500);
  assert.equal(await evaluate('document.getElementById("object-list").value'), elevated.id,
    'Unconfirmed DOM choice must survive the 150 ms render refresh');
  assert.equal((await report()).state.selected_id, other.id,
    'Choosing an option must not select the object before confirmation');
  await enter();
  let current = await report();
  assert.equal(current.state.selected_id, elevated.id);
  assert.equal(current.snapshot_sha256, nativeSnapshot.sha256);
  assert.equal(current.state.cut, null);
  assert(!current.state.hidden_ids.includes(elevated.id));
  for (const view of ['plan', 'section', 'three_d']) {
    current = await command({ action: 'set_view', view });
    assert.equal(current.state.selected_id, elevated.id);
  }
  checks.push('Tab/Enter, visible focus, object iteration, hidden selection and another floor preserve the same UUID across views.');
  checks.push('A different UUID chosen in the DOM survives 500 ms of render refresh and is selected by Enter without changing the model snapshot.');
  const accessibility = await call('Accessibility.getFullAXTree');
  for (const label of ['Abrir projeto JSON', 'Selecionar e localizar', 'Ocultar seleção', 'Restaurar vista gráfica']) {
    assert(accessibility.nodes.some(node => node.name?.value === label && !node.ignored), `Accessible label absent: ${label}`);
  }
  checks.push('Browser accessibility tree exposes the named DOM controls; no human screen-reader claim.');
  async function screenshot(name) {
    await delay(500);
    const capture = await call('Page.captureScreenshot', { format: 'png', captureBeyondViewport: false });
    const bytes = Buffer.from(capture.data, 'base64');
    await writeFile(join(evidence, name), bytes);
    screenshots[name] = hash(bytes);
  }
  await screenshot('web-100.png');
  await command({ action: 'set_preferences', scale: 1.5, high_contrast: true });
  assert.equal((await report()).state.scale, 1.5);
  await screenshot('web-150-contrast.png');
  checks.push('150% scale and high contrast set through the same Rust command; actual browser captures saved for inspection.');
  const lost = await evaluate(`(() => {
    const canvas=document.getElementById('prumo-canvas');
    const gl=canvas.getContext('webgl2') || canvas.getContext('webgl');
    window.prumoLoss=gl?.getExtension('WEBGL_lose_context');
    if(!window.prumoLoss) return false;
    window.prumoLoss.loseContext(); return true;
  })()`);
  assert(lost, 'WEBGL_lose_context extension unavailable; recovery criterion is not waived.');
  await until(async () => Boolean((await report()).state.graphics_failure), 'WebGL loss observed');
  await evaluate('document.getElementById("next-object").focus();true');
  await enter();
  assert((await report()).state.selected_id);
  assert.equal((await report()).snapshot_sha256, nativeSnapshot.sha256);
  await screenshot('web-context-lost.png');
  await evaluate('window.prumoLoss.restoreContext(); true');
  await until(async () => !(await report()).state.graphics_failure, 'WebGL recovery observed', 30000);
  assert.equal((await report()).snapshot_sha256, nativeSnapshot.sha256);
  checks.push('Actual WebGL context loss retains DOM navigation and model snapshot; restoration resumes the shared session.');
  const rejects = await evaluate(`(() => {
    try { window.prumoTestModule.import_project('{"$serde_json::private::Number":"1"}'); return false; }
    catch { return true; }
  })()`);
  assert(rejects);
  assert.equal((await report()).snapshot_sha256, nativeSnapshot.sha256);
  checks.push('Invalid reserved JSON representation is rejected without replacing the loaded project.');
  const result = {
    suite: 'PRU-011-web', passed: true, browser: await call('Browser.getVersion'),
    package_source: manifest.source, source_dirty: manifest.source_dirty,
    wasm_sha256: manifest.files['pkg/prumo_web_bg.wasm'],
    fixture_sha256: hash(fixtureText), snapshot_sha256: nativeSnapshot.sha256,
    native_window_executed: false, browser_executed: true, human_screen_reader_tested: false,
    matching_native_capture: nativeCapture ?? null,
    checks, screenshots,
  };
  await writeFile(join(evidence, 'report.json'), JSON.stringify(result, null, 2) + '\n');
  console.log(JSON.stringify(result, null, 2));
} catch (error) {
  const error_locations = [...String(error.stack ?? '').matchAll(/verify_web\.mjs:(\d+):(\d+)/g)]
    .slice(0, 3).map(match => ({ line: Number(match[1]), column: Number(match[2]) }));
  console.error(JSON.stringify({ suite: 'PRU-011-web', passed: false, checks, diagnostics, import_activation: importActivation, error_locations }, null, 2));
  await writeFile(join(evidence, 'failure.json'), JSON.stringify({ suite: 'PRU-011-web', passed: false, error: String(error), checks, stderr }, null, 2) + '\n');
  throw error;
} finally {
  socket?.close();
  child.kill('SIGTERM');
  await new Promise(ok => server.close(ok));
  await rm(temporary, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
}
