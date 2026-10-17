-- Semantic version barrier: schema-3 binaries must not rewrite lifecycle records.
-- Keep original revision JSON and immutable content unchanged.
CREATE TABLE lifecycle_format (version INTEGER PRIMARY KEY CHECK(version = 1)) STRICT;
INSERT INTO lifecycle_format VALUES(1);
