# Windows recovery verification — 12 November 2026

Application code: `8df1fcf3d69f6d1da532bb890234a7db0352efff`.
[Review and exact limitations](../../reviews/windows-claim-recovery-cr01b-review.md).

These are synthetic Windows test logs, published as text files. Line endings and trailing output whitespace are normalized for publication; the manifest records SHA-256 values of those published UTF-8 text files. Test messages and outcomes are unchanged.

- [Initial broad run: four core failures](claim-recovery-final-windows-lib.txt)
- [Quieter default-concurrency rerun: two core failures](claim-recovery-final-windows-lib-quiet-rerun.txt)
- [Both reviewer probes pass in isolation](claim-recovery-final-windows-reviewer-probes-isolated.txt)
- [Queue tests: 19/19](claim-recovery-final-windows-queue-isolated.txt)
- [UI tests: 347/347](claim-recovery-final-windows-ui.txt)
- [Provider regressions: 32/32 and 38/38](claim-recovery-final-windows-provider-regressions.txt)
- [Recovery UI interaction: 1/1](claim-recovery-final-windows-browser-job-recovery.txt)
- [Connect AI: 1/1](claim-recovery-final-windows-browser-connect-ai.txt)
- [DOCX: 7/7](claim-recovery-final-windows-browser-documents-docx.txt)
- [PDF: 5/5](claim-recovery-final-windows-browser-documents-pdf.txt)
- [TXT/MD: 9/9](claim-recovery-final-windows-browser-documents-ingestion.txt)
- [Ask: 1/1](claim-recovery-final-windows-browser-ask_anansi.txt)

Both broad failures and isolated passes are retained. There is no clean 403-test Windows library pass, real provider-quality benchmark, or native credential-store certification in this evidence.
