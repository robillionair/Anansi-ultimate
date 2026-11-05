import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {DatabaseSync} from 'node:sqlite';
import {verifyApplicationReadiness} from '../../scripts/windows-release/launch-acceptance.mjs';
test('review storage contract: marker alone cannot establish application readiness',()=>{
 const root=fs.mkdtempSync('E:/Anansi/tmp/release-review/marker-contract-');
 fs.writeFileSync(root+'/.anansi-root','x');
 assert.equal(verifyApplicationReadiness(root,[]).ready,false);
});
test('review storage contract: unrelated valid SQLite database cannot establish ANANSI readiness',()=>{
 const root=fs.mkdtempSync('E:/Anansi/tmp/release-review/unrelated-contract-');
 const db=new DatabaseSync(root+'/workspaces.sqlite3');
 db.exec('CREATE TABLE unrelated(value TEXT)');db.close();
 assert.equal(verifyApplicationReadiness(root,[]).ready,false);
});
