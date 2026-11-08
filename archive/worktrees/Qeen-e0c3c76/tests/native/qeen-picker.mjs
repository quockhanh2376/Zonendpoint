import {remote} from 'webdriverio';
import {spawn} from 'node:child_process';
import {mkdtempSync,mkdirSync,writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import path from 'node:path';
import assert from 'node:assert/strict';
const root=mkdtempSync(path.join(tmpdir(),'zonendpoint-real-picker-'));
const fixtures=path.join(root,'fixtures');mkdirSync(fixtures);
const fixture=path.join(fixtures,'Kiểm thử — benign.txt');writeFileSync(fixture,'Native picker unicode benign fixture');
const env={...process.env,LOCALAPPDATA:path.join(root,'appdata'),ZONENDPOINT_CLAMD:'127.0.0.1:13310'};
delete env.ZONENDPOINT_TEST_FIXTURE;
const driver=spawn('C:/Users/admzon/.cargo/bin/tauri-driver.exe',['--native-driver','E:/Zonendpoint-tools/edgedriver/msedgedriver.exe','--port','4445','--native-port','4446'],{env,windowsHide:true,stdio:'ignore'});
let b;
try{
 for(let i=0;i<100;i++){try{await fetch('http://127.0.0.1:4445/status',{signal:AbortSignal.timeout(1000)});break;}catch{await new Promise(r=>setTimeout(r,100));}}
 b=await remote({hostname:'127.0.0.1',port:4445,logLevel:'error',connectionRetryCount:0,capabilities:{'tauri:options':{application:'E:/Zonendpoint-tools/qeen-target/debug/zonendpoint.exe',webviewOptions:{userDataFolder:path.join(root,'webview')}}}});
 await b.$('h1').waitForExist({timeout:15000});
 for(const kind of ['File','Folder']){
  await b.$('button=Select target ▾').click();await b.$(`button=${kind}`).click();
  console.log(`WAIT_${kind.toUpperCase()}_PICKER: ${kind==='File'?fixture:fixtures}`);
  await b.waitUntil(async()=>(await b.$('.selected-path').getText()).includes(kind==='File'?'benign.txt':fixtures),{timeout:180000});
  await b.$('button.primary').waitForEnabled();await b.$('button.primary').click();
  await b.waitUntil(async()=>await b.$('button.primary').getText()==='All Good',{timeout:20000});
  assert.match(await b.$('.results').getText(),/Clean in this scan/);
  console.log(`PASS_REAL_${kind.toUpperCase()}_PICKER`);
  await b.saveScreenshot(path.join(root,`${kind.toLowerCase()}.png`));
 }
 console.log('PICKER_ARTIFACTS '+root);
}catch(e){console.error(e);process.exitCode=1;if(b)await b.saveScreenshot(path.join(root,'failure.png')).catch(()=>{});}
finally{if(b)await b.deleteSession().catch(()=>{});driver.kill();}
