#!/usr/bin/env python3
"""Optional Ollama-compatible TEXT bridge for Prat011/free-cluely's chat path.

Runs the user's unmodified, subscription-authenticated Claude/Codex CLI.
Does not implement image/audio understanding or change upstream capture behavior.
"""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import threading
from urllib.parse import urlsplit

from core import CopilotError, MODEL_CHOICES, PROVIDERS, build_messages, load_profile
from providers import Providers


def make_handler(provider, model, clients, lock):
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, format, *args):
            # Questions and model responses are never written to access logs.
            pass

        def allowed(self):
            try:
                hostname = urlsplit("http://" + self.headers.get("Host", "")).hostname
            except ValueError:
                return False
            return hostname in ("localhost", "127.0.0.1") and not self.headers.get("Origin")

        def send_json(self, status, value):
            encoded = json.dumps(value).encode("utf-8")
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(encoded)))
            self.send_header("Cache-Control", "no-store")
            self.send_header("X-Content-Type-Options", "nosniff")
            self.end_headers()
            self.wfile.write(encoded)

        def do_GET(self):
            if not self.allowed():
                return self.send_json(403, {"error": "Loopback Node/desktop requests only."})
            if self.path == "/api/tags":
                return self.send_json(200, {"models": [{"name": model, "model": model, "size": 0,
                    "modified_at": datetime.now(timezone.utc).isoformat(),
                    "details": {"family": "subscription-cli-bridge"}}]})
            if self.path == "/api/version":
                return self.send_json(200, {"version": "personal-copilot-bridge-1"})
            return self.send_json(404, {"error": "Text-only bridge: /api/tags and /api/generate are supported."})

        def do_POST(self):
            if not self.allowed():
                return self.send_json(403, {"error": "Loopback Node/desktop requests only."})
            if self.path != "/api/generate":
                return self.send_json(404, {"error": "Only /api/generate text requests are supported."})
            try:
                size = int(self.headers.get("Content-Length", "0"))
                if size < 1 or size > 150000:
                    return self.send_json(413, {"error": "Request must be between 1 and 150,000 bytes."})
                body = json.loads(self.rfile.read(size))
                if not isinstance(body, dict) or not isinstance(body.get("prompt"), str):
                    return self.send_json(400, {"error": "A text prompt is required."})
                if body.get("images"):
                    return self.send_json(400, {"error": "This bridge supports text, not image/audio requests."})
                if body.get("model", model) != model:
                    return self.send_json(400, {"error": "Choose the model returned by /api/tags."})
                if not lock.acquire(blocking=False):
                    return self.send_json(429, {"error": "One question at a time. Wait for the current answer."})
                try:
                    profile = load_profile()
                    profile.update(provider=provider, model=model)
                    messages = build_messages(profile, "Concise answer", body["prompt"])
                    cancel = threading.Event()
                    if body.get("stream", False):
                        self.send_response(200)
                        self.send_header("Content-Type", "application/x-ndjson")
                        self.send_header("Cache-Control", "no-store")
                        self.end_headers()
                        def token(value):
                            item = {"model": model, "response": value, "done": False}
                            try:
                                self.wfile.write((json.dumps(item) + "\n").encode("utf-8"))
                                self.wfile.flush()
                            except (BrokenPipeError, ConnectionResetError):
                                cancel.set()
                                raise
                        try:
                            clients.stream(provider, model, messages, cancel, token)
                            self.wfile.write((json.dumps({"model": model, "response": "", "done": True}) + "\n").encode("utf-8"))
                        except (BrokenPipeError, ConnectionResetError):
                            cancel.set()
                        except Exception:
                            self.wfile.write(b'{"error":"CLI request failed. Check login and plan limits."}\n')
                    else:
                        answer = clients.stream(provider, model, messages, cancel, lambda _: None)
                        self.send_json(200, {"model": model, "response": answer, "done": True})
                finally:
                    lock.release()
            except (ValueError, TypeError, UnicodeError):
                self.send_json(400, {"error": "Invalid JSON request."})
            except CopilotError as exc:
                self.send_json(503, {"error": str(exc)})
            except (BrokenPipeError, ConnectionResetError):
                pass
            except Exception:
                self.send_json(500, {"error": "Bridge failed. Check the CLI connection."})
    return Handler


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--provider", choices=("claude", "codex"), default="claude")
    parser.add_argument("--model", default=None)
    parser.add_argument("--port", type=int, default=11435)
    args = parser.parse_args()
    provider = PROVIDERS[0] if args.provider == "claude" else PROVIDERS[1]
    model = args.model or MODEL_CHOICES[provider][0]
    handler = make_handler(provider, model, Providers(), threading.Lock())
    server = ThreadingHTTPServer(("127.0.0.1", args.port), handler)
    print(f"Text bridge ready at http://127.0.0.1:{args.port}. Provider: {provider}; model: {model}.")
    print("Uses your official CLI login. No API billing fallback. Ctrl+C to stop.")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()


if __name__ == "__main__":
    main()
