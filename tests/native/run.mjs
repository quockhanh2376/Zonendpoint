import { remote } from 'webdriverio';
import { spawn, spawnSync } from 'node:child_process';
import { createServer } from 'node:net';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import assert from 'node:assert/strict';

const root = path.resolve('.');
const run = mkdtempSync(path.join(tmpdir(), 'zonendpoint-native-'));
const artifacts = path.join(run, 'artifacts'); mkdirSync(artifacts);
const fixtures = path.join(run, 'fixtures'); mkdirSync(fixtures);
const fixture = path.join(fixtures, 'benign.txt');
writeFileSync(fixture, 'Zonendpoint benign fixture');
const systemRoot = path.join(run, 'system'); mkdirSync(systemRoot); mkdirSync(path.join(systemRoot, 'drivers'));
const coreNames = ['ntoskrnl.exe', 'ntdll.dll', 'kernel32.dll', 'kernelbase.dll', 'ci.dll', 'smss.exe', 'csrss.exe', 'wininit.exe', 'services.exe', 'lsass.exe', 'svchost.exe', 'winlogon.exe', 'winload.efi'];
for (const name of coreNames) writeFileSync(path.join(systemRoot, name), `Benign system fixture ${name}`);
writeFileSync(path.join(systemRoot, 'drivers', 'fixture.sys'), 'Benign driver fixture');
const real = process.argv.includes('--real');
const results = [];
let browser, driver, server, serverPort, mode = 'clean';
let mixedResponses = 0;
const sockets = new Set();
const log = [];
const env = {
  ...process.env,
  LOCALAPPDATA: path.join(run, 'appdata'),
  WEBVIEW2_USER_DATA_FOLDER: path.join(run, 'webview'),
  ZONENDPOINT_TEST_FIXTURE: fixtures,
  ZONENDPOINT_TEST_SYSTEM_ROOT: systemRoot
};
const target = process.env.CARGO_TARGET_DIR || path.join(root, 'src-tauri', 'target');
const binary = path.join(target, 'debug', 'zonendpoint.exe');
const nativeDriver = process.env.EDGE_DRIVER || (existsSync('E:\\Zonendpoint-tools\\edgedriver\\msedgedriver.exe') ? 'E:\\Zonendpoint-tools\\edgedriver\\msedgedriver.exe' : 'msedgedriver.exe');
const report = () => writeFileSync(path.join(artifacts, 'report.json'), JSON.stringify({ real, targetInjection: 'debug feature; real picker NOT RUN', results }, null, 2));

function build() {
  const r = spawnSync('cmd.exe', ['/d', '/c', 'npm.cmd run tauri build -- --debug --no-bundle --features native-ui-tests'], { cwd: root, env, stdio: 'inherit' });
  if (r.status !== 0) throw Error('Native test build failed');
}

async function check(name, test) {
  try {
    await test();
    results.push({ name, status: 'PASS' });
  } catch (e) {
    results.push({ name, status: 'FAIL', error: String(e) });
    throw e;
  }
}

async function button(label) {
  return label === "Start scan" ? browser.$("button.primary") : browser.$(`button=${label}`);
}

async function select(kind = 'file') {
  await (await button("Select target ▾")).click();
  await (await button(kind === "file" ? "File" : "Folder")).click();
  await browser.waitUntil(async () => (await browser.$('.selected-path').getText()).includes(kind === 'file' ? 'benign.txt' : fixtures), { timeout: 5000 });
}

async function scan(expected) {
  writeFileSync(fixture, `Benign fixture ${Date.now()} ${Math.random()}`);
  await select();
  const start = await button('Start scan');
  await start.waitForEnabled();
  await start.click();
  const clean = expected === 'Scan complete' && (real || mode === 'clean');
  const targetHeading = clean ? 'All Good' : expected;
  await browser.waitUntil(async () => await browser.$('h2').isExisting(), { timeout: 20000 });
  await browser.waitUntil(async () => (await browser.$('h2').getText()) === targetHeading, { timeout: 20000 });
  assert.equal(await browser.execute(() => {
    const r = document.querySelector('h2').getBoundingClientRect();
    return r.top >= 0 && r.bottom <= innerHeight;
  }), true, 'Scan outcome heading must be visible without scrolling');

  if (clean) {
    assert.equal(await browser.$('.all-good-line').getText(), 'All Good');
    assert.equal(await browser.$('button.primary').getText(), 'Quick Scan');
    assert.equal(await browser.$('.results').isExisting(), false);
  }
}

function startMockServer(port = 0) {
  server = createServer(s => {
    sockets.add(s);
    s.on('close', () => sockets.delete(s));
    s.on('error', () => {});
    let data = Buffer.alloc(0), command = false;
    s.on('data', chunk => {
      data = Buffer.concat([data, chunk]);
      if (!command) {
        const end = data.indexOf(0);
        if (end < 0) return;
        const cmd = data.subarray(0, end).toString();
        data = data.subarray(end + 1);
        command = true;
        if (cmd === 'zVERSION') {
          s.end('ClamAV native-mock/signature-1\0');
          return;
        }
        if (cmd !== 'zINSTREAM') {
          s.destroy();
          return;
        }
      }
      while (data.length >= 4) {
        const n = data.readUInt32BE();
        if (n > 25 * 1024 * 1024) { s.destroy(); return; }
        if (data.length < 4 + n) return;
        data = data.subarray(4 + n);
        if (n === 0) {
          if (mode === 'delay') return;
          s.end((mode === 'infected' || (mode === 'mixed' && mixedResponses++ === 0))
            ? 'stream: Native-Mock-Signature FOUND\0'
            : mode === 'malformed'
              ? 'garbage OK\0'
              : mode === 'error'
                ? 'stream: failed ERROR\0'
                : 'stream: OK\0');
          return;
        }
      }
    });
  });
  return new Promise(resolve => server.listen(port, '127.0.0.1', () => {
    serverPort = server.address().port;
    resolve();
  }));
}

async function stopMockServer() {
  for (const s of sockets) s.destroy();
  if (server) {
    await new Promise(r => server.close(r));
    server = null;
  }
}

try {
  if (!process.argv.includes('--skip-build')) build();
  if (!real) {
    await startMockServer(0);
    env.ZONENDPOINT_CLAMD = `127.0.0.1:${serverPort}`;
  } else {
    if (!process.env.ZONENDPOINT_CLAMD) throw Error('Set ZONENDPOINT_CLAMD to private real lab endpoint');
  }
  driver = spawn(process.env.TAURI_DRIVER || 'tauri-driver.exe', ['--native-driver', nativeDriver, '--port', '4445', '--native-port', '4446'], { env, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
  log.length = 0;
  driver.stdout.on('data', d => log.push(d.toString()));
  driver.stderr.on('data', d => log.push(d.toString()));
  driver.on('error', e => log.push(String(e)));
  for (let i = 0; i < 100; i++) {
    try {
      await fetch('http://127.0.0.1:4445/status', { signal: AbortSignal.timeout(1000) });
      await new Promise(r => setTimeout(r, 500));
      break;
    } catch {
      await new Promise(r => setTimeout(r, 100));
    }
  }
  async function session() {
    browser = await remote({
      hostname: '127.0.0.1',
      port: 4445,
      logLevel: 'error',
      connectionRetryCount: 2,
      capabilities: {
        'tauri:options': {
          application: binary,
          webviewOptions: { userDataFolder: path.join(run, 'webview') }
        }
      }
    });
    await browser.$('h1').waitForExist({ timeout: 15000 });
  }
  await session();

  await check('native shell and English controls', async () => {
    assert.equal(await browser.$('h1').getText(), 'Scan files and folders');
    assert.equal(await browser.execute(() => document.documentElement.dataset.theme), 'light');
    await (await button('Start scan')).waitForEnabled();
    assert.equal(await (await button('Start scan')).getText(), 'Quick Scan');
    assert.equal((await browser.$$('button=Cancel scan')).length, 0);
    assert.equal(await browser.$('.server-status').isExisting(), true);
  });

  await check('menu items', async () => {
    await browser.$('button=Select target ▾').click();
    await browser.$('#target-options').waitForExist({ timeout: 5000 });
    const buttons = await browser.$$('#target-options button');
    assert.equal(buttons.length, 2);
    const labels = [];
    for (const b of buttons) {
      labels.push(await b.getText());
    }
    assert.deepEqual(labels, ['File', 'Folder']);
    assert.ok(!labels.includes('Quick Scan'));
    await browser.keys(['Escape']);
  });

  await check('auto-return', async () => {
    await select('file');
    assert.equal(await browser.$('button.primary').getText(), 'Scan');
    await browser.$('button.primary').click();
    await browser.waitUntil(async () => await browser.$('.all-good-line').isExisting(), { timeout: 20000 });
    assert.equal(await browser.$('button.primary').getText(), 'Quick Scan');
    assert.equal(await browser.$('.all-good-line').getText(), 'All Good');
  });

  await check('single All Good line', async () => {
    assert.equal(await browser.$('.all-good-line').getText(), 'All Good');
    assert.equal(await browser.$('h2').getText(), 'All Good');
    assert.equal(await browser.$('.results').isExisting(), false);
    assert.equal((await browser.$$('.results li')).length, 0);
  });

  await check('Quick Scan runs approved core and driver fixtures', async () => {
    await browser.$('button.primary').click();
    await browser.waitUntil(async () => await browser.$('.all-good-line').isExisting(), { timeout: 20000 });
    assert.equal(await browser.$('.all-good-line').getText(), 'All Good');
    assert.equal(await browser.$('.results').isExisting(), false);
    assert.equal(await browser.$('button.primary').getText(), 'Quick Scan');
  });

  if (!real) await check('incomplete list', async () => {
    rmSync(path.join(systemRoot, 'ci.dll'));
    await browser.$('button.primary').click();
    await browser.waitUntil(async () => await browser.$('h2').getText() === 'Scan incomplete', { timeout: 20000 });
    assert.equal(await browser.$('.notice.warning').getText(), 'Do not treat an incomplete scan as clean.');
    const items = await browser.$$('.results li');
    assert.ok(items.length > 0, 'Incomplete scan must show error items');
    for (const item of items) {
      assert.doesNotMatch(await item.$('.status').getText(), /Clean/);
    }
    assert.match(await browser.$('.results').getText(), /ci\.dll/);
    writeFileSync(path.join(systemRoot, 'ci.dll'), 'Restored harmless ci fixture');
  });

  if (!real) await check('threat-only list', async () => {
    mode = 'infected';
    writeFileSync(path.join(systemRoot, 'ci.dll'), 'changed threat simulation ' + Date.now());
    await browser.$('button.primary').click();
    await browser.waitUntil(async () => await browser.$('h2').getText() === 'Scan complete', { timeout: 20000 });
    const items = await browser.$$('.results li');
    assert.ok(items.length > 0, 'Must show threat items');
    for (const item of items) {
      assert.match(await item.$('.status').getText(), /Threat detected/);
      assert.doesNotMatch(await item.$('.status').getText(), /Clean/);
    }
    assert.match(await browser.$('.results').getText(), /Native-Mock-Signature/);
    mode = 'clean';
    writeFileSync(path.join(systemRoot, 'ci.dll'), 'Restored harmless ci fixture');
  });

  if (!real) await check('Quick Scan Stop completes before another mode can start', async () => {
    mode = 'delay';
    writeFileSync(path.join(systemRoot, 'ntoskrnl.exe'), 'changed cancellation fixture ' + Date.now());
    await browser.$('button.primary').click();
    await (await button('Stop')).waitForExist();
    await (await button('Stop')).click();
    await browser.waitUntil(async () => await browser.$('h2').getText() === 'Scan cancelled', { timeout: 10000 });
    assert.equal(await browser.$('.all-good-line').isExisting(), false);
    mode = 'clean';
  });

  if (!real) await check('status icon green→red→green', async () => {
    const statusBtn = await browser.$('.server-status');
    await browser.waitUntil(async () => await statusBtn.getAttribute('data-status') === 'connected', { timeout: 10000 });
    assert.equal(await statusBtn.getAttribute('title'), 'Connected to ZonEndpoint server');

    await stopMockServer();
    await statusBtn.click();
    await browser.waitUntil(async () => await statusBtn.getAttribute('data-status') === 'disconnected', { timeout: 10000 });
    assert.equal(await statusBtn.getAttribute('title'), 'Cannot connect to ZonEndpoint server. Check your connection.');

    await startMockServer(serverPort);
    await statusBtn.click();
    await browser.waitUntil(async () => await statusBtn.getAttribute('data-status') === 'connected', { timeout: 10000 });
    assert.equal(await statusBtn.getAttribute('title'), 'Connected to ZonEndpoint server');
  });

  await check('theme and logo persistence after native relaunch', async () => {
    await browser.$('.theme-toggle').click();
    assert.equal(await browser.execute(() => document.documentElement.dataset.theme), 'dark');
    assert.match(await browser.$('.brand-logo').getAttribute('src'), /logo-dark/);
    await browser.saveScreenshot(path.join(artifacts, 'dark.png'));
    await browser.deleteSession();
    await session();
    assert.equal(await browser.execute(() => document.documentElement.dataset.theme), 'dark');
    await browser.$('.theme-toggle').click();
    assert.match(await browser.$('.brand-logo').getAttribute('src'), /logo-light/);
  });

  if (!real) {
    for (const state of ['error', 'malformed']) {
      await check(`${state} never becomes CLEAN`, async () => {
        mode = state;
        await scan('Scan incomplete');
        assert.doesNotMatch(await browser.$('.results').getText(), /Clean/);
      });
    }

    await check('Stop lifecycle and scan again', async () => {
      mode = 'delay';
      writeFileSync(fixture, `cancel ${Date.now()}`);
      await select();
      await (await button('Start scan')).click();
      await (await button('Stop')).waitForExist();
      await (await button('Stop')).click();
      await browser.waitUntil(async () => await browser.$('h2').getText() === 'Scan cancelled', { timeout: 10000 });
      assert.match(await browser.$('main').getText(), /clean 0/);
      mode = 'clean';
      await scan('Scan complete');
    });

    await check('offline never becomes CLEAN', async () => {
      await stopMockServer();
      await (await button('Start scan')).click();
      await browser.waitUntil(async () => await browser.$('.notice.error').isExisting(), { timeout: 10000 });
      assert.equal(await browser.$('.notice.error').getText(), 'Cannot connect to ZonEndpoint server');
      assert.equal(await browser.$('.all-good-line').isExisting(), false);
      assert.equal(await browser.$('.server-status').getAttribute('data-status'), 'disconnected');
      await startMockServer(serverPort);
    });
  }

  await browser.saveScreenshot(path.join(artifacts, 'final.png'));
  writeFileSync(path.join(artifacts, 'driver.log'), log.join(''));
} catch (e) {
  if (browser) await browser.saveScreenshot(path.join(artifacts, 'failure.png')).catch(() => {});
  console.error(e);
  process.exitCode = 1;
} finally {
  if (browser) await browser.deleteSession().catch(() => {});
  if (driver) driver.kill();
  await stopMockServer();
  writeFileSync(path.join(artifacts, 'driver.log'), log.join(''));
  report();
  console.log(`Native test artifacts: ${artifacts}`);
}
