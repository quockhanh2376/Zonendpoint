import { remote } from 'webdriverio';
import { spawn, spawnSync } from 'node:child_process';
import { createServer } from 'node:net';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import assert from 'node:assert/strict';

const root = path.resolve('.');
const run = mkdtempSync(path.join(tmpdir(), 'zonendpoint-native-'));
const artifacts = path.join(run, 'artifacts'); mkdirSync(artifacts);
const fixtures = path.join(run, 'fixtures'); mkdirSync(fixtures);
const fixture = path.join(fixtures, 'benign.txt');
writeFileSync(fixture, 'Zonendpoint benign fixture');
const real = process.argv.includes('--real');
const results = [];
let browser, driver, server, mode = 'clean';
let mixedResponses = 0;
const sockets = new Set();
const env = { ...process.env, LOCALAPPDATA: path.join(run, 'appdata'),
  WEBVIEW2_USER_DATA_FOLDER: path.join(run, 'webview'), ZONENDPOINT_TEST_FIXTURE: fixtures };
const target = process.env.CARGO_TARGET_DIR || path.join(root, 'src-tauri', 'target');
const binary = path.join(target, 'debug', 'zonendpoint.exe');
const nativeDriver = process.env.EDGE_DRIVER || 'msedgedriver.exe';
const report = () => writeFileSync(path.join(artifacts, 'report.json'), JSON.stringify({real, targetInjection:'debug feature; real picker NOT RUN',results}, null, 2));
function build() {
  const r = spawnSync('cmd.exe', ['/d', '/c', 'npm run tauri build -- --debug --no-bundle --features native-ui-tests'], {cwd:root, env, stdio:'inherit'});
  if (r.status !== 0) throw Error('Native test build failed');
}
async function check(name, test) {
  try { await test(); results.push({name,status:'PASS'}); }
  catch(e) { results.push({name,status:'FAIL',error:String(e)}); throw e; }
}
async function button(label) { return label === "Start scan" ? browser.$("button.primary") : browser.$(`button=${label}`); }
async function select(kind='file') { await (await button("Select target ▾")).click(); await (await button(kind === "file" ? "File" : "Folder")).click(); await browser.waitUntil(async()=> (await browser.$('.selected-path').getText()).includes(kind==='file'?'benign.txt':fixtures),{timeout:5000}); }
async function scan(expected) {
  writeFileSync(fixture, `Benign fixture ${Date.now()} ${Math.random()}`);
  await select(); const start=await button('Start scan'); await start.waitForEnabled(); await start.click();
  await browser.waitUntil(async()=>await browser.$('h2').isExisting(),{timeout:75000});
  await browser.waitUntil(async()=> (await browser.$('h2').getText())===expected,{timeout:75000});
  assert.equal(await browser.execute(()=>{const r=document.querySelector('h2').getBoundingClientRect();return r.top>=0 && r.bottom<=innerHeight;}),true,'Scan outcome heading must be visible without scrolling');
  const clean = expected==='Scan complete' && (real || mode==='clean');
  await browser.waitUntil(async()=>await browser.$('.primary.scan-clean').isExisting()===clean,{timeout:5000});
  if(clean){assert.equal(await browser.$('.primary').getText(),'All Good');assert.equal(await browser.$('.scan-check').isExisting(),true);await browser.waitUntil(async()=>await browser.execute(()=>{const r=document.querySelector('.results').getBoundingClientRect();return r.bottom<=innerHeight;}),{timeout:5000,timeoutMsg:'Window must grow to fit short scan results'});}
}
try {
  if (!process.argv.includes('--skip-build')) build();
  if (!real) {
    server=createServer(s=>{
      sockets.add(s); s.on('close',()=>sockets.delete(s)); s.on('error',()=>{});
      let data=Buffer.alloc(0), command=false;
      s.on('data',chunk=>{
        data=Buffer.concat([data,chunk]);
        if(!command){const end=data.indexOf(0); if(end<0)return; const cmd=data.subarray(0,end).toString(); data=data.subarray(end+1);command=true;
          if(cmd==='zVERSION'){s.end('ClamAV native-mock/signature-1\0');return;}
          if(cmd!=='zINSTREAM'){s.destroy();return;}
        }
        while(data.length>=4){const n=data.readUInt32BE(); if(n>25*1024*1024){s.destroy();return;} if(data.length<4+n)return;data=data.subarray(4+n);if(n===0){
          if(mode==='delay')return;
          s.end((mode==='infected' || (mode==='mixed' && mixedResponses++===0))?'stream: Native-Mock-Signature FOUND\0':mode==='malformed'?'garbage OK\0':mode==='error'?'stream: failed ERROR\0':'stream: OK\0');return;
        }}
      });
    });
    await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
    env.ZONENDPOINT_CLAMD=`127.0.0.1:${server.address().port}`;
  } else {
    if(!process.env.ZONENDPOINT_CLAMD)throw Error('Set ZONENDPOINT_CLAMD to private real lab endpoint');
  }
  driver=spawn(process.env.TAURI_DRIVER || 'tauri-driver.exe',['--native-driver',nativeDriver,'--port','4445','--native-port','4446'],{env,windowsHide:true,stdio:['ignore','pipe','pipe']});
  const log=[]; driver.stdout.on('data',d=>log.push(d.toString()));driver.stderr.on('data',d=>log.push(d.toString()));
  driver.on('error',e=>log.push(String(e)));
  for(let i=0;i<100;i++){try{await fetch('http://127.0.0.1:4445/status', {signal:AbortSignal.timeout(1000)});break;}catch{await new Promise(r=>setTimeout(r,100));}}
  async function session(){browser=await remote({hostname:'127.0.0.1',port:4445,logLevel:'error',connectionRetryCount:0,capabilities:{'tauri:options':{application:binary,webviewOptions:{userDataFolder:path.join(run,'webview')}}}});await browser.$('h1').waitForExist({timeout:15000});}
  await session();
  await check('native shell and English controls',async()=>{
    assert.equal(await browser.$('h1').getText(),'Scan files and folders');
    assert.equal(await browser.execute(()=>document.documentElement.dataset.theme),'light');
    assert.equal(await (await button('Start scan')).isEnabled(),false);
    assert.equal((await browser.$$('button=Cancel scan')).length,0);
  });
  await check('theme and logo persistence after native relaunch',async()=>{
    await browser.$('.theme-toggle').click();
    assert.equal(await browser.execute(()=>document.documentElement.dataset.theme),'dark');
    assert.match(await browser.$('.brand-logo').getAttribute('src'),/logo-dark/);
    await browser.saveScreenshot(path.join(artifacts,'dark.png'));
    await browser.deleteSession();await session();
    assert.equal(await browser.execute(()=>document.documentElement.dataset.theme),'dark');
    await browser.$('.theme-toggle').click();
    assert.match(await browser.$('.brand-logo').getAttribute('src'),/logo-light/);
  });
  await check(real?'real ClamAV benign scan via native UI and Rust IPC':'mock successful native scan',async()=>{await scan('Scan complete');assert.match(await browser.$('.results').getText(),/Clean/);});
  await check('folder target through debug injection',async()=>{writeFileSync(fixture,'folder uncached '+Date.now()+' '+Math.random());await select('folder');await (await button('Start scan')).click();await browser.waitUntil(async()=>await browser.$('h2').getText()==='Scan complete',{timeout:75000});assert.match(await browser.$('.results').getText(),/Clean in this scan/);assert.match(await browser.$('.result-path').getText(),/benign\.txt/);assert.match(await browser.$('main').getText(),/Processed 1 items · clean 1/);});
  if(!real){
    await check('threat details remain visible beyond50 results',async()=>{
      mode='mixed';mixedResponses=0;
      writeFileSync(fixture,'mixed benign '+Date.now());
      const extras=Array.from({length:50},(_,i)=>path.join(fixtures,`extra-${i}.txt`));
      for(const [i,file] of extras.entries())writeFileSync(file,`unique mixed fixture ${i} ${Date.now()}`);
      await select('folder');await (await button('Start scan')).click();
      await browser.waitUntil(async()=>await browser.$('h2').getText()==='Scan complete',{timeout:30000});
      assert.equal((await browser.$$('.results li')).length,51);
      assert.match(await browser.$('.results').getText(),/Threat detected/);
      assert.equal(mixedResponses,51);
      for(const file of extras)rmSync(file);
    });
    await check('mock INFECTED result',async()=>{mode='infected';await scan('Scan complete');assert.match(await browser.$('.results').getText(),/Threat detected/);});
    for(const state of ['error','malformed'])await check(`${state} never becomes CLEAN`,async()=>{mode=state;await scan('Scan incomplete');assert.doesNotMatch(await browser.$('.results').getText(),/Clean/);});
    await check('full scanner timeout never becomes CLEAN',async()=>{mode='delay';await scan('Scan incomplete');assert.doesNotMatch(await browser.$('.results').getText(),/Clean/);});
    await check('Stop lifecycle and scan again',async()=>{
      mode='delay';writeFileSync(fixture,`cancel ${Date.now()}`);await select();await (await button('Start scan')).click();await (await button('Stop')).waitForExist();await (await button('Stop')).click();
      await browser.waitUntil(async()=>await browser.$('h2').getText()==='Scan cancelled',{timeout:10000});
      assert.match(await browser.$('main').getText(),/clean 0/);
      mode='clean';await scan('Scan complete');
    });
    await check('offline never becomes CLEAN',async()=>{for(const s of sockets)s.destroy();await new Promise(r=>server.close(r));server=null;await scan('Scan incomplete');assert.doesNotMatch(await browser.$('.results').getText(),/Clean/);});
  }
  await browser.saveScreenshot(path.join(artifacts,'final.png'));
  writeFileSync(path.join(artifacts,'driver.log'),log.join(''));
} catch(e) {
  if(browser)await browser.saveScreenshot(path.join(artifacts,'failure.png')).catch(()=>{});
  console.error(e); process.exitCode=1;
} finally {
  if(browser)await browser.deleteSession().catch(()=>{});
  if(driver)driver.kill();
  for(const s of sockets)s.destroy();if(server)server.close();
  report();console.log(`Native test artifacts: ${artifacts}`);
}
