# Native Windows document picker observation — 2026-11-13

Verdict: PASS for the eight observed picker/import cases below on the exact built app. This is native desktop UI interaction, not browser file injection. No product changes or rebuild.

Application source `6f216e3224e2c855236aac5492f69b467cf8c5d5`, standalone SHA-256 `47e1695d95325f574970331f89d514bb797669dc00021ded50646868901b2a7c`. Ran with a fresh synthetic workspace under E:/Anansi/qualification/native-dialogs-2026-11-13/data and separate WebView2 profile, shared application-test lease. Onboarding completed visibly without a provider. Only Computer Use sky APIs used for window interaction; no file-input injection, hidden IPC setup, or model calls.

## Observed cases

| Case | Actual observation | Result |
| --- | --- | --- |
| Open/cancel | Choose File opened the Windows Open dialog (filename field, Windows navigation, Custom files filter). Escape closed it; no file chosen and no imported document. | PASS |
| Unicode TXT | Native dialog selected café-üñïcode-notes.txt; exact Unicode basename preserved, 108 bytes/108 characters; review and explicit Import document produced revision 1. | PASS |
| Markdown | Native dialog selected notes.md; 182 bytes/182 characters; explicit import completed and record appeared. | PASS |
| PDF | Native dialog selected normal-3page.pdf, 1479 bytes; explicit import completed after server extraction, record appeared. Pre-extraction review reported 0 characters; not confused with final extracted text. | PASS |
| DOCX | Native dialog selected normal.docx, 37050 bytes; explicit import completed after server extraction, record appeared. | PASS |
| Oversized TXT | 70000-byte oversized.txt refused: The file must be at most 65536 bytes. | PASS |
| Empty TXT | empty.txt refused: The file is empty or contains only whitespace. | PASS |
| Unsupported extension | Inert unsupported.exe fixture selected through the filename field (not executed); refused: EXE files are not supported yet. Only .txt, .md, text-layer .pdf and .docx files can be imported in this slice. Other formats are tracked as follow-up work. | PASS |

The dialog visibly filtered the fixture directory to supported file types; selecting an exact unsupported filename tested the application's separate refusal. No OS security barrier was bypassed. Default Private classification and no project retained.

## Persistence and integrity

Closed the owned test app via its window; process exited 0 and lease released. Read-only SQLite plus body-file inspection after close independently confirmed exactly four document revisions (all version 1, expected filenames), exactly four Completed jobs, nonempty extracted bodies and SHA-256 equality for every body. No document/job for cancelled, empty, oversized or unsupported selections. Evidence: final accessibility tree and post-close integrity report, alongside process log. Synthetic catalog retained outside source; not included in delivery.

## Limits

This accepts the observed document-import picker behavior and these representative fixtures. It is not a complete PDF/DOCX hostile-input review, all file-dialog coverage, export/save dialog acceptance, native fresh restore, concurrent ingestion, clean-VM/SmartScreen or upgrade qualification. No credential dialog tested: that needs a user-operated step because the computer-use skill prohibits automating user authentication dialogs. No owner data, credentials, model calls, generated execution, source changes, installer replacement, push, promotion or remote CI. Native credential/keyring and other standing pilot gates remain separate.
