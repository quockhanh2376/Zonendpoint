import { remote } from 'webdriverio';
import { spawn, spawnSync } from 'node:child_process';
import { createServer } from 'node:net';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync, existsSync } from 'node:fs';
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
mkdirSync(path.join(systemRoot, 'drivers', 'DriverData'));
writeFileSync(path.join(systemRoot, 'drivers', 'DriverData', 'ignored.bin'), 'excluded from quick scan');
const coreNames = ['ntoskrnl.exe', 'ntdll.dll', 'kernel32.dll', 'kernelbase.dll', 'ci.dll', 'smss.exe', 'csrss.exe', 'wininit.exe', 'services.exe', 'lsass.exe', 'svchost.exe', 'winlogon.exe', 'winload.efi'];
for (const name of coreNames) writeFileSync(path.join(systemRoot, name), `Benign system fixture ${name}`);
writeFileSync(path.join(systemRoot, 'drivers', 'fixture.sys'), 'Benign driver fixture');
const testAppdataZon = path.join(run, 'appdata', 'Zonendpoint');
mkdirSync(testAppdataZon, { recursive: true });
function formatIsoVn(date) {
  const vnTime = new Date(date.getTime() + 7 * 3600 * 1000);
  const yyyy = vnTime.getUTCFullYear();
  const mm = String(vnTime.getUTCMonth() + 1).padStart(2, '0');
  const dd = String(vnTime.getUTCDate()).padStart(2, '0');
  const hh = String(vnTime.getUTCHours()).padStart(2, '0');
  const min = String(vnTime.getUTCMinutes()).padStart(2, '0');
  const ss = String(vnTime.getUTCSeconds()).padStart(2, '0');
  return `${yyyy}-${mm}-${dd}T${hh}:${min}:${ss}+07:00`;
}
const mockServerStatus = {
  signatures: 8616464,
  loaded_at: formatIsoVn(new Date(Date.now() - 30 * 60 * 1000)),
  fetched_at: formatIsoVn(new Date(Date.now() - 5 * 60 * 1000)),
  engine: "ClamAV 1.5.4/28182"
};
const mockStatusJson = JSON.stringify(mockServerStatus, null, 2);
const mockStatusWithBom = Buffer.concat([
  Buffer.from([0xEF, 0xBB, 0xBF]),
  Buffer.from(mockStatusJson, 'utf8')
]);
writeFileSync(path.join(testAppdataZon, 'server-status.json'), mockStatusWithBom);
const real = process.argv.includes('--real');
const results = [];
let browser, driver, server, serverPort, mode = 'clean', versionDelayMs = 0;
let mixedResponses = 0;
let customThreatName = null;
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

async function setInputValue(selector, value) {
  await browser.execute((sel, val) => {
    const input = document.querySelector(sel);
    if (!input) throw new Error(`Element ${sel} not found`);
    const nativeInputValueSetter = Object.getOwnPropertyDescriptor(
      window.HTMLInputElement.prototype,
      'value'
    ).set;
    nativeInputValueSetter.call(input, val);
    input.dispatchEvent(new Event('input', { bubbles: true }));
    input.dispatchEvent(new Event('change', { bubbles: true }));
  }, selector, value);
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

function sendCloseMainWindow(pid) {
  const ps = `
    Add-Type @"
    using System;
    using System.Runtime.InteropServices;
    public class NativeWinClose {
      [DllImport("user32.dll", CharSet = CharSet.Auto)]
      public static extern bool PostMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);
    }
"@
    $p = Get-Process -Id ${pid} -ErrorAction SilentlyContinue
    if ($p -and $p.MainWindowHandle -ne 0) {
      [NativeWinClose]::PostMessage($p.MainWindowHandle, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
    }
  `;
  spawnSync('powershell.exe', ['-NoProfile', '-Command', ps]);
}

function isProcessAlive(pid) {
  const ps = `if (Get-Process -Id ${pid} -ErrorAction SilentlyContinue) { "alive" } else { "dead" }`;
  const res = spawnSync('powershell.exe', ['-NoProfile', '-Command', ps], { encoding: 'utf8' });
  return res.stdout.includes('alive');
}

function getOrphanChildren(pid) {
  const ps = `(Get-CimInstance Win32_Process | Where-Object { $_.ParentProcessId -eq ${pid} }).ProcessId`;
  const res = spawnSync('powershell.exe', ['-NoProfile', '-Command', ps], { encoding: 'utf8' });
  return res.stdout.trim().split(/\\s+/).map(Number).filter(Boolean);
}

function findTestAppPid() {
  const resolvedBinary = path.resolve(binary);
  const targetDir = path.resolve(target);
  const worktreeName = path.basename(root);
  const ps = `
    $targetBin = '${resolvedBinary.replace(/'/g, "''")}';
    $targetDir = '${targetDir.replace(/'/g, "''")}';
    $wName = '${worktreeName.replace(/'/g, "''")}';

    # 1. Look up via Win32_Process with full ExecutablePath
    $proc = Get-CimInstance Win32_Process -Filter "Name = 'zonendpoint.exe'" | Where-Object {
      ($_.ExecutablePath -and $_.ExecutablePath -eq $targetBin) -or
      ($_.ExecutablePath -and $_.ExecutablePath.StartsWith($targetDir, [System.StringComparison]::OrdinalIgnoreCase)) -or
      ($_.ExecutablePath -and $_.ExecutablePath -like "*$wName*")
    } | Sort-Object CreationDate -Descending | Select-Object -First 1;

    if (-not $proc) {
      # 2. Fallback to Get-Process
      $proc = Get-Process -Name 'zonendpoint' -ErrorAction SilentlyContinue | Where-Object {
        ($_.Path -and $_.Path -eq $targetBin) -or
        ($_.Path -and $_.Path.StartsWith($targetDir, [System.StringComparison]::OrdinalIgnoreCase)) -or
        ($_.Path -and $_.Path -like "*$wName*")
      } | Select-Object -First 1;
    }

    if ($proc) { $proc.ProcessId }
  `;
  for (let i = 0; i < 15; i++) {
    const res = spawnSync('powershell.exe', ['-NoProfile', '-Command', ps], { encoding: 'utf8' });
    const pid = parseInt(res.stdout.trim().split(/\\s+/)[0], 10);
    if (pid && !isNaN(pid)) return pid;
    const end = Date.now() + 200;
    while (Date.now() < end) {}
  }
  return NaN;
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
          if (versionDelayMs > 0) {
            setTimeout(() => {
              try { s.end('ClamAV native-mock/signature-1\0'); } catch {}
            }, versionDelayMs);
            return;
          }
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
          const sig = customThreatName || 'Native-Mock-Signature';
          s.end((mode === 'infected' || (mode === 'mixed' && mixedResponses++ === 0))
            ? `stream: ${sig} FOUND\0`
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
    process.env.ZONENDPOINT_CLAMD = `127.0.0.1:${serverPort}`;
    process.env.LOCALAPPDATA = env.LOCALAPPDATA;
    // Pre-populate config.json in test appdata so that even if ZONENDPOINT_CLAMD is not read,
    // the saved address points to mock server, never falling back to default 127.0.0.1:13310
    writeFileSync(path.join(testAppdataZon, 'config.json'), JSON.stringify({
      server_address: `127.0.0.1:${serverPort}`
    }, null, 2));
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

  await check('status dot position, app version, and theme toggle in both themes (V-24)', async () => {
    const tauriConf = JSON.parse(readFileSync(path.join(root, 'src-tauri', 'tauri.conf.json'), 'utf8'));
    const expectedVersion = `v${tauriConf.version}`;
    await browser.waitUntil(async () => (await browser.$('.app-version').getText()) === expectedVersion, { timeout: 10000 });
    const verifyPositions = async (themeName) => {
      const info = await browser.execute(() => {
        const status = document.querySelector('.server-status');
        const version = document.querySelector('.app-version');
        const logo = document.querySelector('.brand-logo');
        const card = document.querySelector('.selection-panel');
        const toggle = document.querySelector('.selection-panel .theme-toggle');
        const cardTitle = document.querySelector('.selection-panel h1');
        if (!status || !logo || !card || !toggle || !cardTitle || !version) return null;
        const s = status.getBoundingClientRect();
        const v = version.getBoundingClientRect();
        const l = logo.getBoundingClientRect();
        const c = card.getBoundingClientRect();
        const t = toggle.getBoundingClientRect();
        const h = cardTitle.getBoundingClientRect();

        function parseRgb(str) {
          const m = str ? str.match(/\d+/g) : null;
          return m ? m.slice(0, 3).map(Number) : [0, 0, 0];
        }
        function lum(r, g, b) {
          const a = [r, g, b].map(val => {
            val /= 255;
            return val <= 0.03928 ? val / 12.92 : Math.pow((val + 0.055) / 1.055, 2.4);
          });
          return 0.2126 * a[0] + 0.7152 * a[1] + 0.0722 * a[2];
        }
        function contrast(rgb1, rgb2) {
          const l1 = lum(...rgb1), l2 = lum(...rgb2);
          return (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05);
        }
        const vColor = getComputedStyle(version).color;
        const pageBg = getComputedStyle(document.body).backgroundColor;
        const contrastRatio = contrast(parseRgb(vColor), parseRgb(pageBg));

        return {
          versionText: version.textContent.trim(),
          versionBelowStatus: v.top >= s.bottom - 1,
          versionInLeftColumn: v.right <= c.left + 5,
          versionContrast: contrastRatio,
          statusInLeftColumn: s.right <= c.left + 5,
          statusNextToLogo: s.left >= l.left,
          statusAlignedWithLogo: s.top >= l.top - 10 && s.bottom <= l.bottom + 10,
          statusAlignedWithCardTop: Math.abs(s.top - c.top) <= 2,
          statusTopDiff: Math.abs(s.top - c.top),
          toggleInsideCard: t.left >= c.left && t.right <= c.right && t.top >= c.top && t.bottom <= c.bottom,
          toggleSameRowAsTitle: Math.abs((t.top + t.bottom) / 2 - (h.top + h.bottom) / 2) < 20,
          toggleInTopRightOfCard: t.right > h.right,
          statusWidth: s.width,
          statusHeight: s.height,
          toggleWidth: t.width,
          toggleHeight: t.height,
          noFloatingHeaderControls: document.querySelector('.header-controls') === null,
          noDocHScroll: document.documentElement.scrollWidth <= window.innerWidth
        };
      });
      assert.ok(info, `Elements must exist in ${themeName} theme`);
      assert.equal(info.versionText, expectedVersion, `Version label must show ${expectedVersion} in ${themeName}`);
      assert.ok(info.versionBelowStatus, `Version label must sit directly below status dot in ${themeName}`);
      assert.ok(info.versionInLeftColumn, `Version label must be in left column in ${themeName}`);
      assert.ok(info.versionContrast >= 4.5, `Version text contrast must be >= 4.5:1 in ${themeName} (got ${info.versionContrast.toFixed(2)})`);
      assert.ok(info.noDocHScroll, `Document must not horizontally overflow in ${themeName} at 800px`);
      assert.ok(info.statusInLeftColumn, `Status dot must be in left column (left of card) in ${themeName}`);
      assert.ok(info.statusNextToLogo, `Status dot must sit next to logo in ${themeName}`);
      assert.ok(info.statusAlignedWithLogo, `Status dot must be vertically aligned with logo in ${themeName}`);
      assert.ok(info.statusAlignedWithCardTop, `Status dot top must align with card top in ${themeName} (diff: ${info.statusTopDiff}px)`);
      assert.ok(info.toggleInsideCard, `Theme toggle must be inside scan card in ${themeName}`);
      assert.ok(info.toggleSameRowAsTitle, `Theme toggle must be in same row as card title in ${themeName}`);
      assert.ok(info.toggleInTopRightOfCard, `Theme toggle must be in top-right of card in ${themeName}`);
      assert.ok(info.statusWidth >= 36 && info.statusHeight >= 36, `Status dot target must be >=36x36px in ${themeName}`);
      assert.ok(info.toggleWidth >= 36 && info.toggleHeight >= 36, `Theme toggle target must be >=36x36px in ${themeName}`);
      assert.ok(info.noFloatingHeaderControls, `No floating buttons left in topbar in ${themeName}`);
    };

    assert.equal(await browser.execute(() => document.documentElement.dataset.theme), 'light');
    await verifyPositions('light');

    await browser.$('.theme-toggle').click();
    assert.equal(await browser.execute(() => document.documentElement.dataset.theme), 'dark');
    await verifyPositions('dark');

    await browser.$('.theme-toggle').click();
    assert.equal(await browser.execute(() => document.documentElement.dataset.theme), 'light');
  });

  await check('server status Cover badge displays formatted signatures from local file (V-52)', async () => {
    const coverEl = await browser.$('.brand-cover');
    assert.equal(await coverEl.isExisting(), true, 'Cover badge element must exist');
    await browser.waitUntil(async () => {
      const txt = await coverEl.getText();
      return txt.includes('Cover:') && txt.includes('8,616,464');
    }, { timeout: 10000, timeoutMsg: 'Cover badge did not display 8,616,464 within timeout' });

    const title = await coverEl.getAttribute('title');
    assert.match(title, /ClamAV signatures on server.*loaded.*\(UTC\+7\).*synced.*\(UTC\+7\)/i);

    // Verify in dark theme as well
    await browser.$('.theme-toggle').click();
    assert.equal(await coverEl.isExisting(), true);
    const darkTxt = await coverEl.getText();
    assert.ok(darkTxt.includes('8,616,464'), 'Cover badge must remain visible and populated in dark theme');
    await browser.$('.theme-toggle').click(); // switch back to light
  });

  await check('keyboard focus order', async () => {
    await browser.waitUntil(async () => !(await browser.$('.server-status').getAttribute('disabled')), { timeout: 10000 });
    await browser.execute(() => {
      document.querySelector('.server-status')?.focus();
    });
    assert.equal(await browser.execute(() => document.activeElement?.classList.contains('server-status')), true, 'First focus is status dot');

    await browser.keys(['Tab']);
    assert.equal(await browser.execute(() => document.activeElement?.classList.contains('theme-toggle')), true, 'Second focus is theme toggle');

    await browser.keys(['Tab']);
    assert.equal(await browser.execute(() => document.activeElement?.textContent?.includes('Select target')), true, 'Third focus is Select target');

    await browser.keys(['Tab']);
    assert.equal(await browser.execute(() => document.activeElement?.classList.contains('primary')), true, 'Fourth focus is main action button');
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
    await browser.waitUntil(async () => (await browser.execute(() => window.innerHeight)) > 300, { timeout: 10000 });
    const heightWithThreats = await browser.execute(() => window.innerHeight);
    assert.ok(heightWithThreats > 300, `Window innerHeight must expand for non-clean results (got ${heightWithThreats})`);
    mode = 'clean';
    writeFileSync(path.join(systemRoot, 'ci.dll'), 'Restored harmless ci fixture');
  });

  if (!real) await check('150+ char threat name produces no horizontal scroll in Light and Dark', async () => {
    mode = 'infected';
    customThreatName = 'Win.Test.LongName.Trojan.Dropper.Generic.VeryLongSignatureStringWithNoSpacesToTestTextWrappingAndLayoutIntegrityUnderExtremeLengthConditions.Variant1234567890.End';
    assert.ok(customThreatName.length >= 150, 'Threat name must be at least 150 characters');
    writeFileSync(path.join(systemRoot, 'ci.dll'), 'changed threat simulation ' + Date.now());
    await browser.$('button.primary').click();
    await browser.waitUntil(async () => await browser.$('h2').getText() === 'Scan complete', { timeout: 20000 });
    await browser.$('.results').waitForExist({ timeout: 5000 });
    assert.match(await browser.$('.results').getText(), /Win\.Test\.LongName/);

    const checkNoHorizontalScroll = async () => {
      return await browser.execute(() => {
        const results = document.querySelector('.results');
        return results ? results.scrollWidth <= results.clientWidth : false;
      });
    };

    assert.equal(await browser.execute(() => document.documentElement.dataset.theme), 'light');
    assert.equal(await checkNoHorizontalScroll(), true, 'Results list must have scrollWidth <= clientWidth in Light theme');

    await browser.$('.theme-toggle').click();
    assert.equal(await browser.execute(() => document.documentElement.dataset.theme), 'dark');
    assert.equal(await checkNoHorizontalScroll(), true, 'Results list must have scrollWidth <= clientWidth in Dark theme');

    await browser.$('.theme-toggle').click();
    assert.equal(await browser.execute(() => document.documentElement.dataset.theme), 'light');

    mode = 'clean';
    customThreatName = null;
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

  if (!real) await check('status icon connected→checking→disconnected in Light and Dark', async () => {
    const statusIconArtifactsDir = 'E:\\Zonendpoint-tools\\artifacts\\status-icon';
    mkdirSync(statusIconArtifactsDir, { recursive: true });
    const statusBtn = await browser.$('.server-status');
    await browser.waitUntil(async () => await statusBtn.getAttribute('data-status') === 'connected', { timeout: 10000 });
    assert.equal(await statusBtn.getAttribute('title'), 'Show server settings');
    assert.equal(await browser.$('.server-status svg.status-icon-connected').isExisting(), true, 'Connected checkmark SVG must exist');

    // 1. Connected screenshots (light & dark)
    await browser.saveScreenshot(path.join(statusIconArtifactsDir, 'connected-light.png'));
    await browser.$('.theme-toggle').click();
    assert.equal(await browser.execute(() => document.documentElement.dataset.theme), 'dark');
    assert.equal(await browser.$('.server-status svg.status-icon-connected').isExisting(), true, 'Connected checkmark SVG must exist in dark');
    await browser.saveScreenshot(path.join(statusIconArtifactsDir, 'connected-dark.png'));
    await browser.$('.theme-toggle').click();
    assert.equal(await browser.execute(() => document.documentElement.dataset.theme), 'light');

    // 2. Checking state: delay version response for light screenshot
    versionDelayMs = 2000;
    await statusBtn.click();
    await browser.waitUntil(async () => await statusBtn.getAttribute('data-status') === 'checking', { timeout: 3000 });
    assert.equal(await statusBtn.getAttribute('title'), 'Checking connection…');
    assert.equal(await browser.$('.server-status svg.server-status-spinner').isExisting(), true, 'Checking spinner SVG must exist in light');
    await browser.saveScreenshot(path.join(statusIconArtifactsDir, 'checking-light.png'));
    await browser.waitUntil(async () => await statusBtn.getAttribute('data-status') === 'connected', { timeout: 10000 });

    // Checking state: dark theme screenshot
    await browser.$('.theme-toggle').click();
    assert.equal(await browser.execute(() => document.documentElement.dataset.theme), 'dark');
    await statusBtn.click();
    await browser.waitUntil(async () => await statusBtn.getAttribute('data-status') === 'checking', { timeout: 3000 });
    assert.equal(await browser.$('.server-status svg.server-status-spinner').isExisting(), true, 'Checking spinner SVG must exist in dark');
    await browser.saveScreenshot(path.join(statusIconArtifactsDir, 'checking-dark.png'));
    await browser.waitUntil(async () => await statusBtn.getAttribute('data-status') === 'connected', { timeout: 10000 });
    await browser.$('.theme-toggle').click();
    assert.equal(await browser.execute(() => document.documentElement.dataset.theme), 'light');
    versionDelayMs = 0;

    // 3. Disconnected state: stop mock server, trigger check
    await stopMockServer();
    await statusBtn.click();
    await browser.waitUntil(async () => await statusBtn.getAttribute('data-status') === 'disconnected', { timeout: 10000 });
    assert.equal(await statusBtn.getAttribute('title'), 'Cannot connect to ZonEndpoint server. Check your connection.');
    assert.equal(await browser.$('.server-status svg.status-icon-disconnected').isExisting(), true, 'Disconnected X SVG must exist in light');
    await browser.saveScreenshot(path.join(statusIconArtifactsDir, 'disconnected-light.png'));

    await browser.$('.theme-toggle').click();
    assert.equal(await browser.execute(() => document.documentElement.dataset.theme), 'dark');
    assert.equal(await browser.$('.server-status svg.status-icon-disconnected').isExisting(), true, 'Disconnected X SVG must exist in dark');
    await browser.saveScreenshot(path.join(statusIconArtifactsDir, 'disconnected-dark.png'));
    await browser.$('.theme-toggle').click();
    assert.equal(await browser.execute(() => document.documentElement.dataset.theme), 'light');

    // Restore mock server
    await startMockServer(serverPort);
    await statusBtn.click();
    await browser.waitUntil(async () => await statusBtn.getAttribute('data-status') === 'connected', { timeout: 10000 });
    assert.equal(await statusBtn.getAttribute('title'), 'Show server settings');
  });

  if (!real) await check('automatic preflight and background check preserves icon without flashing checking (Q-14)', async () => {
    const statusBtn = await browser.$('.server-status');
    await browser.waitUntil(async () => await statusBtn.getAttribute('data-status') === 'connected', { timeout: 10000 });

    // Branch 2: automatic checkServer(manual=false) during startScan preflight
    versionDelayMs = 1500;
    const scanBtn = await browser.$('button.primary');
    await scanBtn.click();

    // While checkServer is awaiting clamd preflight (delayed by 1.5s), status must STAY "connected" (no spinner, no "checking")
    await browser.pause(400);
    assert.equal(await statusBtn.getAttribute('data-status'), 'connected', 'Status must remain connected during preflight');
    assert.equal(await browser.$('.server-status svg.status-icon-connected').isExisting(), true, 'Connected checkmark must remain visible');
    assert.equal(await browser.$('.server-status svg.server-status-spinner').isExisting(), false, 'Spinner must NOT appear during automated preflight');

    // Wait for scan to complete
    await browser.waitUntil(async () => await browser.$('.all-good-line').isExisting(), { timeout: 20000 });
    versionDelayMs = 0;
    assert.equal(await statusBtn.getAttribute('data-status'), 'connected');

    // Test offline transition via automated check without flashing "checking":
    await stopMockServer();
    await scanBtn.click();
    await browser.waitUntil(async () => await statusBtn.getAttribute('data-status') === 'disconnected', { timeout: 10000 });
    assert.equal(await browser.$('.server-status svg.status-icon-disconnected').isExisting(), true);
    assert.equal(await browser.$('.server-status svg.server-status-spinner').isExisting(), false, 'Spinner must never have flashed during offline transition');

    // Restore mock server
    await startMockServer(serverPort);
    // Branch 1: manual click MUST show checking spinner
    versionDelayMs = 1500;
    await statusBtn.click();
    await browser.waitUntil(async () => await statusBtn.getAttribute('data-status') === 'checking', { timeout: 3000 });
    assert.equal(await browser.$('.server-status svg.server-status-spinner').isExisting(), true, 'Manual click MUST show checking spinner');
    await browser.waitUntil(async () => await statusBtn.getAttribute('data-status') === 'connected', { timeout: 10000 });
    assert.equal(await browser.$('.server-status svg.status-icon-connected').isExisting(), true);
    versionDelayMs = 0;
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
      await browser.waitUntil(async () => (await browser.execute(() => window.innerHeight)) <= 300, { timeout: 10000 });
      const heightAfterAllGood = await browser.execute(() => window.innerHeight);
      assert.ok(heightAfterAllGood <= 300, `Window innerHeight after All Good must be compact <= 300px (got ${heightAfterAllGood})`);

      // Verify no excessive empty space under card (< 80px gap)
      const gap = await browser.execute(() => {
        const card = document.querySelector('.all-good-card');
        return card ? window.innerHeight - card.getBoundingClientRect().bottom : null;
      });
      assert.ok(gap !== null && gap >= 0 && gap < 80, `No large empty space under All Good card (gap: ${gap}px)`);
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

    await check('server settings input, validation, and V-54 row visibility', async () => {
      // 1. In disconnected state: server row is automatically visible
      const input = await browser.$('#server-address-input');
      assert.equal(await input.isExisting(), true, 'Server address input exists when disconnected');
      const saveBtn = await browser.$('.server-save-btn');
      assert.equal(await saveBtn.isExisting(), true, 'Server save button exists');

      // Test rejection of public IP
      await setInputValue('#server-address-input', '8.8.8.8:3310');
      await saveBtn.click();
      await browser.waitUntil(async () => await browser.$('.server-msg-error').isExisting(), { timeout: 5000 });
      assert.match(await browser.$('.server-msg-error').getText(), /must use loopback, private/);

      // Test saving valid loopback address
      await setInputValue('#server-address-input', `127.0.0.1:${serverPort}`);
      await saveBtn.click();
      await browser.waitUntil(async () => await browser.$('.server-status').getAttribute('data-status') === 'connected', { timeout: 10000 });

      // State 1: Connected OK -> Server row is HIDDEN
      await browser.waitUntil(async () => !(await browser.$('.server-config-form').isExisting()), { timeout: 5000 });
      assert.equal(await browser.$('.server-config-form').isExisting(), false, 'Server row must be hidden when connected OK');
      assert.equal(await browser.$('#server-address-input').isExisting(), false);
      assert.equal(await browser.$('.server-status').getAttribute('title'), 'Show server settings');
      assert.equal(await browser.$('.server-status').getAttribute('aria-label'), 'Show server settings');

      // State 2: Click status icon while connected -> TOGGLE visible / hidden
      await browser.$('.server-status').click();
      await browser.waitUntil(async () => await browser.$('.server-config-form').isExisting(), { timeout: 3000 });
      assert.equal(await browser.$('#server-address-input').isExisting(), true, 'Server row appears after clicking icon');
      assert.equal(await browser.$('.server-status').getAttribute('title'), 'Hide server settings');
      assert.equal(await browser.$('.server-status').getAttribute('aria-label'), 'Hide server settings');

      await browser.$('.server-status').click();
      await browser.waitUntil(async () => !(await browser.$('.server-config-form').isExisting()), { timeout: 3000 });
      assert.equal(await browser.$('.server-config-form').isExisting(), false, 'Server row hides after clicking icon again');
      assert.equal(await browser.$('.server-status').getAttribute('title'), 'Show server settings');

      // State 3: Connection failure / server offline -> Server row automatically APPEARS
      await stopMockServer();
      await browser.$('.server-status').click();
      await browser.waitUntil(async () => await browser.$('.server-status').getAttribute('data-status') === 'disconnected', { timeout: 10000 });
      assert.equal(await browser.$('.server-config-form').isExisting(), true, 'Server row must be visible when connection fails');
      assert.equal(await browser.$('#server-address-input').isExisting(), true);

      // Restore mock server and verify it automatically hides again when reconnected OK
      await startMockServer(serverPort);
      await browser.$('.server-status').click();
      await browser.waitUntil(async () => await browser.$('.server-status').getAttribute('data-status') === 'connected', { timeout: 10000 });
      await browser.waitUntil(async () => !(await browser.$('.server-config-form').isExisting()), { timeout: 5000 });
      assert.equal(await browser.$('.server-config-form').isExisting(), false, 'Server row must automatically hide when reconnected OK');
      assert.equal(await browser.$('.server-status').getAttribute('title'), 'Show server settings');
    });

    await check('window close after scan and during scan terminates cleanly without orphan processes (V-30)', async () => {
      const appPid = findTestAppPid();
      assert.ok(appPid, 'Test application PID must be found');

      // 1. Test close after scan:
      sendCloseMainWindow(appPid);
      let exitedAfterScan = false;
      for (let i = 0; i < 25; i++) {
        await new Promise(r => setTimeout(r, 200));
        if (!isProcessAlive(appPid)) {
          exitedAfterScan = true;
          break;
        }
      }
      assert.ok(exitedAfterScan, `Application process ${appPid} must exit cleanly after window close`);
      const orphans = getOrphanChildren(appPid);
      assert.equal(orphans.length, 0, `No orphan child processes should remain after close (found: ${orphans.join(', ')})`);
      await browser.deleteSession().catch(() => {});

      // 2. Test close during active scan:
      mode = 'delay';
      writeFileSync(path.join(systemRoot, 'ntoskrnl.exe'), 'changed cancellation fixture ' + Date.now());
      await session();
      const appPid2 = findTestAppPid();
      assert.ok(appPid2, 'Second test application PID must be found');

      await browser.waitUntil(async () => await browser.$('.server-status').getAttribute('data-status') === 'connected', { timeout: 10000 });
      const startBtn = await button('Start scan');
      await startBtn.waitForEnabled({ timeout: 10000 });
      await startBtn.click();
      await browser.waitUntil(async () => {
        const h2 = await browser.$('h2');
        return (await h2.isExisting()) && (await h2.getText()) === 'Scanning';
      }, { timeout: 10000 });

      sendCloseMainWindow(appPid2);
      let exitedDuringScan = false;
      for (let i = 0; i < 25; i++) {
        await new Promise(r => setTimeout(r, 200));
        if (!isProcessAlive(appPid2)) {
          exitedDuringScan = true;
          break;
        }
      }
      assert.ok(exitedDuringScan, `Application process ${appPid2} must cancel scan and exit cleanly on window close`);
      const orphans2 = getOrphanChildren(appPid2);
      assert.equal(orphans2.length, 0, `No orphan child processes should remain after close during scan`);

      mode = 'clean';
      writeFileSync(path.join(systemRoot, 'ntoskrnl.exe'), 'Restored harmless ntoskrnl fixture');
      await browser.deleteSession().catch(() => {});
      await session();
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
