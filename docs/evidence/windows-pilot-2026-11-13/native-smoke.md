# native-smoke

````text
--- Native Candidate Acceptance Starting ---
Candidate Binary: E:\Anansi\qualification\artifacts\anansi-desktop.exe
Candidate SHA-256: 47e1695d95325f574970331f89d514bb797669dc00021ded50646868901b2a7c (26051072 bytes)
Storage Root: E:\Anansi\qualification\native-nq01-2026-11-13\data
WebView2 UDF: E:\Anansi\qualification\native-nq01-2026-11-13\webview-udf
CDP Port: 19323
Started candidate process PID: 100856
Connected to WebView2 CDP. Version: 157.0.4341.46
[1/7] Startup & Readiness verified. ANANSI Ultimate 0.1.0 core 0.1.0 os windows
[2/7] Shell execution denied by capability boundary. Output: Command plugin:shell|execute not allowed by ACL
[3/7] Created test workspace ID: 2833e09f9874467e972fd1ff78d0a232 Name: Candidate Validation 1794527870240
[4/7] Created memory record ID: 6ed6eccde5b31225d518b0f60cceb47f
[4/7] Memory record verified via search.
[5/7] Created project record ID: 3d81f8e7c695beba839df4df00efbf23
[5/7] Registered artifact ID: 33338263bab6a677a9365febcd0492cb
[5/7] Artifact readback verified via native IPC.
[5/7] Artifact correctly unreadable from a different workspace.
[6/7] Queued backup job ID: a1d434748c7de9b43685909cbe2d20b8
[6/7] Backup job completed.
Restarting candidate process to verify data survival...
Started candidate process PID: 86564
Connected to WebView2 CDP. Version: 157.0.4341.46
[6/7] PASS: Record persisted and survived desktop restart.
[6/7] PASS: Artifact persisted and survived desktop restart.
[7/7] Captured screenshot: test-results/native-candidate-verified.png
--- ALL 7 NATIVE CANDIDATE ACCEPTANCE CHECKS PASSED ---
````
