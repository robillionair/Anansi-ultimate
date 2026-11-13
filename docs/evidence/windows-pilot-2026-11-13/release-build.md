# release-build

````text
Starting reproducible isolated Windows release build...
ANANSI RELEASE BUILD START — Commit: 6f216e3224e2c855236aac5492f69b467cf8c5d5 (Dirty: false)
Isolated Build Cache: E:\Anansi\qualification\build-cache
Isolated Temp Dir:    E:\Anansi\qualification\tmp
Artifacts Output:     E:\Anansi\qualification\artifacts
Starting Step 1: Web frontend production build (Vite)...
Web build finished with exit code 0 in 9.00s.
Starting Step 2: API executable compilation (cargo build --release -p anansi-api)...
API build finished with exit code 0 in 330.24s.
Starting Step 3: Desktop executable compilation (Tauri --no-bundle)...
Desktop build finished with exit code 0 in 462.49s.
Starting Step 4: NSIS Installer bundling (Tauri --bundles nsis --ci --no-sign)...
Installer build finished with exit code 0 in 91.06s.
ANANSI RELEASE BUILD FINISHED SUCCESSFULLY in 897.50s.
Manifest written to: E:\Anansi\qualification\artifacts\release-manifest.json
Build successful! Manifest written to E:\Anansi\qualification\artifacts\release-manifest.json
````
