//! Windows-only synthetic setup acceleration. Public Ask is never replaced.
//! One real note supplies the schema/ACL template; subsequent rows and bodies
//! are bulk-created and protected before the caller runs any production reads.

use anansi_core::{Core, content};
use rusqlite::params;
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    os::windows::process::CommandExt,
    process::{Command, Stdio},
};

pub fn notes(core: &Core, ws: &str, entries: &[(String, String)]) {
    let Some((title, body)) = entries.first() else {
        return;
    };
    let first = core
        .memory_command(
            "create",
            json!({"contractVersion":1,"workspaceId":ws,
        "kind":"note","draft":{"title":title,"body":body,"privacy":"private"}}),
        )
        .unwrap();
    let first_id = first["memory"]["id"].as_str().unwrap();
    let first_revision = first["memory"]["revisionId"].as_str().unwrap();
    let root = core.storage_root().unwrap();
    let template_dir = root.join("content").join(ws).join(first_id);
    let template_file = template_dir.join(format!("{first_revision}.md"));
    let mut conn = rusqlite::Connection::open(root.join("workspaces.sqlite3")).unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    let metadata: String = conn.query_row("SELECT metadata FROM memory_revisions WHERE workspace_id=?1 AND memory_id=?2 AND id=?3",
        params![ws,first_id,first_revision],|r|r.get(0)).unwrap();
    let template: Value = serde_json::from_str(&metadata).unwrap();
    let tx = conn.transaction().unwrap();
    let mut paths = Vec::new();
    for (title, body) in entries.iter().skip(1) {
        let id = content::random_id().unwrap();
        let revision = content::random_id().unwrap();
        let dir = root.join("content").join(ws).join(&id);
        fs::create_dir(&dir).unwrap();
        let file = dir.join(format!("{revision}.md"));
        let mut handle = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&file)
            .unwrap();
        handle.write_all(body.as_bytes()).unwrap();
        handle.sync_all().unwrap();
        let mut meta = template.clone();
        meta["id"] = json!(id);
        meta["revisionId"] = json!(revision);
        meta["title"] = json!(title);
        meta["snippet"] = Value::Null;
        tx.execute(
            "INSERT INTO memories(workspace_id,id,current_revision) VALUES(?1,?2,?3)",
            params![ws, id, revision],
        )
        .unwrap();
        tx.execute("INSERT INTO memory_revisions(workspace_id,memory_id,id,version,metadata,body_hash) VALUES(?1,?2,?3,1,?4,?5)",
            params![ws,id,revision,meta.to_string(),content::hash(body.as_bytes())]).unwrap();
        tx.execute("INSERT INTO memory_body_fts(workspace_id,memory_id,revision_id,title,body) VALUES(?1,?2,?3,?4,?5)",
            params![ws,id,revision,title,body]).unwrap();
        paths.push(json!({"path":dir,"directory":true}));
        paths.push(json!({"path":file,"directory":false}));
    }
    tx.commit().unwrap();
    if paths.is_empty() {
        return;
    }
    let script = r#"
$ErrorActionPreference='Stop'
[Console]::InputEncoding=[Text.UTF8Encoding]::new($false)
$sections=[Security.AccessControl.AccessControlSections]'Access,Owner,Group'
$dirSddl=[IO.Directory]::GetAccessControl($env:ANANSI_SEED_DIR).GetSecurityDescriptorSddlForm($sections)
$fileSddl=[IO.File]::GetAccessControl($env:ANANSI_SEED_FILE).GetSecurityDescriptorSddlForm($sections)
$items=ConvertFrom-Json ([Console]::In.ReadToEnd())
foreach($item in $items) {
 if($item.directory) {
  $acl=New-Object Security.AccessControl.DirectorySecurity
  $acl.SetSecurityDescriptorSddlForm($dirSddl,$sections)
  [IO.Directory]::SetAccessControl($item.path,$acl)
 } else {
  $acl=New-Object Security.AccessControl.FileSecurity
  $acl.SetSecurityDescriptorSddlForm($fileSddl,$sections)
  [IO.File]::SetAccessControl($item.path,$acl)
 }
}
"#;
    let mut child = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env("ANANSI_SEED_DIR", template_dir)
        .env("ANANSI_SEED_FILE", template_file)
        .creation_flags(0x08000000)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&paths).unwrap())
        .unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "synthetic setup ACL protection failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        conn.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        0
    );
}
