# ANANSI Corporate — Ultimate 2.0 product plan

Status: owner-requested post-v1 scope, 12 November 2026. Planning only. Start
implementation after ANANSI Ultimate 1.0 ships; this adds no Windows pilot gate.

## Product and acceptance intent

ANANSI Corporate is the company workspace edition of Ultimate 2.0: a governed
Second Brain, company file catalog, CRM-style operating system, and AI work
assistant. Employees can work from Ask ANANSI, while boards and record screens
show and let them edit the same underlying work. The Pipedrive reference is the
clear pipeline-and-activity interaction model, not a claim of compatibility,
integration, or a requirement to reproduce that product.

Before any implementation, each slice must define its exact actors, permitted
sources and actions, evidence, limits, and named acceptance tests. Success means
a real authorized workflow survives restart, recovery, denial and cancellation;
screen mockups or a model's claim that an action succeeded are insufficient.

This extends [Enterprise Workspaces](post-v1-enterprise-workspaces.md), which
remains the authority for institutional deployment, department isolation, model
policy, audit and revocation. It uses the [2.0 Nanzi and chat plan](post-v1-nanzi.md).
There is one shared permission model and one company knowledge system.

## One conversation, connected business views

The primary interface is Ask ANANSI: users ask questions, find documents,
prepare updates, and delegate permitted work. A side panel shows the selected
organization, department, project/customer, model, context and pending action.
The same records remain available in conventional views:

| View | Planned work |
| --- | --- |
| Company knowledge | Search, citations, file locations, revisions and navigable Second Brain bubbles |
| Companies and contacts | Customer and supplier records, owners, communications references and related work |
| Pipelines | Leads/opportunities, configurable stages, assigned owners, values/currencies, activities and stage history |
| Tasks and projects | Assignments, due dates, dependencies, reminders, approvals and linked deliverables |
| Documents | Proposals, contracts, policies and supporting files with exact source and revision provenance |
| Team dashboard | Authorized workload, overdue actions, pipeline totals and unresolved blockers derived from recorded data |
| Administration | Identity, groups, source enrollment, model policy, quotas, retention, backups and scoped audit access |

Teams may later configure an operations, hiring or support pipeline using the
same task and record foundation. Sensitive personnel and financial decisions
remain separately governed; a sales pilot does not qualify those workflows.
Forecasts and AI suggestions must be labelled separately from recorded facts.

## Company file intelligence

The goal is to know where eligible company files are, what they contain, and
which work they support. “Scan the system” means an explicitly enrolled scope:
selected folders, approved shared drives, or all eligible locations on managed
devices. It does not mean silently reading every employee's computer.

1. An authorized owner enrolls a device/share and approves roots, exclusions,
   identities, extraction types, storage and model policy. Native agents or
   approved connectors perform enumeration; browser clients never receive
   arbitrary filesystem capabilities. Service credentials do not grant every
   employee access to the inventoried material.
2. Inventory metadata read-only before optional content analysis. Record stable
   source/device identity, authorized location, file type, size, modified time,
   revision/hash when available, owner and effective access/classification.
   Report inaccessible, excluded, unavailable and unsupported items explicitly.
3. Extract eligible text or opt-in image/OCR descriptions with bounded workers.
   Treat documents as untrusted data; never execute macros or embedded commands.
   Exclude credentials, operating-system locations and protected material by
   default. Local institutional models are the default; cloud disclosure follows
   the existing privacy, authorization and cost gates.
4. Suggest customer/project/topic links and Second Brain bubbles with source
   evidence and uncertainty. Let authorized owners review, correct, exclude,
   split or merge suggestions. AI grouping cannot grant access or downgrade a
   classification. Detect duplicates without deleting originals.
5. Update the catalog incrementally as files change, move or disappear. Distinguish
   the original revision used in an answer from the current file and location.
   Recheck identity and access before opening it. An offline share or stale index
   is reported as such, never as proof that a file does not exist.

The file catalog indexes and references authoritative sources; approved indexed
copies and derived summaries have their own retention and provenance rules.
Scanning never silently moves, renames, deletes or reorganizes original files.
Import undo removes approved derived/indexed material within its policy scope.
Later file-changing tools need separate scope, previews, conflict checks and
approval. Indexing and rescanning use bounded, resumable, cancellable jobs with
visible skips, diagnostics, quotas and storage budgets.

Permission filtering applies to names, paths, previews, embeddings, suggestions,
citations, graph links and counts, not only document bodies. Revocation must stop
further permitted use according to the enterprise enforcement contract. Offline
clients must not claim immediate remote revocation; protected offline access is
a separate policy and acceptance problem.

## CRM and company work records

Create organizations, contacts, opportunities, activities, tasks, projects and
approvals with stable IDs, ownership, field-level validation, revision history
and source references. Link a deal to its proposals, contracts, meeting records,
decisions and next actions without inventing missing relationships.

Offer a pipeline board, record details, activity timeline, search, saved views,
owner filters and dashboards. Keep currencies, units, dates and timezones
explicit. Totals must come from permitted records; suggestions and estimates
carry their assumptions. Moving a card and asking chat to change its stage use
the same validated backend command.

Decide the source of truth per integration: built-in ANANSI records or an
existing CRM mirrored through an approved connector. Do not silently maintain
two authoritative versions. Start with reviewed import/read-only synchronization;
connector writes require separate acceptance for permissions, version conflicts,
idempotency, partial failure and reconciliation. Pipedrive integration is a
possible later connector, not an implemented or automatically authorized one.

## Chat-driven actions and Nanzis

For each request, resolve the actor and scope, retrieve permitted evidence,
propose a bounded structured action, and ask only for genuinely missing details
or required approval. File or model text can suggest work but cannot authorize it.

| Example request | Intended behavior |
| --- | --- |
| “Find the current contract for this customer.” | Return permitted matching files with location, revision, citations and ambiguity when several candidates exist |
| “What should Sales follow up this week?” | Read permitted pipeline/tasks, separate recorded commitments from suggestions, and show evidence |
| “Move this deal to proposal and assign tomorrow's follow-up to Mari.” | Resolve the exact deal/person/date, preview changed fields, then apply permitted version-fenced updates and report actual results |
| “Prepare a proposal using the approved template.” | Draft a versioned artifact citing the selected template and source records; do not send it automatically |
| “Organize this shared drive.” | Propose an approved scan and reviewable virtual organization; changing source files is a separate action |
| “Ask a Nanzi to monitor these accounts.” | Define account scope, allowed reads/actions, schedule, budget and review owner before enabling the routine |

Use allowlisted, typed backend tools rather than arbitrary shell or generated
code. Honor standing authorization for low-impact operations within the task;
require appropriate confirmation/approval for external messages, bulk edits,
sharing, file changes, payments or other high-impact actions. Show the exact
objects and destination before committing. Recheck current permissions and
expected versions at execution, not just when a proposal was created.

Record proposed, approved, executing, completed, failed and unknown outcomes
truthfully. Tool completion is established by the backend or connector receipt.
Use idempotency and reconciliation before retrying an uncertain external write.
Multi-step actions retain per-step results and identify what remains or needs
compensation; an external send cannot be undone by editing local history.

Nanzis use the intersection of user permissions, task grants and organization
policy. Scheduled work rechecks all three at execution. Private department
context must never become shared agent memory through a summary or cached reply.
This does not enable the v1 generated-code execution boundary.

## Corporate deployment and operational foundation

Plan one institution per managed deployment first, with desktop and browser
clients on the same policy boundary. Organization identity/SSO, provisioning,
role/group changes and account removal are separate from deferred ChatGPT OAuth.
Keep approved on-premises/local inference available without a ChatGPT account.

Require organization-defined retention, backup/restore tests, recovery objectives,
service health, safe upgrades/rollback, connector health, scan quotas, inference
budgets and support diagnostics. Keep secrets outside model input and support
exports. Define how audit-store failures affect operations that require audit.
Managers see permitted team activity; administration alone does not grant access
to employee or department content. Follow the enterprise plan for separately
protected evidence and content-capture policy.

Banks, government and other sensitive deployments require their own threat
model, institutional acceptance and applicable review. This product plan is not
a claim of regulatory compliance, certified isolation or production readiness.

## Delivery sequence after v1

1. **Foundation and inventory pilot:** one institution, synthetic HR and Sales
   collections, one approved share and internal model, identity, permission-aware
   catalog, cited chat and audit. Read-only; no external actions.
2. **Corporate workspace:** contacts, companies, one sales pipeline and tasks,
   with conventional views and chat over the same records. Reviewed file links.
3. **Controlled chat updates:** clarify → preview → approve when required →
   version-fenced record update → inspect receipt → reopen history.
4. **Managed source expansion:** incremental endpoint/shared-drive scanning,
   supported photo/OCR analysis, classification review and enterprise connectors.
5. **Delegation:** scoped Nanzis, scheduled work, notifications and approved
   connector writes, each as its own verified vertical slice.

Avoid starting with whole-company indexing plus unrestricted autonomous writes.
No corporate implementation is commissioned by this planning update.

## Required acceptance workflows

- **CORP-01 Inventory:** scan enrolled synthetic roots containing duplicates,
  moved, changed, unsupported, inaccessible and excluded files. Resume after
  interruption, cancel safely, and rescan without duplicates or source mutation.
- **CORP-02 Isolation:** HR and Sales can use a shared policy collection but
  cannot discover each other's names, paths, records, graph counts, cached
  answers or model context, including by direct API and guessed IDs.
- **CORP-03 Grounding:** answers cite exact revisions and known locations;
  outdated, conflicting, unavailable and missing evidence stays explicit.
- **CORP-04 Chat/board parity:** update the same deal through chat and board,
  reject ambiguous identities and stale versions, retain history after restart,
  and never broaden permissions through an AI suggestion.
- **CORP-05 Approval:** opening/editing a proposal changes nothing externally.
  Authorized execution sends only the approved fields/destination, handles
  duplicate submission once, and reconciles interrupted unknown outcomes.
- **CORP-06 Delegation:** Nanzi tasks respect current identity, source, action,
  expiry and spending grants. Revocation stops subsequent protected actions.
- **CORP-07 Recovery:** backup/fresh restore preserves access boundaries,
  provenance, CRM history and audit continuity; disconnected sources remain
  identified rather than silently remapped to unrelated files.
- **CORP-08 Usability and scale:** a staff member completes find → clarify →
  prepare → approve → inspect from chat, with a conventional UI fallback. Freeze
  representative institution-specific corpus sizes, resource budgets and latency
  criteria before benchmarking; retain failures and unsupported cases.
