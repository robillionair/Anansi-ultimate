# M14 — macOS and ANANSI Core

Status: Tracked planning task for Milestone 14 (following completion of M13 document ingestion). Core crate architecture decoupled; headless daemon CLI and multi-platform service harness scheduled next.

Authority: [master specification](../docs/MASTER_SPEC.md), sections 18, 69–70, 81; [canonical roadmap](../docs/ROADMAP.md).

Required scope: Native Apple Silicon, Keychain, MLX where supported, authenticated pairing/headless operation; enterprise isolation design gate. All applicable cross-cutting constraints and section 82 milestone items remain mandatory.

Dependencies: complete the preceding milestone and verify relevant shared contracts. Before implementation, inventory every bullet in the cited sections, define task-sized vertical slices, exact files, security boundaries, migrations, tests, observable workflow evidence, rollback and acceptance criteria. Record unresolved design choices in decisions; do not invent integrations or claim mocks are live verification.

Planning completion requires requirement-level traceability, explicit deferred sub-slices with rationale, current official dependency/provider verification where relevant, executable verification commands and reviewable Definition of Done. Stop at each approved task boundary. Historical planning baseline follows:
