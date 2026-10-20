// One shared native/browser test lease outside every checkout. Never steal it
// based on age/PID; an interrupted lease requires explicit owner reconciliation.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {spawn} from 'node:child_process';
import {randomUUID} from 'node:crypto';

const leasePath=path.join(os.homedir(),'.codex','anansi-application-test.lease.json');
const [script,...args]=process.argv.slice(2);
if(!script)throw Error('Supply a Node verification script and arguments.');
const identity=randomUUID();
const record={identity,owner:'M2-M4 integration',pid:process.pid,startedAt:new Date().toISOString(),cwd:process.cwd(),script,args};
let descriptor;
try{descriptor=fs.openSync(leasePath,'wx',0o600);}catch(error){
  if(error.code==='EEXIST')throw Error('Application test lease occupied: '+leasePath+'. Continue independent checks; do not steal it.');
  throw error;
}
try{
  fs.writeFileSync(descriptor,JSON.stringify(record,null,2));fs.fsyncSync(descriptor);fs.closeSync(descriptor);
  const child=spawn(process.execPath,[script,...args],{stdio:'inherit',windowsHide:true});
  const code=await new Promise((resolve,reject)=>{child.once('error',reject);child.once('exit',exit=>resolve(exit??1));});
  process.exitCode=code;
}finally{
  const current=JSON.parse(fs.readFileSync(leasePath,'utf8'));
  if(current.identity===identity)fs.unlinkSync(leasePath);
}
