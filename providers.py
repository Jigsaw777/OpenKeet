"""Personal CLI subscription adapters; no credential extraction or API fallbacks."""
from __future__ import annotations

from collections import deque
import json
import os
from pathlib import Path
import queue
import re
import shutil
import subprocess
import tempfile
import threading
import time

from core import CopilotError, LocalOllama, PROVIDERS


def subscription_environment() -> dict[str, str]:
    env = os.environ.copy()
    # Avoid choosing separately billed API credentials inherited from a dev shell.
    for name in ("ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN", "ANTHROPIC_BASE_URL",
                 "OPENAI_API_KEY", "CODEX_API_KEY", "OPENAI_BASE_URL",
                 "CLAUDE_CODE_USE_BEDROCK", "CLAUDE_CODE_USE_VERTEX", "CLAUDE_CODE_USE_FOUNDRY"):
        env.pop(name, None)
    env["CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC"] = "1"
    env["MAX_THINKING_TOKENS"] = "0"
    return env


def parse_claude_event(item: dict, streamed: bool) -> tuple[str, str | None, bool]:
    """Return text, failure and whether a successful terminal result was received."""
    kind = item.get("type")
    if kind == "stream_event":
        event = item.get("event", {})
        delta = event.get("delta", {})
        if event.get("type") == "content_block_delta" and delta.get("type") == "text_delta":
            return delta.get("text", ""), None, False
    if kind == "assistant" and not streamed:
        blocks = item.get("message", {}).get("content", [])
        return "".join(b.get("text", "") for b in blocks if b.get("type") == "text"), None, False
    if kind == "result":
        if item.get("is_error") or item.get("subtype", "success") != "success":
            return "", "Claude could not finish. Check plan limits, login and model availability in Claude Code.", False
        return ("" if streamed else item.get("result", "")), None, True
    return "", None, False


def parse_codex_event(item: dict, previous: dict[str, str]) -> tuple[str, str | None, bool]:
    kind = item.get("type")
    if kind in ("error", "turn.failed"):
        return "", "Codex could not finish. Check login, model availability and plan limits in the CLI.", False
    if kind in ("item.updated", "item.completed"):
        value = item.get("item", {})
        if value.get("type") == "agent_message":
            key, text = value.get("id", "answer"), value.get("text", "")
            old = previous.get(key, "")
            previous[key] = text
            return (text[len(old):] if text.startswith(old) else text), None, False
    return "", None, kind == "turn.completed"


class SubscriptionCLI:
    def __init__(self, kind: str):
        self.kind = kind
        self.checked_at = 0.0

    def _executable(self):
        name = "claude" if self.kind == "claude" else "codex"
        path = shutil.which(name)
        if not path:
            raise CopilotError(f"Install {name} CLI and sign in with your subscription first. See README.md.")
        return path

    @staticmethod
    def _options():
        return {"creationflags": subprocess.CREATE_NO_WINDOW} if os.name == "nt" else {}

    def _check_subscription(self, executable, env, cwd):
        if self.checked_at and time.monotonic() - self.checked_at < 300:
            return
        args = [executable, "auth", "status"] if self.kind == "claude" else [executable, "login", "status"]
        try:
            result = subprocess.run(args, capture_output=True, text=True, encoding="utf-8", errors="replace",
                                    env=env, cwd=cwd, timeout=15, **self._options())
        except (OSError, subprocess.TimeoutExpired) as exc:
            raise CopilotError("CLI login check failed. Open the official CLI and verify subscription sign-in.") from exc
        if self.kind == "claude":
            try:
                details = json.loads(result.stdout)
            except ValueError as exc:
                raise CopilotError("Update Claude Code, then run claude auth login with your Claude account.") from exc
            # Console/API-key sign-in is deliberately rejected in subscription mode.
            ready = result.returncode == 0 and details.get("authMethod") in ("claude.ai", "oauth_token")
        else:
            ready = result.returncode == 0 and "chatgpt" in (result.stdout + result.stderr).lower()
        if not ready:
            command = "claude auth login" if self.kind == "claude" else "codex login"
            raise CopilotError(f"Subscription login not confirmed. Run {command} and choose your subscription account.")
        self.checked_at = time.monotonic()

    def command(self, executable: str, model: str) -> list[str]:
        if not re.fullmatch(r"[a-zA-Z0-9][a-zA-Z0-9._:/()\-]{0,119}", model):
            raise CopilotError("Enter a model alias or model ID, without spaces or shell flags.")
        if self.kind == "claude":
            return [executable, "-p", "--safe-mode", "--output-format", "stream-json", "--verbose",
                    "--include-partial-messages", "--tools", "", "--disallowedTools", "mcp__*",
                    "--strict-mcp-config", "--mcp-config", '{"mcpServers":{}}',
                    "--no-session-persistence", "--max-turns", "1", "--model", model]
        result = [executable, "exec", "--json", "--ephemeral", "--ignore-user-config",
                  "--skip-git-repo-check", "--sandbox", "read-only",
                  "-c", 'model_reasoning_effort="low"', "-c", 'web_search="disabled"',
                  "-c", 'features.shell_tool=false', "-c", 'features.apply_patch_freeform=false']
        if model != "default":
            result.extend(["--model", model])
        return result + ["-"]

    def stream(self, model, messages, cancel, on_token):
        if cancel.is_set():
            return ""
        executable, env = self._executable(), subscription_environment()
        # Fresh cwd avoids using a source repository as a question-answering workspace.
        with tempfile.TemporaryDirectory(prefix="personal-copilot-") as directory:
            self._check_subscription(executable, env, directory)
            prompt = ("Answer the final user question using the instructions and reference data in the "
                      "conversation below. Do not use tools, execute commands, inspect files, or browse. "
                      "Do not discuss the CLI. Return just the requested answer.\n" +
                      json.dumps(messages, ensure_ascii=False))
            process = subprocess.Popen(self.command(executable, model), stdin=subprocess.PIPE,
                                       stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
                                       encoding="utf-8", errors="replace", bufsize=1,
                                       cwd=directory, env=env, **self._options())
            events, errors = queue.Queue(), deque(maxlen=12)

            def read_stdout():
                try:
                    for line in process.stdout:
                        events.put(line)
                finally:
                    events.put(None)

            def read_stderr():
                for line in process.stderr:
                    errors.append(line[:1000])

            reader = threading.Thread(target=read_stdout, daemon=True)
            error_reader = threading.Thread(target=read_stderr, daemon=True)
            reader.start()
            error_reader.start()
            text, completed, previous, started = [], False, {}, time.monotonic()
            try:
                try:
                    process.stdin.write(prompt)
                    process.stdin.close()
                except (BrokenPipeError, OSError) as exc:
                    raise CopilotError("CLI exited before receiving the question. Check the CLI version and login.") from exc
                while True:
                    if cancel.is_set():
                        break
                    if time.monotonic() - started > 120:
                        raise CopilotError("The model exceeded the 120 second timeout. Try a smaller model or shorter context.")
                    try:
                        line = events.get(timeout=0.1)
                    except queue.Empty:
                        continue
                    if line is None:
                        break
                    try:
                        event = json.loads(line)
                    except ValueError:
                        continue
                    if self.kind == "claude":
                        delta, failure, done = parse_claude_event(event, bool(text))
                    else:
                        delta, failure, done = parse_codex_event(event, previous)
                    if failure:
                        raise CopilotError(failure)
                    if delta:
                        text.append(delta)
                        on_token(delta)
                    completed = completed or done
                if not cancel.is_set():
                    returncode = process.wait(timeout=5)
                    if returncode or not completed or not text:
                        raise CopilotError("CLI did not return a complete answer. Update it and check login, model access and plan limits.")
                return "".join(text)
            finally:
                if process.poll() is None:
                    process.terminate()
                    try:
                        process.wait(timeout=3)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait(timeout=3)
                reader.join(timeout=1)
                error_reader.join(timeout=1)
                for handle in (process.stdin, process.stdout, process.stderr):
                    if handle and not handle.closed:
                        handle.close()


class Providers:
    def __init__(self):
        self.clients = {PROVIDERS[0]: SubscriptionCLI("claude"),
                        PROVIDERS[1]: SubscriptionCLI("codex"), PROVIDERS[2]: LocalOllama()}

    def stream(self, provider, model, messages, cancel, on_token):
        if provider not in self.clients:
            raise CopilotError("Choose Claude Code, Codex CLI or Ollama.")
        return self.clients[provider].stream(model, messages, cancel, on_token)

    def check(self, provider, model):
        client = self.clients[provider]
        if isinstance(client, LocalOllama):
            models = client.installed_models()
            return "Local models installed: " + (", ".join(models) or "none; run ollama pull qwen3:4b")
        with tempfile.TemporaryDirectory(prefix="personal-copilot-check-") as directory:
            client._check_subscription(client._executable(), subscription_environment(), directory)
        return "Subscription sign-in confirmed. Selected model: " + model + ". Plan limits still apply."
