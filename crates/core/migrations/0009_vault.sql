CREATE TABLE vaults (
  workspace_id TEXT PRIMARY KEY REFERENCES workspaces(id) ON DELETE RESTRICT,
  vault_id TEXT NOT NULL UNIQUE CHECK(length(vault_id)=32),
  active_generation TEXT NOT NULL CHECK(length(active_generation)=32),
  epoch INTEGER NOT NULL CHECK(epoch>0),
  recovery_acknowledged INTEGER NOT NULL CHECK(recovery_acknowledged=1),
  FOREIGN KEY(vault_id,active_generation) REFERENCES vault_generations(vault_id,generation_id) DEFERRABLE INITIALLY DEFERRED
) STRICT;
CREATE TABLE vault_generations (
  vault_id TEXT NOT NULL REFERENCES vaults(vault_id) DEFERRABLE INITIALLY DEFERRED,
  generation_id TEXT NOT NULL CHECK(length(generation_id)=32),
  source_generation_id TEXT NOT NULL CHECK(length(source_generation_id)=32),
  manifest_hash TEXT NOT NULL CHECK(length(manifest_hash)=64),
  passphrase_slot BLOB NOT NULL CHECK(length(passphrase_slot)=210),
  recovery_slot BLOB NOT NULL CHECK(length(recovery_slot)=210),
  PRIMARY KEY(vault_id,generation_id)
) STRICT;
CREATE TABLE vault_members (
  vault_id TEXT NOT NULL,
  generation_id TEXT NOT NULL,
  record_id TEXT NOT NULL CHECK(length(record_id)=32),
  PRIMARY KEY(vault_id,generation_id,record_id),
  FOREIGN KEY(vault_id,generation_id) REFERENCES vault_generations(vault_id,generation_id) ON DELETE RESTRICT
) STRICT;
-- Unassociated opaque file inventory supports locked ciphertext backup without
-- exposing revision pointers, per-record history counts or relationship metadata.
CREATE TABLE vault_files (
  vault_id TEXT NOT NULL,
  generation_id TEXT NOT NULL,
  token TEXT NOT NULL CHECK(token='manifest' OR length(token)=32),
  envelope_hash TEXT NOT NULL CHECK(length(envelope_hash)=64),
  byte_length INTEGER NOT NULL CHECK(byte_length BETWEEN 80 AND 8388688),
  PRIMARY KEY(vault_id,generation_id,token),
  FOREIGN KEY(vault_id,generation_id) REFERENCES vault_generations(vault_id,generation_id) ON DELETE RESTRICT
) STRICT;
-- History counts, revision pointers, edge IDs/labels and all domain metadata stay
-- inside the encrypted generation manifest/payloads, never this plaintext index.
CREATE TABLE vault_maintenance (singleton INTEGER PRIMARY KEY CHECK(singleton=1), enabled INTEGER NOT NULL CHECK(enabled IN (0,1))) STRICT;
INSERT INTO vault_maintenance VALUES(1,0);
CREATE TRIGGER vault_block_memories_insert BEFORE INSERT ON memories WHEN (SELECT enabled FROM vault_maintenance)=0 BEGIN SELECT RAISE(ABORT,'Schema 9 requires vault-aware writes'); END;
CREATE TRIGGER vault_block_memories_update BEFORE UPDATE ON memories WHEN (SELECT enabled FROM vault_maintenance)=0 BEGIN SELECT RAISE(ABORT,'Schema 9 requires vault-aware writes'); END;
CREATE TRIGGER vault_block_memories_delete BEFORE DELETE ON memories WHEN (SELECT enabled FROM vault_maintenance)=0 BEGIN SELECT RAISE(ABORT,'Schema 9 requires vault-aware writes'); END;
CREATE TRIGGER vault_block_revisions_insert BEFORE INSERT ON memory_revisions WHEN (SELECT enabled FROM vault_maintenance)=0 BEGIN SELECT RAISE(ABORT,'Schema 9 requires vault-aware writes'); END;
CREATE TRIGGER vault_block_revisions_update BEFORE UPDATE ON memory_revisions WHEN (SELECT enabled FROM vault_maintenance)=0 BEGIN SELECT RAISE(ABORT,'Schema 9 requires vault-aware writes'); END;
CREATE TRIGGER vault_block_revisions_delete BEFORE DELETE ON memory_revisions WHEN (SELECT enabled FROM vault_maintenance)=0 BEGIN SELECT RAISE(ABORT,'Schema 9 requires vault-aware writes'); END;
CREATE TRIGGER vault_block_edges_insert BEFORE INSERT ON relationships WHEN (SELECT enabled FROM vault_maintenance)=0 BEGIN SELECT RAISE(ABORT,'Schema 9 requires vault-aware writes'); END;
CREATE TRIGGER vault_block_edges_update BEFORE UPDATE ON relationships WHEN (SELECT enabled FROM vault_maintenance)=0 BEGIN SELECT RAISE(ABORT,'Schema 9 requires vault-aware writes'); END;
CREATE TRIGGER vault_block_edges_delete BEFORE DELETE ON relationships WHEN (SELECT enabled FROM vault_maintenance)=0 BEGIN SELECT RAISE(ABORT,'Schema 9 requires vault-aware writes'); END;
CREATE TRIGGER vault_block_edge_history_insert BEFORE INSERT ON relationship_revisions WHEN (SELECT enabled FROM vault_maintenance)=0 BEGIN SELECT RAISE(ABORT,'Schema 9 requires vault-aware writes'); END;
CREATE TRIGGER vault_block_edge_history_update BEFORE UPDATE ON relationship_revisions WHEN (SELECT enabled FROM vault_maintenance)=0 BEGIN SELECT RAISE(ABORT,'Schema 9 requires vault-aware writes'); END;
CREATE TRIGGER vault_block_edge_history_delete BEFORE DELETE ON relationship_revisions WHEN (SELECT enabled FROM vault_maintenance)=0 BEGIN SELECT RAISE(ABORT,'Schema 9 requires vault-aware writes'); END;
