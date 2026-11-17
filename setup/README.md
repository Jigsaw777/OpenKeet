# OpenKeet setup snapshots

Download the setup file for **Windows 11 x64**, run it, and open **OpenKeet** from Start. These builds are unsigned. Setup downloads Microsoft Edge WebView2 if it is missing. [First run](../docs/getting-started.md#download-and-install)

| Version | Released | Direct download | Verification |
| --- | --- | --- | --- |
| Latest | Changes with each release | [OpenKeet_x64-setup.exe](https://github.com/Jigsaw777/OpenKeet/releases/latest/download/OpenKeet_x64-setup.exe) | [SHA-256](https://github.com/Jigsaw777/OpenKeet/releases/latest/download/OpenKeet_x64-setup.exe.sha256) · [Release notes](https://github.com/Jigsaw777/OpenKeet/releases/latest) |
| `v0.3` | 14 November 2026 | [OpenKeet_x64-setup.exe](https://github.com/Jigsaw777/OpenKeet/releases/download/v0.3/OpenKeet_x64-setup.exe) | [SHA-256](https://github.com/Jigsaw777/OpenKeet/releases/download/v0.3/OpenKeet_x64-setup.exe.sha256) · [Release notes](https://github.com/Jigsaw777/OpenKeet/releases/tag/v0.3) |
| `v0.2` | 10 November 2026 | [OpenKeet_x64-setup.exe](https://github.com/Jigsaw777/OpenKeet/releases/download/v0.2/OpenKeet_x64-setup.exe) | [SHA-256](https://github.com/Jigsaw777/OpenKeet/releases/download/v0.2/OpenKeet_x64-setup.exe.sha256) · [Release notes](https://github.com/Jigsaw777/OpenKeet/releases/tag/v0.2) |
| `v0.1.1-preview.2` | 10 November 2026 | [OpenKeet_x64-setup.exe](https://github.com/Jigsaw777/OpenKeet/releases/download/v0.1.1-preview.2/OpenKeet_x64-setup.exe) | [SHA-256](https://github.com/Jigsaw777/OpenKeet/releases/download/v0.1.1-preview.2/OpenKeet_x64-setup.exe.sha256) · [Release notes](https://github.com/Jigsaw777/OpenKeet/releases/tag/v0.1.1-preview.2) |
| `v0.1.1-preview.1` | 9 November 2026 | [OpenKeet_0.1.1_x64-setup.exe](https://github.com/Jigsaw777/OpenKeet/releases/download/v0.1.1-preview.1/OpenKeet_0.1.1_x64-setup.exe) | [SHA-256](https://github.com/Jigsaw777/OpenKeet/releases/download/v0.1.1-preview.1/OpenKeet_0.1.1_x64-setup.exe.sha256) · [Release notes and notices](https://github.com/Jigsaw777/OpenKeet/releases/tag/v0.1.1-preview.1) |

The table lists every published version, newest first. The first two were published as previews before plain `vMAJOR.MINOR` numbering started with `v0.2`. Each merge to `main` that changes the app or build publishes the next version automatically, with its own tag, installer, checksum, and notes. The **Latest** row always points at the newest one, including a version not yet listed here. [All versions](https://github.com/Jigsaw777/OpenKeet/releases) · [Build from source](../docs/getting-started.md#1-build-the-windows-preview)

<details>
<summary>Download the latest setup file from PowerShell</summary>

If the browser download times out, run this from your Downloads folder.

```powershell
curl.exe -fL "https://github.com/Jigsaw777/OpenKeet/releases/latest/download/OpenKeet_x64-setup.exe" -o "OpenKeet_x64-setup.exe"
```

</details>
