# ANANSI Ultimate 2.0 — Enterprise Workspaces

Status · Proposed post-v1 design track under the [2.0 master plan](post-v1-nanzi.md). Begin after 1.0 ships. The first target is one institution operating one private ANANSI deployment. Shared hosting across institutions and disconnected installations need separate designs and acceptance.

This is the security and deployment foundation for [ANANSI Corporate](post-v1-anansi-corporate.md). Corporate adds the permission-aware company file catalog, CRM-style pipelines, tasks and chat-driven actions on this same boundary. It does not introduce a parallel permission system or grant an AI blanket access to company files.

## Product model

Give employees the conversation-first Ask ANANSI interface and give the institution an administration console. Employees search approved material, ask a local model for cited answers, and assign work to approved Nanzis. Administrators grant access through groups and manage AI policies. Auditors review what the system accessed, what it sent to inference, and who approved exceptions.

Present Second Brain as one company knowledge map with separately protected collections. Each employee sees the bubbles, records, citations, and links they can open. A company policy bubble can be widely shared while HR, Sales, Risk, and individual cases each have separate access. Bubbles are views of protected collections. Classification and grouping suggestions go through review before they can affect access.

## Deployment and authority

```text
Managed Windows / browser clients
        |
        v
Institution identity + ANANSI policy enforcement
        |
        +--> Authorized Second Brain collections and indexes
        +--> Approved model workers on institution infrastructure
        +--> Separately administered audit store and export
```

- Host the service, storage, and model workers on infrastructure controlled by the institution. For the first pilot, use a managed internal server with a local model endpoint, institution identity and group membership, encrypted storage and backups, and network egress blocked by default. A model outage surfaces as unavailable. Deployment to a disconnected network can follow once offline identity, model provisioning, updates, recovery, and audit export are tested.
- Treat each employee, Nanzi, ingestion job, and service as an identified actor. Enforce access in the backend for each operation. Desktop and browser use the same policy boundary. The model worker receives bounded input for a request and runs without direct access to the document store, credentials, or arbitrary network tools.
- Separate application administration, audit administration, and host operations. Identify who can inspect process memory, manage encryption keys, change firewall rules, or restore backups in the deployment threat model. Validate those trust boundaries before using sensitive institutional data. [NIST Zero Trust Architecture](https://csrc.nist.gov/pubs/sp/800/207/final)

## Access model

Evaluate **identity and role + department or project membership + information classification + requested action**. Require all applicable grants. Deny by default. Use institution-defined labels, with illustrative tiers such as Internal, Confidential, and Restricted. Department membership and classification are separate checks. A Confidential HR grant has no effect on Confidential Sales records. Approved cross-team work gets a scoped, expiring collection or case grant with an owner and reason.

| Role | Normal authority |
| --- | --- |
| Employee | Read, ask about, or contribute to assigned collections according to explicit permissions. View their own activity and request access. |
| Collection owner | Approve membership and sharing within delegated collections, review classification changes, and revoke access. |
| Organization administrator | Manage groups, models, policy defaults, and provisioning. Content access requires an explicit content grant. |
| Security reviewer or auditor | Search permitted audit metadata, investigate denied actions, and export evidence. Reading protected prompt or response bodies requires a separate logged grant. |
| Nanzi or service job | Use only the intersection of the sponsoring user's current access, its own task grant, and organizational policy. Recheck access when a scheduled task runs. |

Define distinct permissions for discovering, reading, using as AI context, writing, sharing, exporting, reclassifying, and inspecting audit content. Provide a grant preview and explain denials in terms the requester is allowed to see. Require a reason and a short expiry for exceptional access. Use independent approval for emergency access and sensitive downgrades, with notifications and review.

## From a question to an answer

1. The employee signs in and selects a workspace or task. Ask ANANSI shows the active scope and approved model. A model or OpenJEV-inspired classifier can suggest a workflow. The backend makes the authorization decision.
2. Retrieval searches only the permitted partitions. Check every exact record revision before compiling context. Protect filenames, snippet previews, embeddings, cached answers, and graph metadata alongside the body. A denied record must stay absent from search, suggestions, citations, and Second Brain counts.
3. Apply policy to pasted text and attachments as well as retrieved documents. Record a disclosure of the permitted context and send it to the approved institutional model. Treat document and model text as data, and evaluate tool requests against policy before execution. [OWASP sensitive information disclosure](https://genai.owasp.org/llmrisk/llm022025-sensitive-information-disclosure/) · [OWASP prompt injection](https://genai.owasp.org/llmrisk/llm01-prompt-injection/)
4. Scope saved answers, summaries, images, and exports to the most restrictive source classification and applicable audience. For a summary built from several collections, readers need access to all required sources unless an authorized owner approves a separately reviewed redacted copy. Recheck access at read, export, and further inference.
5. When access is revoked, reject new reads immediately after the policy change becomes effective, invalidate caches and queued work, and stop further output or tool actions from affected runs at the next enforcement point. Define and test a revocation time bound, including what happens to tokens already sent and files previously exported.

Example · A Sales employee asks about a client renewal. Ask ANANSI can cite Sales playbooks and authorized account records. It cannot discover HR reviews or customer data assigned to another case. A Risk reviewer can obtain a time-limited grant to the specific renewal case. The reviewer, approval, access scope, and expiry are recorded.

## Audit and oversight

Record allowed and denied actions, including searches, selected records, actual model dispatch, outputs saved or exported, Nanzi runs, permission changes, and audit access. Distinguish a search match, a record opened by a person, and a revision or chunk actually sent to inference. Give each run a correlation ID so an auditor can follow it from request to response and any downstream action.

| Evidence tier | Contents and visibility |
| --- | --- |
| Default activity ledger | Actor and service IDs, role or grant, timestamp, action, resource and revision IDs, selected chunk references, policy version, model identity and destination, approval and outcome, usage when reported, and integrity digests for the payloads. Managers see scoped summaries. Designated reviewers can inspect permitted events. |
| Controlled content capture | For workflows where the institution requires exact reconstruction, optionally capture the submitted prompt, compiled model input, and output in an encrypted evidence store with separate keys and access grants. Exclude credentials and secrets, record redactions, and log every view and export. Show employees what the deployment records. |

Keep audit metadata scoped because it can reveal sensitive relationships and activity. Use append-only storage with tamper-evident checkpoints retained outside the application administrator's control. Monitor gaps and audit-store health. For workflows that require audit, persist a dispatch intent before proceeding and reconcile completion or unknown outcomes after interruption. Define retention, deletion, backup, and incident-export policies for both evidence tiers with the institution before a pilot. [NIST SP 800-53 security and privacy control catalog](https://csrc.nist.gov/pubs/sp/800/53/r5/upd1/final)

## Build sequence and pilot gates

1. **Design and threat model.** Choose one pilot workflow, identify the deployment and trust boundaries, map roles and information classes, and set audit, revocation, and retention rules. Agree on evidence and acceptance with the pilot institution.
2. **Read-only internal pilot.** One institution, two departments, one shared policy collection, and one approved on-premises model. Use synthetic records first. Build sign-in, group grants, permission-filtered Second Brain, cited Ask, and the activity ledger as one workflow. Verify that HR and Sales can use the shared collection but cannot discover each other's data, including through pasted instructions, caches, direct API requests, and guessed IDs.
3. **Governed collaboration.** Add expiring cross-team grants, derived artifact rules, collection-level ingestion, separate auditor controls, and recovery. Prove revocation during queued and active work, audit tamper and outage handling, and restore without widening access. Give enterprise scanning only administrator-approved locations. Classify proposed imports before they become searchable.
4. **Delegated work.** Add a read-only Nanzi using the same identity, policy, and audit path. Then test controlled writes, schedules, and tools as separately approved slices. Use independent abuse and isolation testing before onboarding sensitive live workflows.

The first pilot should answer questions over approved policy and procedure documents. Transaction changes, personnel decisions, and other high-impact actions need separate product decisions, human approval, and review by the institution. Use the references above as design inputs. Each pilot requires its own security and acceptance evidence.
