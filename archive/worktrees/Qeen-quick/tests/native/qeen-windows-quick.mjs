import{remote}from'webdriverio';import{spawn}from'node:child_process';import{mkdtempSync,writeFileSync}from'node:fs';import{tmpdir}from'node:os';import path from'node:path';import assert from'node:assert/strict';
const run=process.env.QEEN_QUICK_RUN_DIR || mkdtempSync(path.join(tmpdir(),'zonendpoint-windows-quick-'));
const env={...process.env,LOCALAPPDATA:path.join(run,'data'),ZONENDPOINT_CLAMD:'127.0.0.1:13310'};
delete env.ZONENDPOINT_TEST_SYSTEM_ROOT;delete env.ZONENDPOINT_TEST_FIXTURE;
const d=spawn('C:/Users/admzon/.cargo/bin/tauri-driver.exe',['--native-driver','E:/Zonendpoint-tools/edgedriver/msedgedriver.exe','--port','4445','--native-port','4446'],{env,windowsHide:true,stdio:'ignore'});
let b;let result;
try{
 for(let i=0;i<100;i++){try{await fetch('http://127.0.0.1:4445/status',{signal:AbortSignal.timeout(1000)});break;}catch{await new Promise(r=>setTimeout(r,100));}}
 b=await remote({hostname:'127.0.0.1',port:4445,logLevel:'error',connectionRetryCount:0,capabilities:{'tauri:options':{application:'E:/Zonendpoint-tools/qeen-target/debug/zonendpoint.exe',webviewOptions:{userDataFolder:path.join(run,'webview')}}}});
 await b.$('button.primary').waitForEnabled({timeout:15000});assert.equal(await b.$('button.primary').getText(),'Quick scan');await b.$('button.primary').click();
 const deadline=Date.now()+900000;let terminal=false;let ticks=0;
 while(Date.now()<deadline){if(await b.$('h2').isExisting()){const heading=await b.$('h2').getText();if(heading!=='Scanning'){terminal=true;break;}if(ticks++%15===0)console.log('SCANNING '+await b.$('.selected-path').getText());}await new Promise(r=>setTimeout(r,1000));}
 if(!terminal){if(await b.$('button=Stop').isExisting())await b.$('button=Stop').click();throw Error('Actual system Quick scan exceeded15-minute QA limit; cancellation requested');}
 const heading=await b.$('h2').getText();const text=await b.$('main').getText();const paths=await b.execute(() => Array.from(document.querySelectorAll('.result-path'), e => e.textContent));
 assert(paths.length>=13);assert(paths.every(p=>/\\windows\\system32\\/i.test(p)), 'Quick scan must stay in native System32 scope');
 const core=['ntoskrnl.exe','ntdll.dll','kernel32.dll','kernelbase.dll','ci.dll','smss.exe','csrss.exe','wininit.exe','services.exe','lsass.exe','svchost.exe','winlogon.exe','winload.efi'];
 for(const name of core)assert(paths.some(p=>p.toLowerCase().endsWith('\\'+name)),'Missing core result '+name);
 const clean=await b.$('.primary.scan-clean').isExisting();if(heading!=='Scan complete')assert.equal(clean,false);
 result={heading,allGood:clean,coreResults:core.length,retainedDetails:paths.length,text,scope:'actual Windows native root; no fixture environment override'};
 writeFileSync(path.join(run,'report.json'),JSON.stringify(result,null,2));await b.saveScreenshot(path.join(run,'result.png'));console.log('ACTUAL_QUICK_RESULT '+JSON.stringify({heading,allGood:clean,retainedDetails:paths.length,artifacts:run}));
}catch(e){console.error(e);process.exitCode=1;}finally{if(b)await b.deleteSession().catch(()=>{});d.kill();console.log('ARTIFACTS '+run);}


