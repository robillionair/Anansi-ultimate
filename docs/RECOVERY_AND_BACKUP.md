# Source recovery and backup

This implements project source protection only. Workspace, application, product-artifact and credential-aware backups remain separate M2/M7/M15 requirements (master section 76). An archive inside .git is not independent protection. Do not put real memories/secrets into committed history; a bundle is not encrypted and is not a secret-scanning service.

## Create and verify a source bundle

Use a clean committed checkout, Git and PowerShell 7. The destination must be an explicit absolute new .bundle path in an existing independent directory outside this checkout and .git. Symlink/junction ancestors and existing destination files are refused. No directory is automatically created; no existing backup is overwritten. Native commands receive argument arrays; there is no shell-string evaluation.

~~~powershell
pnpm backup:source -Destination 'C:\Backups\ANANSI\source-unique.bundle'
git bundle verify 'C:\Backups\ANANSI\source-unique.bundle'
git bundle list-heads 'C:\Backups\ANANSI\source-unique.bundle'
git clone 'C:\Backups\ANANSI\source-unique.bundle' 'C:\Recovery\ANANSI-new'
git -C 'C:\Recovery\ANANSI-new' rev-parse HEAD
git -C 'C:\Recovery\ANANSI-new' rev-parse 'HEAD^{tree}'
~~~

Replace example locations with existing independent parents and unused names. The command emits checkpoint, tree, SHA256 and captured source refs after verification. The bundle preserves local branches, remote-tracking refs, tags and HEAD with their history; it excludes stash/tool-private refs and raw Git configuration. Remote-tracking refs are available through bundle fetch but are not automatically recreated as local branches in a normal clone. Default clone HEAD follows the backed-up source HEAD; explicitly switch master or the recorded checkpoint when desired.

Compare restored HEAD/tree with the emitted record, then in the restored directory run frozen install, pnpm check and pnpm test. Review git ls-files for unexpected credential/database paths; ignore rules are not proof that old committed history is free of secrets. The restored clone's origin points to the bundle: set the authorized GitHub origin deliberately before remote recovery work.

## Dirty-source and failure recovery

The tool refuses all nonignored uncommitted changes. Commit reviewed source when appropriate. Otherwise export tracked changes separately and explicitly copy necessary untracked source files to a protected independent location before retrying; the bundle never includes them. Never blindly archive ignored private data.

~~~powershell
git diff --binary --output='C:\Recovery\tracked-working.patch'
git diff --cached --binary --output='C:\Recovery\tracked-index.patch'
git ls-files --others --exclude-standard
~~~

Restore into a new directory, inspect patches and use git apply --check before applying reviewed content; restore untracked files separately. These patches can contain sensitive content and need appropriate storage. The two patches capture different stages: apply the index patch first, then the working-tree patch. Never reset the original repository to make the backup script accept a dirty tree.

For missing/corrupt bundles, preserve originals, verify an earlier bundle or clone the authorized remote into a fresh directory. Failed backup creation leaves the destination unmodified and removes only its unique staging file. Failed restoration must not overwrite the current checkout. Keep drill output or remove only a positively verified task-owned path; avoid recursive deletion of computed directories.

For CI failures, repair on the task branch and wait for both checks on the new SHA. For divergence, fetch and inspect both histories; merge deliberately without reset, force push, tag movement or protection bypass. Existing tags must never be overwritten.

## Remote recovery and milestone boundary

~~~powershell
git clone https://github.com/robillionair/Anansi-ultimate.git 'C:\Recovery\ANANSI-remote-new'
git -C 'C:\Recovery\ANANSI-remote-new' log -5 --oneline
git ls-remote origin refs/heads/master
~~~

Only mark M0 complete after the actual local restore/checks, Windows/Linux CI and master protection have been verified. Then create the annotated m0-foundation-verified tag on the verified checkpoint and push that tag without force. Stop before M1. Actual tested paths, hashes, commits and run URLs belong in [M0 evidence](evidence/M0.md).

Keep an independent local bundle and the authorized remote. Two copies on the same device do not protect against device loss; product data and secrets require their own backup policy. Back up new milestones again; a prior bundle cannot contain future commits/tags.
