import { spawnSync } from 'node:child_process';
import path from 'node:path';
import os from 'node:os';
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';
export function cargo(args,options={}) {
  const local=path.join(process.env.CARGO_HOME||path.join(os.homedir(),'.cargo'),'bin','cargo'+(process.platform==='win32'?'.exe':''));
  return spawnSync(fs.existsSync(local)?local:'cargo',args,{encoding:'utf8',windowsHide:true,...options});
}
if (process.argv[1] && path.resolve(process.argv[1])===fileURLToPath(import.meta.url)) {
  const result=cargo(process.argv.slice(2),{stdio:'inherit'});process.exitCode=result.status??1;
}
