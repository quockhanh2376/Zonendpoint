import { remote } from 'webdriverio';
import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { mkdtempSync, mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';

const outDir = 'E:\\Zonendpoint-tools\\artifacts\\321d32b';
const run = mkdtempSync(path.join(tmpdir(), 'zonendpoint-capture-'));
const fixtures = path.join(run, 'fixtures'); mkdirSync(fixtures);
const fixture = path.join(fixtures, 'benign.txt');
writeFileSync(fixture, 'Zonendpoint benign fixture');

const systemRoot = path.join(run, 'system'); mkdirSync(systemRoot); mkdirSync(path.join(systemRoot, 'drivers'));
mkdirSync(path.join(systemRoot, 'drivers', 'DriverData'));
writeFileSync(path.join(systemRoot, 'drivers', 'DriverData', 'ignored.bin'), 'excluded from quick scan');
const coreNames = ['ntoskrnl.exe', 'ntdll.dll', 'kernel32.dll', 'kernelbase.dll', 'ci.dll', 'smss.exe', 'csrss.exe', 'wininit.exe', 'services.exe', 'lsass.exe', 'svchost.exe', 'winlogon.exe', 'winload.efi'];
for (const name of coreNames) writeFileSync(path.join(systemRoot, name), `Benign system fixture ${name}`);
writeFileSync(path.join(systemRoot, 'drivers', 'fixture.sys'), 'Benign driver fixture');

let browser, driver, server, serverPort;
const sockets = new Set();

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
          s.end('stream: OK\0');
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

const env = {
  ...process.env,
  LOCALAPPDATA: path.join(run, 'appdata'),
  WEBVIEW2_USER_DATA_FOLDER: path.join(run, 'webview'),
  ZONENDPOINT_TEST_FIXTURE: fixtures,
  ZONENDPOINT_TEST_SYSTEM_ROOT: systemRoot
};

const binary = 'E:\\Zonendpoint-vic-compact\\src-tauri\\target\\debug\\zonendpoint.exe';
const nativeDriver = 'E:\\Zonendpoint-tools\\edgedriver\\msedgedriver.exe';

try {
  await startMockServer(0);
  env.ZONENDPOINT_CLAMD = `127.0.0.1:${serverPort}`;

  driver = spawn('tauri-driver.exe', ['--native-driver', nativeDriver, '--port', '4447', '--native-port', '4448'], { env, windowsHide: true, stdio: 'ignore' });
  for (let i = 0; i < 100; i++) {
    try {
      await fetch('http://127.0.0.1:4447/status', { signal: AbortSignal.timeout(1000) });
      await new Promise(r => setTimeout(r, 500));
      break;
    } catch {
      await new Promise(r => setTimeout(r, 100));
    }
  }

  browser = await remote({
    hostname: '127.0.0.1',
    port: 4447,
    logLevel: 'error',
    capabilities: {
      'tauri:options': {
        application: binary,
        webviewOptions: { userDataFolder: path.join(run, 'webview') }
      }
    }
  });

  await browser.$('h1').waitForExist({ timeout: 15000 });
  const statusBtn = await browser.$('.server-status');
  await browser.waitUntil(async () => await statusBtn.getAttribute('data-status') === 'connected', { timeout: 10000 });

  // 1. Idle Light
  await browser.saveScreenshot(path.join(outDir, 'idle-light.png'));
  console.log('Saved idle-light.png');

  // 2. Idle Dark
  await browser.$('.theme-toggle').click();
  await browser.waitUntil(async () => await browser.execute(() => document.documentElement.dataset.theme === 'dark'), { timeout: 5000 });
  await browser.saveScreenshot(path.join(outDir, 'idle-dark.png'));
  console.log('Saved idle-dark.png');

  // 3. All Good Dark
  const startBtn = await browser.$('button.primary');
  await startBtn.waitForEnabled();
  await startBtn.click();
  await browser.waitUntil(async () => await browser.$('.all-good-line').isExisting(), { timeout: 20000 });
  await browser.saveScreenshot(path.join(outDir, 'all-good-dark.png'));
  console.log('Saved all-good-dark.png');

  // 4. All Good Light
  await browser.$('.theme-toggle').click();
  await browser.waitUntil(async () => await browser.execute(() => document.documentElement.dataset.theme === 'light'), { timeout: 5000 });
  await browser.saveScreenshot(path.join(outDir, 'all-good-light.png'));
  console.log('Saved all-good-light.png');

} catch (err) {
  console.error('Capture failed:', err);
  process.exitCode = 1;
} finally {
  if (browser) await browser.deleteSession().catch(() => {});
  if (driver) driver.kill();
  for (const s of sockets) s.destroy();
  if (server) await new Promise(r => server.close(r));
}
