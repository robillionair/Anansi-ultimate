import { spawn } from 'node:child_process';
import os from 'node:os';
import path from 'node:path';
const bin=path.join(process.env.CARGO_HOME||path.join(os.homedir(),'.cargo'),'bin');
const env={...process.env,PATH:bin+path.delimiter+process.env.PATH};
const child=spawn(process.execPath,[process.env.npm_execpath,'--filter','@anansi/desktop',process.argv[2]||'build'],{stdio:'inherit',env});
child.on('exit',code=>{process.exitCode=code??1;});
