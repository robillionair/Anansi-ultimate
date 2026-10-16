use crate::{
    CommandError, ErrorCode,
    settings::WorkspaceSettings,
    workspace::{Workspace, WorkspaceDetails},
};
use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Component, Path, PathBuf},
    time::Duration,
};

const APPLICATION_ID: i64 = 0x414e414e;
const SCHEMA_VERSION: i64 = 2;
const ROOT_MARKER: &[u8] = b"ANANSI workspace root v1\n";
pub const DATABASE_NAME: &str = "workspaces.sqlite3";

pub fn unavailable() -> CommandError {
    CommandError::new(
        ErrorCode::StorageUnavailable,
        "Local storage is unavailable. Check its configured folder and permissions, then retry.",
    )
}
fn corrupt() -> CommandError {
    CommandError::new(
        ErrorCode::CorruptStorage,
        "Workspace storage cannot be read. Keep the original files and restore a verified backup; no reset was performed.",
    )
}
pub fn reject_links(path: &Path) -> io::Result<()> {
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(meta) => {
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if meta.file_attributes() & 0x400 != 0 {
                        return Err(io::Error::other("Reparse path rejected"));
                    }
                }
                if meta.file_type().is_symlink() {
                    return Err(io::Error::other("Symlink path rejected"));
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
pub fn private_permissions(path: &Path, directory: bool, verify_only: bool) -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let result = std::process::Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                include_str!("protect.ps1"),
            ])
            .env("ANANSI_PERMISSION_PATH", path)
            .env(
                "ANANSI_PERMISSION_DIRECTORY",
                if directory { "1" } else { "0" },
            )
            .env(
                "ANANSI_PERMISSION_VERIFY",
                if verify_only { "1" } else { "0" },
            )
            .creation_flags(0x08000000)
            .output()?;
        if !result.status.success() {
            return Err(io::Error::other(
                "Private permissions could not be verified",
            ));
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = if directory { 0o700 } else { 0o600 };
        if !verify_only {
            fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
        }
        if fs::metadata(path)?.permissions().mode() & 0o777 != mode {
            return Err(io::Error::other("Private writable permissions required"));
        }
    }
    Ok(())
}
fn validate_root(root: &Path) -> Result<(), CommandError> {
    if !root.is_absolute()
        || root
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        return Err(unavailable());
    }
    reject_links(root).map_err(|_| unavailable())?;
    if root
        .ancestors()
        .any(|ancestor| ancestor.join(".git").exists())
    {
        return Err(unavailable());
    }
    Ok(())
}
#[derive(Clone)]
pub struct Storage {
    root: PathBuf,
}
impl Storage {
    pub fn new(root: PathBuf) -> Result<Self, CommandError> {
        validate_root(&root)?;
        Ok(Self { root })
    }
    fn root_ready(&self, create: bool) -> Result<bool, CommandError> {
        validate_root(&self.root)?;
        if !self.root.exists() {
            if !create {
                return Ok(false);
            }
            fs::create_dir_all(&self.root).map_err(|_| unavailable())?;
        }
        let marker = self.root.join(".anansi-root");
        reject_links(&marker).map_err(|_| unavailable())?;
        if !marker.exists() {
            if fs::read_dir(&self.root)
                .map_err(|_| unavailable())?
                .next()
                .is_some()
            {
                return Err(unavailable());
            }
            if !create {
                return Ok(false);
            }
            private_permissions(&self.root, true, false).map_err(|_| unavailable())?;
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&marker).map_err(|_| unavailable())?;
            private_permissions(&marker, false, false).map_err(|_| unavailable())?;
            file.write_all(ROOT_MARKER)
                .and_then(|_| file.sync_all())
                .map_err(|_| unavailable())?;
        }
        private_permissions(&self.root, true, true).map_err(|_| unavailable())?;
        if fs::metadata(&marker).map_err(|_| unavailable())?.len() != ROOT_MARKER.len() as u64
            || fs::read(&marker).map_err(|_| unavailable())? != ROOT_MARKER
        {
            return Err(unavailable());
        }
        Ok(true)
    }
    fn connection(&self, create: bool) -> Result<Option<Connection>, CommandError> {
        if !self.root_ready(create)? {
            return Ok(None);
        }
        let path = self.root.join(DATABASE_NAME);
        for file in [
            path.clone(),
            self.root.join(format!("{DATABASE_NAME}-wal")),
            self.root.join(format!("{DATABASE_NAME}-shm")),
            self.root.join(format!("{DATABASE_NAME}-journal")),
        ] {
            reject_links(&file).map_err(|_| unavailable())?;
        }
        let mut fresh = false;
        if !path.exists() {
            if !create {
                return Ok(None);
            }
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match options.open(&path) {
                Ok(file) => {
                    drop(file);
                    private_permissions(&path, false, false).map_err(|_| unavailable())?;
                    fresh = true;
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
                Err(_) => return Err(unavailable()),
            }
        }
        private_permissions(&path, false, true).map_err(|_| unavailable())?;
        // Inspect existing files read-only before allowing SQLite to recover or write them.
        if !fresh {
            let probe = Connection::open_with_flags(
                &path,
                OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )
            .map_err(|_| corrupt())?;
            probe
                .busy_timeout(Duration::from_secs(5))
                .map_err(|_| unavailable())?;
            probe
                .pragma_update(None, "foreign_keys", true)
                .map_err(|_| unavailable())?;
            check_identity(&probe)?;
        }
        let mut connection = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|_| unavailable())?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(|_| unavailable())?;
        connection
            .pragma_update(None, "foreign_keys", true)
            .map_err(|_| unavailable())?;
        if fresh {
            migrate(
                &mut connection,
                concat!(
                    include_str!("../migrations/0001_workspace.sql"),
                    "\n",
                    include_str!("../migrations/0002_onboarding.sql")
                ),
            )
            .map_err(|_| corrupt())?;
        }
        check_identity(&connection)?;
        let integrity: String = connection
            .query_row("PRAGMA quick_check(1)", [], |row| row.get(0))
            .map_err(|_| corrupt())?;
        if integrity != "ok"
            || connection
                .query_row("PRAGMA foreign_key_check", [], |_| Ok(()))
                .optional()
                .map_err(|_| corrupt())?
                .is_some()
        {
            return Err(corrupt());
        }
        // Only the recognized v1 catalog may move forward; migration is atomic.
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|_| corrupt())?;
        if version == 1 {
            let tx = connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|_| unavailable())?;
            let locked_version: i64 = tx
                .pragma_query_value(None, "user_version", |row| row.get(0))
                .map_err(|_| corrupt())?;
            if locked_version == 1 {
                tx.execute_batch(include_str!("../migrations/0002_onboarding.sql"))
                    .map_err(|_| corrupt())?;
                tx.pragma_update(None, "user_version", SCHEMA_VERSION)
                    .map_err(|_| corrupt())?;
            } else if locked_version != SCHEMA_VERSION {
                return Err(corrupt());
            }
            tx.commit().map_err(|_| unavailable())?;
        }
        connection.prepare("SELECT w.id, w.name, w.name_key, s.reduced_motion FROM workspaces w JOIN workspace_settings s ON s.workspace_id = w.id").map_err(|_| corrupt())?;
        let journal: String = connection
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .map_err(|_| corrupt())?;
        if journal != "wal" {
            connection
                .pragma_update(None, "journal_mode", "WAL")
                .map_err(|_| unavailable())?;
        }
        Ok(Some(connection))
    }
    pub fn list(&self) -> Result<Vec<Workspace>, CommandError> {
        let Some(connection) = self.connection(false)? else {
            return Ok(vec![]);
        };
        let mut statement = connection
            .prepare("SELECT id, name FROM workspaces ORDER BY name_key, id")
            .map_err(|_| corrupt())?;
        statement
            .query_map([], |row| {
                Ok(Workspace {
                    id: row.get(0)?,
                    name: row.get(1)?,
                })
            })
            .map_err(|_| corrupt())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| corrupt())
    }
    pub fn create(&self, name: &str) -> Result<WorkspaceDetails, CommandError> {
        crate::workspace::validate_name(name)?;
        let mut connection = self.connection(true)?.ok_or_else(unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| unavailable())?;
        if tx
            .query_row(
                "SELECT id FROM workspaces WHERE name_key = ?1",
                [name.to_ascii_lowercase()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| corrupt())?
            .is_some()
        {
            return Err(CommandError::new(
                ErrorCode::WorkspaceExists,
                "A workspace with that name already exists. Choose it from the list or use a different name.",
            ));
        }
        let mut random = [0_u8; 16];
        getrandom::fill(&mut random).map_err(|_| unavailable())?;
        let id: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
        tx.execute(
            "INSERT INTO workspaces(id,name,name_key) VALUES(?1,?2,?3)",
            params![id, name, name.to_ascii_lowercase()],
        )
        .map_err(|_| unavailable())?;
        tx.execute(
            "INSERT INTO workspace_settings(workspace_id) VALUES(?1)",
            [&id],
        )
        .map_err(|_| unavailable())?;
        tx.commit().map_err(|_| unavailable())?;
        Self::details(&connection, &id)
    }
    pub fn open(&self, id: &str) -> Result<WorkspaceDetails, CommandError> {
        crate::workspace::validate_id(id)?;
        let connection = self.connection(false)?.ok_or_else(not_found)?;
        Self::details(&connection, id)
    }
    pub fn update(&self, id: &str, reduced_motion: bool) -> Result<WorkspaceDetails, CommandError> {
        self.update_preferences(id, reduced_motion, None, None)
    }
    pub fn update_preferences(
        &self,
        id: &str,
        reduced_motion: bool,
        intent: Option<crate::settings::Intent>,
        onboarding_complete: Option<bool>,
    ) -> Result<WorkspaceDetails, CommandError> {
        crate::workspace::validate_id(id)?;
        let mut connection = self.connection(false)?.ok_or_else(not_found)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| unavailable())?;
        let changed = tx
            .execute(
                "UPDATE workspace_settings SET reduced_motion=?1, intent=COALESCE(?3,intent), onboarding_complete=COALESCE(?4,onboarding_complete) WHERE workspace_id=?2",
                params![reduced_motion, id, intent.map(|value| value.as_str()), onboarding_complete],
            )
            .map_err(|_| corrupt())?;
        if changed != 1 {
            return Err(not_found());
        }
        let details = Self::details(&tx, id)?;
        tx.commit().map_err(|_| unavailable())?;
        Ok(details)
    }
    fn details(connection: &Connection, id: &str) -> Result<WorkspaceDetails, CommandError> {
        connection.query_row("SELECT w.id,w.name,s.reduced_motion,s.intent,s.onboarding_complete FROM workspaces w JOIN workspace_settings s ON s.workspace_id=w.id WHERE w.id=?1", [id], |row| {
            let intent: String = row.get(3)?;
            let intent = serde_json::from_value(serde_json::Value::String(intent)).map_err(|_| rusqlite::Error::InvalidQuery)?;
            Ok(WorkspaceDetails { contract_version:1, schema_version:SCHEMA_VERSION as u32, workspace: Workspace { id:row.get(0)?, name:row.get(1)? }, settings:WorkspaceSettings { reduced_motion:row.get(2)?, intent, onboarding_complete:row.get(4)? } })
        }).optional().map_err(|_| corrupt())?.ok_or_else(not_found)
    }
}
fn not_found() -> CommandError {
    CommandError::new(
        ErrorCode::WorkspaceNotFound,
        "Workspace not found. Refresh the local list and choose an existing workspace.",
    )
}
fn check_identity(connection: &Connection) -> Result<(), CommandError> {
    let id: i64 = connection
        .pragma_query_value(None, "application_id", |row| row.get(0))
        .map_err(|_| corrupt())?;
    if id != APPLICATION_ID {
        return Err(corrupt());
    }
    let version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|_| corrupt())?;
    if !(1..=SCHEMA_VERSION).contains(&version) {
        return Err(CommandError::new(
            ErrorCode::UnsupportedSchema,
            "Storage uses an unsupported schema version. Keep the original files and use a compatible ANANSI version.",
        ));
    }
    Ok(())
}
fn migrate(connection: &mut Connection, sql: &str) -> rusqlite::Result<()> {
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    tx.execute_batch(sql)?;
    tx.pragma_update(None, "application_id", APPLICATION_ID)?;
    tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    tx.commit()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migration_failure_rolls_back_everything() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("CREATE TABLE original(value INTEGER); INSERT INTO original VALUES(42);")
            .unwrap();
        assert!(
            migrate(
                &mut connection,
                "CREATE TABLE partial(id INTEGER); INVALID SQL;"
            )
            .is_err()
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE name='partial'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
        assert_eq!(
            connection
                .query_row("SELECT value FROM original", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            42
        );
        assert_eq!(
            connection
                .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            connection
                .pragma_query_value(None, "application_id", |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    #[test]
    fn every_connection_enforces_foreign_keys_wal_and_busy_timeout() {
        let temp = tempfile::tempdir().unwrap();
        let storage = Storage::new(temp.path().join("store")).unwrap();
        storage.create("Synthetic").unwrap();
        for _ in 0..2 {
            let connection = storage.connection(false).unwrap().unwrap();
            assert_eq!(
                connection
                    .pragma_query_value(None, "foreign_keys", |r| r.get::<_, i64>(0))
                    .unwrap(),
                1
            );
            assert_eq!(
                connection
                    .pragma_query_value(None, "journal_mode", |r| r.get::<_, String>(0))
                    .unwrap(),
                "wal"
            );
            assert_eq!(
                connection
                    .pragma_query_value(None, "busy_timeout", |r| r.get::<_, i64>(0))
                    .unwrap(),
                5000
            );
            assert!(
                connection
                    .execute(
                        "INSERT INTO workspace_settings(workspace_id) VALUES('missing')",
                        []
                    )
                    .is_err()
            );
        }
    }
}
