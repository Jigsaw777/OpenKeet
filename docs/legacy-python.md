# Legacy Python prototype

For the current Windows desktop app, use [Get started with OpenKeet](getting-started.md). The legacy Python app has a separate interface, settings, and audio path. macOS and Linux prototype paths are source-only and lack current desktop validation.

## Run the prototype

On Windows, see the [legacy Windows guide](../WINDOWS_QUICKSTART.md). On macOS or Linux, use Python 3.11 or 3.12 with Tk. From the repository root:

```sh
python3 -m venv .venv
.venv/bin/python -m pip install -r requirements.txt
.venv/bin/python app.py
```

After setup, `sh start_mac_linux.command` uses the same environment. Check Tk with `python3 -m tkinter`. Linux may also need its Tk, PortAudio, and PulseAudio client libraries.

## Practice with a question

1. Start Ollama and download a supported local Qwen model. In the prototype, select **Local Ollama**, choose the matching tag, and select **Check connection**.
2. Type a practice question and select **Suggest answer**. **Your context** can include a résumé, a job description, and notes. TXT, Markdown, DOCX, and text PDFs are supported.
3. Keep **Save context** off unless you want a local, unencrypted profile. Context becomes part of your prompts when used. Completed questions and answers can be exported to Markdown.

The prototype also contains adapters for installed Claude Code and Codex binaries. They have separate account terms, entitlements, and limits. Use the [Windows desktop provider guide](ai-providers.md) for current supported setup.

## Select an audio source

- **Windows.** Select the playback output carrying the practice call. See the [legacy Windows guide](../WINDOWS_QUICKSTART.md).
- **macOS.** Use a virtual input such as [BlackHole](https://github.com/ExistentialAudio/BlackHole). In Audio MIDI Setup, create a Multi-Output Device for BlackHole and your headphones. Send the mock call there and select BlackHole as the input in OpenKeet.
- **Linux.** Select a PulseAudio or compatible PipeWire monitor for the output that plays the practice call.

In **Audio**, load the local `small.en` model and select **Start listening**. The Windows launcher starts model setup automatically. Stop when finished. [Prototype validation](../VALIDATION.md)
