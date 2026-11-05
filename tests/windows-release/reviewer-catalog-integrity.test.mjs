import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {verifyApplicationReadiness,verifyApplicationPersistence} from '../../scripts/windows-release/launch-acceptance.mjs';
test('review: SQLite magic alone cannot establish catalog readiness or persistence',()=>{
  const root=fs.mkdtempSync('E:/Anansi/tmp/release-review/header-only-');
  fs.writeFileSync(root+'/workspaces.sqlite3',Buffer.from('SQLite format 3\0'));
  assert.equal(fs.statSync(root+'/workspaces.sqlite3').size,16);
  assert.equal(verifyApplicationReadiness(root,[]).ready,false,'a header-only non-database must not qualify');
});
test('review: header-only catalog cannot pass restart validation',()=>{
  const root=fs.mkdtempSync('E:/Anansi/tmp/release-review/persist-header-');
  fs.writeFileSync(root+'/workspaces.sqlite3',Buffer.from('SQLite format 3\0'));
  assert.equal(verifyApplicationPersistence(root,['workspaces.sqlite3']).persistent,false);
});
