# ANANSI Ultimate — Recovery Guide

1. Read `AGENTS.md`, `docs/MASTER_SPEC.md`, `docs/BUILD_STATUS.md`, the active plan, and `docs/DECISIONS.md`.
2. Check `git status` and inspect the last verified commit recorded in build status.
3. Do not discard uncommitted work. Identify the active phase and its acceptance criteria.
4. Re-run the phase's verification commands before changing code.
5. Continue only the unfinished acceptance criteria, then record evidence and update status.
6. If requirements conflict, stop implementation and record the conflict in `docs/DECISIONS.md`.

Recovery checkpoints should contain source, tests, and the corresponding status/documentation updates. Never treat a build artifact alone as a recoverable project state.

## Reconciled resume entrypoint

Begin with [the active M1 plan](../plans/PHASE-001-runnable-shell.md), the active/next task in project-state.json, and [the canonical roadmap](ROADMAP.md). M0.1-M0.3 are complete; M1.0 detailed planning/readiness audit is complete; next is M1.1 native-tool verification. Read the detailed acceptance plan before coding. Milestone IDs follow master section 82; legacy numeric plan filenames do not define milestone numbers. Run `node --test tests/repository-hygiene.test.mjs` and `node scripts/validate-planning.mjs --self-test` for current foundation checks; these do not run product tests or replace M0 CI/restore checks.

If a documentation checkpoint records the previous verified implementation commit, inspect subsequent metadata commits as well. Never reset to a recorded SHA just because it is older than HEAD. Fresh recovery uses `git clone https://github.com/robillionair/Anansi-ultimate.git`; bundle recovery uses `git bundle verify <bundle>` followed by `git clone <bundle> <new-directory>`. M0 independent backup/restore has been proved and recorded in docs/evidence/M0.md; follow RECOVERY_AND_BACKUP.md to restore or create subsequent source backups.
