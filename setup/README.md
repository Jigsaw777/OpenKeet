# OpenKeet setup snapshots

Download the setup file for **Windows 11 x64**, run it, and open **OpenKeet** from Start. These builds are unsigned previews. Setup downloads Microsoft Edge WebView2 if it is missing. [First run](../docs/getting-started.md#download-and-install)

| Snapshot | Direct download | Verification |
| --- | --- | --- |
| Latest | [OpenKeet_x64-setup.exe](https://github.com/Jigsaw777/OpenKeet/releases/latest/download/OpenKeet_x64-setup.exe) | [SHA-256](https://github.com/Jigsaw777/OpenKeet/releases/latest/download/OpenKeet_x64-setup.exe.sha256) · [Release notes](https://github.com/Jigsaw777/OpenKeet/releases/latest) |
| `v0.1.1-preview.1` | [OpenKeet_0.1.1_x64-setup.exe](https://github.com/Jigsaw777/OpenKeet/releases/download/v0.1.1-preview.1/OpenKeet_0.1.1_x64-setup.exe) | [SHA-256](https://github.com/Jigsaw777/OpenKeet/releases/download/v0.1.1-preview.1/OpenKeet_0.1.1_x64-setup.exe.sha256) · [Release notes and notices](https://github.com/Jigsaw777/OpenKeet/releases/tag/v0.1.1-preview.1) |

Each merge to `main` that changes the app or build publishes the next snapshot automatically, with its own release tag, installer, checksum, and notes. [All snapshots](https://github.com/Jigsaw777/OpenKeet/releases) · [Build from source](../docs/getting-started.md#1-build-the-windows-preview)

<details>
<summary>Download preview 1 from PowerShell</summary>

If the browser download times out, run this from your Downloads folder.

```powershell
curl.exe -fL -H "Accept: application/octet-stream" "https://api.github.com/repos/Jigsaw777/OpenKeet/releases/assets/726676993" -o "OpenKeet_0.1.1_x64-setup.exe"
```

</details>
