---
description: Build release and package Endless for distribution
disable-model-invocation: true
allowed-tools: Bash
version: "1.0"
---
Build a release binary and package it with assets/shaders into a zip.

```bash
cd /c/code/endless/rust && k3s-claude cargo-lock build --release 2>&1
```

If build succeeds, package:

```bash
rm -rf /c/code/endless/dist && mkdir -p /c/code/endless/dist/Endless/assets /c/code/endless/dist/Endless/shaders && cp /c/code/endless/rust/target/release/endless.exe /c/code/endless/dist/Endless/ && cp /c/code/endless/assets/*.png /c/code/endless/dist/Endless/assets/ && cp /c/code/endless/shaders/*.wgsl /c/code/endless/dist/Endless/shaders/ && cd /c/code/endless/dist && rm -f Endless.zip && powershell.exe -NoProfile -Command "Compress-Archive -Path 'C:\code\endless\dist\Endless' -DestinationPath 'C:\code\endless\dist\Endless.zip'" && ls -lh Endless.zip
```

Report build errors if any. Confirm zip size and location (`C:\code\endless\dist\Endless.zip`).
