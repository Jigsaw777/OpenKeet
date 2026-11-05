#!/usr/bin/env python3
"""Personal Interview Assistant for local transcription and text answers."""
from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import queue
import shutil
import sys
import threading
import time

from core import (CONTEXT_LIMIT, MODEL_CHOICES, MODELS, MODES, PRESETS, PROVIDERS, PROFILE_PATH, CopilotError,
                  LocalOllama, build_messages, clean_profile, load_profile,
                  read_document, save_profile)


APP_NAME = "Personal Interview Assistant"


def diagnostics():
    print(APP_NAME + " diagnostics")
    print("Python:", sys.version.split()[0])
    for module in ("tkinter", "pypdf", "numpy", "sounddevice", "soundcard", "faster_whisper"):
        print(module + ":", "installed" if importlib.util.find_spec(module) else "not installed")
    for command in ("claude", "codex"):
        print(command + " CLI:", "installed" if shutil.which(command) else "not installed")
    try:
        print("Supported installed models:", ", ".join(LocalOllama().installed_models()) or "none")
    except CopilotError as exc:
        print("Ollama:", exc)


if "--diagnostics" in sys.argv:
    diagnostics()
    raise SystemExit(0)

try:
    import tkinter as tk
    from tkinter import filedialog, ttk
    from tkinter.scrolledtext import ScrolledText
except ImportError:
    raise SystemExit("Tkinter is missing. Install a Python build with Tk support; see README.md.")

from audio import DEFAULT_SPEECH_MODEL, SPEECH_MODELS, SpeechListener, input_devices, loopback_devices
from capture_privacy import set_capture_exclusion
from live import LiveAnswerPlanner, QuestionBuffer, SemanticResponseDecoder, commit_checked_answer
from providers import Providers

BG, PANEL, FG, MUTED, ACCENT = "#111827", "#1b2538", "#edf3ff", "#b0bfd4", "#78e8ba"


class App:
    def __init__(self, root):
        self.root = root
        self.root.title(APP_NAME)
        width = min(960, root.winfo_screenwidth() - 40)
        height = min(840, root.winfo_screenheight() - 80)
        root.geometry(f"{width}x{height}")
        root.minsize(700, 600)
        root.configure(bg=BG)
        self.events = queue.Queue()
        self.client = Providers()
        self.claude_signing_in = False
        self.cancel = threading.Event()
        self.history = []
        self.audio_answers = LiveAnswerPlanner()
        self.questions = QuestionBuffer()
        self.current_draft = False
        self.current_audio_request = None
        self.current_semantic = None
        self.current_obsolete = False
        self.superseded = False
        self.last_final_question = None
        self.previous_final_snapshot = None
        self.context_signature = None
        self.busy = False
        self.audio_loading = False
        self.audio_active = False
        self.audio_stopping = False
        self.last_transcript = ""
        self.last_speech_end = None
        self.current_speech_end = None
        self.first_token_seconds = None
        self.latency_note = ""
        self.pending_speech = ""
        self.auto_timer = None
        self.started = 0.0
        self.current_question = ""
        self.current_answer = ""
        self.last_answer = ""
        self.device_map = {}
        self.listener = SpeechListener(lambda kind, data: self.events.put((kind, data)))
        try:
            self.initial_profile = load_profile()
            initial_error = None
        except CopilotError as exc:
            self.initial_profile, initial_error = clean_profile({}), str(exc)
        self.model = tk.StringVar(value=self.initial_profile["model"])
        self.provider = tk.StringVar(value=self.initial_profile["provider"])
        self.preset = tk.StringVar(value=self.initial_profile["preset"])
        self.mode = tk.StringVar(value=next(iter(MODES)))
        self.status = tk.StringVar(value="Add your context, check your provider connection, then ask or start listening.")
        self.audio_status = tk.StringVar(value="Optional: load the local speech model to enable listening.")
        self.pin = tk.BooleanVar(value=False)
        self.capture_exclusion = tk.BooleanVar(value=sys.platform == "win32")
        self.privacy_status = tk.StringVar(value="Capture exclusion is unavailable on this platform.")
        self.capture_windows = {}
        if sys.platform == "win32":
            self.capture_recheck = root.register(self.recheck_capture)
            self.capture_destroy = root.register(self.forget_capture)
            # Tk may recreate a combobox wrapper after postcommand, just before deiconify.
            root.tk.call("trace", "add", "execution", "wm", "enter", root.register(self.before_wm))
        self.auto = tk.BooleanVar(value=True)
        self.live_drafts = tk.BooleanVar(value=True)
        self.live_question = tk.StringVar(value="Live transcript appears here while the speaker is talking.")
        self.answer_heading = tk.StringVar(value="Suggested answer")
        self.default_route = "Default microphone" if sys.platform == "darwin" else "System output (default)"
        self.device = tk.StringVar(value=self.default_route)
        self.speech_model = tk.StringVar(value=DEFAULT_SPEECH_MODEL)
        self.threshold = tk.DoubleVar(value=0.012)
        self.question_pause = tk.DoubleVar(value=1.2)
        self._styles()
        self._layout()
        if sys.platform == "win32":
            self.protect_window(str(root))
        if initial_error:
            self.status.set(initial_error)
        root.bind("<Control-Return>", lambda e: self.generate())
        if sys.platform == "darwin":
            root.bind("<Command-Return>", lambda e: self.generate())
        root.protocol("WM_DELETE_WINDOW", self.close)
        root.after(60, self.pump)

    def _styles(self):
        style = ttk.Style()
        style.theme_use("clam")
        style.configure(".", background=BG, foreground=FG, font=("TkDefaultFont", 11))
        style.configure("TFrame", background=BG)
        style.configure("TLabel", background=BG, foreground=FG)
        style.configure("Muted.TLabel", foreground=MUTED, font=("TkDefaultFont", 10))
        style.configure("Title.TLabel", foreground=ACCENT, font=("TkDefaultFont", 19, "bold"))
        style.configure("TButton", background=PANEL, foreground=FG, padding=(12, 7))
        style.map("TButton", background=[("active", "#34445c")], foreground=[("disabled", "#8795ab")])
        style.configure("Accent.TButton", background=ACCENT, foreground=BG)
        style.map("Accent.TButton", background=[("active", "#a0f5d1")])
        style.configure("TNotebook", background=BG, borderwidth=0)
        style.configure("TNotebook.Tab", padding=(15, 9), background=PANEL)
        style.map("TNotebook.Tab", background=[("selected", "#34445c")])
        style.configure("TCombobox", fieldbackground=PANEL, foreground=FG, arrowcolor=FG)
        style.map("TCombobox", fieldbackground=[("readonly", PANEL)],
                  foreground=[("readonly", FG)], selectbackground=[("readonly", PANEL)],
                  selectforeground=[("readonly", FG)])
        style.configure("TCheckbutton", background=BG, foreground=FG)
        style.map("TCheckbutton", background=[("active", BG)])

    def _text(self, parent, height=5):
        box = ScrolledText(parent, height=height, wrap="word", font=("TkDefaultFont", 12),
                           bg=PANEL, fg=FG, insertbackground=ACCENT, selectbackground="#375675",
                           relief="flat", padx=12, pady=10, undo=True)
        return box

    def _layout(self):
        outer = ttk.Frame(self.root, padding=18)
        outer.pack(fill="both", expand=True)
        header = ttk.Frame(outer)
        header.pack(fill="x", pady=(0, 12))
        ttk.Label(header, text=APP_NAME, style="Title.TLabel").pack(side="left")
        ttk.Checkbutton(header, text="Keep on top", variable=self.pin,
                        command=lambda: self.root.attributes("-topmost", self.pin.get())).pack(side="right")
        self.provider_note = ttk.Label(outer, style="Muted.TLabel", wraplength=850)
        self.provider_note.pack(anchor="w", pady=(0, 10))
        if sys.platform == "win32":
            ttk.Checkbutton(outer, text="Exclude this window from supported Windows captures (verify with your meeting app)",
                            variable=self.capture_exclusion, command=self.toggle_capture).pack(anchor="w", pady=(0, 8))
            ttk.Label(outer, textvariable=self.privacy_status, style="Muted.TLabel",
                      wraplength=850).pack(anchor="w", pady=(0, 8))
        else:
            ttk.Label(outer, text="Screen sharing: share a specific app window and keep the assistant outside it. Whole-screen invisibility is not guaranteed.",
                      style="Muted.TLabel", wraplength=850).pack(anchor="w", pady=(0, 8))
        controls = ttk.Frame(outer)
        controls.pack(fill="x", pady=(0, 12))
        provider_combo = self.combobox(controls, textvariable=self.provider, values=PROVIDERS,
                                      state="readonly", width=25)
        provider_combo.pack(side="left", padx=(0, 10))
        provider_combo.bind("<<ComboboxSelected>>", self.change_provider)
        self.model_combo = self.combobox(controls, textvariable=self.model,
                                        values=MODEL_CHOICES[self.provider.get()], width=18)
        self.model_combo.pack(side="left")
        ttk.Button(controls, text="Check connection", command=self.check_model).pack(side="left", padx=10)
        if sys.platform == "win32":
            self.claude_signin_button = ttk.Button(controls, text="Sign in to Claude", command=self.sign_in_to_claude)
        self.change_provider(reset=False)
        self.notebook = ttk.Notebook(outer)
        self.notebook.pack(fill="both", expand=True)
        assistant, context, audio = (ttk.Frame(self.notebook, padding=12) for _ in range(3))
        for frame, label in ((assistant, "Assistant"), (context, "Your context"), (audio, "Audio")):
            self.notebook.add(frame, text=label)
        self._assistant_tab(assistant)
        self._context_tab(context)
        self._audio_tab(audio)
        ttk.Label(outer, textvariable=self.status, style="Muted.TLabel", wraplength=850).pack(
            fill="x", pady=(12, 0))

    def _assistant_tab(self, parent):
        ttk.Label(parent, text="Question or follow-up").pack(anchor="w")
        self.question = self._text(parent, 4)
        self.question.pack(fill="x", pady=(6, 10))
        row = ttk.Frame(parent)
        row.pack(fill="x", pady=(0, 10))
        self.combobox(row, textvariable=self.mode, values=list(MODES), state="readonly", width=20).pack(side="left")
        self.generate_button = ttk.Button(row, text="Suggest answer", style="Accent.TButton", command=self.generate)
        self.generate_button.pack(side="left", padx=8)
        self.stop_button = ttk.Button(row, text="Stop", command=self.stop_generation, state="disabled")
        self.stop_button.pack(side="left")
        ttk.Button(row, text="Clear session", command=self.clear_session).pack(side="right")
        ttk.Label(parent, textvariable=self.answer_heading).pack(anchor="w")
        self.answer = self._text(parent, 10)
        self.answer.pack(fill="both", expand=True, pady=(6, 8))
        self.answer.configure(state="disabled")
        footer = ttk.Frame(parent)
        footer.pack(fill="x")
        ttk.Button(footer, text="Copy answer", command=self.copy_answer).pack(side="left")
        ttk.Button(footer, text="Export session", command=self.export_session).pack(side="left", padx=8)
        ttk.Label(footer, text="Ctrl+Enter / ⌘+Enter to ask", style="Muted.TLabel").pack(side="right")

    def _context_tab(self, parent):
        row = ttk.Frame(parent)
        row.pack(fill="x", pady=(0, 8))
        ttk.Label(row, text="Practice focus").pack(side="left", padx=(0, 8))
        self.combobox(row, textvariable=self.preset, values=list(PRESETS), state="readonly", width=31).pack(side="left")
        self.context_boxes = {}
        context_tabs = ttk.Notebook(parent)
        context_tabs.pack(fill="both", expand=True, pady=(5, 8))
        for key, label in (("resume", "Resume / experience"), ("job", "Job description"), ("notes", "Personal notes")):
            frame = ttk.Frame(context_tabs, padding=8)
            context_tabs.add(frame, text=label)
            title = ttk.Frame(frame)
            title.pack(fill="x", pady=(5, 4))
            ttk.Label(title, text=label).pack(side="left")
            ttk.Button(title, text="Import file", command=lambda k=key: self.import_document(k)).pack(side="right")
            box = self._text(frame, 9)
            box.insert("1.0", self.initial_profile[key])
            box.pack(fill="both", expand=True)
            self.context_boxes[key] = box
        ttk.Label(parent, text="No resume is preloaded. Claude/Codex receive your reference text when you ask; Ollama keeps inference local.",
                  style="Muted.TLabel", wraplength=800).pack(anchor="w", pady=(8, 0))
        ttk.Label(parent, text="Save context writes an unencrypted local profile. Session answers are kept in memory.",
                  style="Muted.TLabel", wraplength=800).pack(anchor="w", pady=(4, 8))
        ttk.Button(parent, text="Save context on this computer", command=self.save_context).pack(anchor="w")

    def _audio_tab(self, parent):
        ttk.Label(parent, text="Continuous local speech transcription", font=("TkDefaultFont", 14, "bold")).pack(anchor="w")
        ttk.Label(parent, text="Rolling transcripts can trigger an early draft while speech continues. Completed speech then triggers a final answer.",
                  style="Muted.TLabel", wraplength=800).pack(anchor="w", pady=(6, 8))
        ttk.Label(parent, text="Windows/Linux: select System output or a specific loopback. macOS: route meeting audio through BlackHole and select that input. "
                  "Keep the selected playback device unmuted. Audio is transcribed locally.",
                  style="Muted.TLabel", wraplength=800).pack(anchor="w", pady=(0, 12))
        row = ttk.Frame(parent)
        row.pack(fill="x", pady=(0, 10))
        self.combobox(row, textvariable=self.speech_model, values=SPEECH_MODELS, state="readonly", width=10).pack(side="left", padx=(0, 8))
        self.load_audio_button = ttk.Button(row, text="Load speech model (one download)", command=self.load_speech)
        self.load_audio_button.pack(side="left")
        ttk.Button(row, text="Refresh inputs", command=self.refresh_inputs).pack(side="left", padx=8)
        self.device_map = {"System output (default)": ("system", None), "Default microphone": ("input", None)}
        self.device_combo = self.combobox(parent, textvariable=self.device, values=list(self.device_map), state="readonly")
        self.device_combo.pack(fill="x", pady=(0, 10))
        threshold_row = ttk.Frame(parent)
        threshold_row.pack(fill="x", pady=(0, 10))
        ttk.Label(threshold_row, text="Speech threshold").pack(side="left", padx=(0, 12))
        ttk.Scale(threshold_row, from_=0.003, to=0.06, variable=self.threshold).pack(side="left", fill="x", expand=True)
        ttk.Label(parent, text="Lower the threshold for quiet speech; raise it if background noise triggers transcription.",
                  style="Muted.TLabel", wraplength=800).pack(anchor="w", pady=(0, 8))
        pause_row = ttk.Frame(parent)
        pause_row.pack(fill="x", pady=(0, 8))
        ttk.Label(pause_row, text="Question pause (seconds)").pack(side="left", padx=(0, 8))
        ttk.Spinbox(pause_row, from_=0.6, to=3.0, increment=0.2,
                    textvariable=self.question_pause, width=5).pack(side="left")
        ttk.Label(pause_row, text="Continuations can extend the same question.",
                  style="Muted.TLabel").pack(side="left", padx=12)
        row = ttk.Frame(parent)
        row.pack(fill="x", pady=(0, 10))
        self.listen_button = ttk.Button(row, text="Start listening", command=self.toggle_listening, state="disabled")
        self.listen_button.pack(side="left")
        ttk.Button(row, text="Use latest speech as question", command=self.use_transcript).pack(side="left", padx=8)
        ttk.Checkbutton(parent, text="Automatically answer speech", variable=self.auto,
                        command=self.toggle_auto).pack(anchor="w")
        ttk.Checkbutton(parent, text="Draft answers while the question is still being asked", variable=self.live_drafts,
                        command=self.toggle_live_drafts).pack(anchor="w")
        ttk.Label(parent, text="Auto mode also reacts to your voice if this input includes it. Prefer a dedicated question input.",
                  style="Muted.TLabel", wraplength=800).pack(anchor="w", pady=(4, 8))
        ttk.Label(parent, textvariable=self.live_question, style="Muted.TLabel", wraplength=800).pack(anchor="w", pady=(0, 6))
        self.transcript = self._text(parent, 9)
        self.transcript.pack(fill="both", expand=True)
        self.transcript.configure(state="disabled")
        ttk.Label(parent, textvariable=self.audio_status, style="Muted.TLabel", wraplength=800).pack(anchor="w", pady=(8, 0))

    def get_profile(self):
        return {"model": self.model.get(), "provider": self.provider.get(), "preset": self.preset.get(),
                **{k: box.get("1.0", "end-1c") for k, box in self.context_boxes.items()}}

    def change_provider(self, event=None, reset=True):
        provider = self.provider.get()
        if sys.platform == "win32":
            if provider == PROVIDERS[0]:
                self.claude_signin_button.pack(side="left")
            else:
                self.claude_signin_button.pack_forget()
        self.model_combo.configure(values=MODEL_CHOICES[provider], state="readonly" if provider == PROVIDERS[2] else "normal")
        if reset:
            self.model.set(MODEL_CHOICES[provider][0])
        note = "Local Ollama: questions and context stay on this computer." if provider == PROVIDERS[2] else "Cloud model via your official CLI login: question and reference text are sent to the selected provider. Subscription limits apply."
        self.provider_note.configure(text=note)

    def toggle_capture(self):
        for path in tuple(self.capture_windows):
            self.recheck_capture(path)

    def combobox(self, parent, **options):
        combo = ttk.Combobox(parent, **options)
        if sys.platform == "win32":
            def protect_popdown():
                path = self.root.tk.call("ttk::combobox::PopdownWindow", str(combo))
                self.track_capture(path)
            combo.configure(postcommand=protect_popdown)
        return combo

    def protect_window(self, path):
        self.track_capture(path)
        self.recheck_capture(path, prepare=True)

    def track_capture(self, path):
        if path not in self.capture_windows:
            self.capture_windows[path] = ""
            for event in ("<Map>", "<Configure>"):
                self.root.tk.call("bind", path, event, f"+{self.capture_recheck} %W")
            self.root.tk.call("bind", path, "<Destroy>", f"+{self.capture_destroy} %W")

    def before_wm(self, command, operation):
        args = self.root.tk.splitlist(command)
        if len(args) == 3 and args[1] == "deiconify":
            self.recheck_capture(args[2], prepare=True)

    def forget_capture(self, path):
        if path in self.capture_windows:
            del self.capture_windows[path]
            if self.capture_windows:
                self.show_capture_status()

    def recheck_capture(self, path, *, prepare=False):
        if path not in self.capture_windows:
            return
        try:
            set_capture_exclusion(self.root, self.capture_exclusion.get(), path=path, prepare=prepare)
            self.capture_windows[path] = ""
        except Exception as exc:
            self.capture_windows[path] = str(exc)
        self.show_capture_status()

    def show_capture_status(self):
        error = next((error for error in self.capture_windows.values() if error), None)
        if error:
            self.privacy_status.set("Capture protection unavailable or failed. " + error)
        elif self.capture_exclusion.get():
            self.privacy_status.set("Windows confirmed capture exclusion. Check the receiving view for your share mode.")
        else:
            self.privacy_status.set("Capture exclusion is OFF. Assistant windows may appear in captures.")

    def protected_path(self, title, initial=""):
        dialog = tk.Toplevel(self.root)
        dialog.withdraw()
        dialog.transient(self.root)
        dialog.attributes("-toolwindow", True)
        dialog.title(title)
        value = tk.StringVar(dialog, value=initial)
        result = None

        def accept(event=None):
            nonlocal result
            path = value.get().strip().strip('"')
            if path:
                result = Path(path).expanduser()
                dialog.destroy()

        ttk.Label(dialog, text="Paste a file path or type it below.").pack(padx=12, pady=8)
        entry = ttk.Entry(dialog, textvariable=value, width=62)
        entry.pack(padx=12)
        ttk.Button(dialog, text="Continue", command=accept).pack(pady=8)
        ttk.Label(dialog, textvariable=self.privacy_status, wraplength=450).pack(padx=12, pady=(0, 8))
        dialog.bind("<Return>", accept)
        dialog.bind("<Escape>", lambda event: dialog.destroy())
        self.protect_window(str(dialog))
        dialog.deiconify()
        dialog.wait_visibility()
        dialog.grab_set()
        entry.focus_set()
        dialog.wait_window()
        return result

    def toggle_auto(self):
        if not self.auto.get():
            self.audio_answers.clear()

    def toggle_live_drafts(self):
        if not self.live_drafts.get():
            self.audio_answers.drop_partials()

    def check_model(self):
        self.status.set("Checking provider connection…")
        provider, model = self.provider.get(), self.model.get()
        def work():
            try:
                self.events.put(("status", self.client.check(provider, model)))
            except Exception as exc:
                self.events.put(("status", str(exc)))
        threading.Thread(target=work, daemon=True).start()

    def sign_in_to_claude(self):
        if self.claude_signing_in:
            return
        self.claude_signing_in = True
        self.claude_signin_button.configure(state="disabled")
        self.status.set("Complete Claude sign-in in your browser and the sign-in window.")
        def work():
            try:
                self.client.sign_in_to_claude()
                self.events.put(("claude_signin", None))
            except Exception as exc:
                self.events.put(("claude_signin", str(exc)))
        threading.Thread(target=work, daemon=True).start()

    def save_context(self):
        profile = self.get_profile()
        try:
            # Use the same validation as generation rather than silently truncating edits.
            build_messages(profile, self.mode.get(), "Validate context")
            save_profile(profile)
            self.status.set("Context saved locally. New answers will use these fields.")
        except Exception as exc:
            self.status.set("Could not save context. " + str(exc))

    def import_document(self, key):
        if sys.platform == "win32" and self.capture_exclusion.get():
            filename = self.protected_path("Import document")
        else:
            filename = filedialog.askopenfilename(parent=self.root, filetypes=[("Documents", "*.txt *.md *.pdf *.docx"), ("All files", "*")])
        if not filename:
            return
        self.status.set("Reading document locally…")
        def work():
            try:
                text = read_document(Path(filename))
                limit = 3000 if key == "notes" else CONTEXT_LIMIT
                self.events.put(("import", (key, text, len(text) > limit)))
            except Exception as exc:
                self.events.put(("error", "Could not read document: " + str(exc)))
        threading.Thread(target=work, daemon=True).start()

    def generate(self, speech_end=None, *, draft=False, audio_request=None):
        if self.busy:
            return
        question = self.question.get("1.0", "end-1c").strip()
        profile = self.get_profile()
        try:
            signature = json.dumps({k: v for k, v in profile.items() if k != "model"}, sort_keys=True)
            history = self.history if signature == self.context_signature else []
            previous = self.last_final_question if signature == self.context_signature else None
            speech_context = None
            if audio_request is not None and not draft:
                speech_context = {"segments": list(audio_request.segments),
                                  "previous_question": previous["question"] if previous else ""}
            messages = build_messages(profile, self.mode.get(), question, history, draft=draft,
                                      speech_context=speech_context)
        except Exception as exc:
            self.status.set(str(exc))
            return
        if signature != self.context_signature:
            self.history.clear()
            self.last_final_question = None
            self.context_signature = signature
        self.busy = True
        self.current_draft = draft
        self.current_audio_request = audio_request
        self.current_semantic = None
        self.current_obsolete = self.superseded = False
        self.previous_final_snapshot = previous
        self.current_question = question
        self.current_answer = ""
        self.first_token_seconds = None
        self.latency_note = ""
        self.current_speech_end = speech_end if speech_end is not None else (self.last_speech_end if question == self.last_transcript else None)
        self.started = time.monotonic()
        self.cancel = threading.Event()
        cancel = self.cancel
        self.generate_button.configure(state="disabled")
        self.stop_button.configure(state="normal")
        self._replace(self.answer, "")
        self.answer_heading.set("PROVISIONAL DRAFT · question may still change" if draft else
                                "Checking the spoken question" if audio_request is not None else "Answer to the submitted question")
        self.status.set(("Drafting early with " if draft else "Generating with ") + profile["provider"] + "…")
        def work():
            try:
                on_token = lambda token: self.events.put(("token", (cancel, token)))
                decoder = None
                if speech_context is not None:
                    decoder = SemanticResponseDecoder(on_token,
                                lambda metadata: self.events.put(("semantic_meta", (cancel, metadata))))
                answer = self.client.stream(profile["provider"], profile["model"], messages, cancel,
                                            decoder.feed if decoder else on_token)
                if decoder is not None and not cancel.is_set():
                    answer = decoder.finish()
                self.events.put(("done", (cancel, (answer, None))))
            except Exception as exc:
                self.events.put(("done", (cancel, ("", str(exc)))))
        threading.Thread(target=work, daemon=True).start()

    def stop_generation(self):
        self.cancel.set()
        self.auto.set(False)
        self.audio_answers.clear()
        self.pending_speech = ""
        if self.auto_timer is not None:
            self.root.after_cancel(self.auto_timer)
            self.auto_timer = None
        if self.busy:
            self.status.set("Stopping at the next response chunk. A stalled model may take up to the request timeout.")

    def clear_session(self):
        if self.busy or self.audio_active or self.audio_stopping:
            self.status.set("Stop generation and listening before clearing the session.")
            return
        if self.auto_timer is not None:
            self.root.after_cancel(self.auto_timer)
            self.auto_timer = None
        self.history.clear()
        self.audio_answers.clear()
        self.questions.clear()
        self.last_final_question = None
        self.last_speech_end = None
        self.last_answer = self.current_answer = self.last_transcript = self.pending_speech = ""
        self.question.delete("1.0", "end")
        self._replace(self.answer, "")
        self._replace(self.transcript, "")
        self.live_question.set("Live transcript appears here while the speaker is talking.")
        self.answer_heading.set("Suggested answer")
        self.status.set("Session cleared. Your editable context is available for the next question.")

    def copy_answer(self):
        text = self.answer.get("1.0", "end-1c")
        if text:
            self.root.clipboard_clear()
            self.root.clipboard_append(text)
            self.status.set("Answer copied to the system clipboard.")

    def export_session(self):
        if self.busy:
            self.status.set("Wait for the current answer to finish before exporting.")
            return
        if not self.history:
            self.status.set("No completed answers to export yet.")
            return
        protected = sys.platform == "win32" and self.capture_exclusion.get()
        default = "personal_interview_assistant_session.md"
        if protected:
            filename = self.protected_path("Export session", str(Path.cwd() / default))
        else:
            filename = filedialog.asksaveasfilename(parent=self.root, defaultextension=".md", initialfile=default,
                                                   filetypes=[("Markdown", "*.md")])
        if filename:
            try:
                content = f"# {APP_NAME} session\n\n"
                for item in self.history:
                    title = "Question" if item["role"] == "user" else "Suggested answer"
                    content += f"## {title}\n\n{item['content']}\n\n"
                with Path(filename).open("x" if protected else "w", encoding="utf-8") as output:
                    output.write(content)
                self.status.set("Session exported to your chosen file.")
            except OSError as exc:
                self.status.set("Export failed. " + str(exc))

    def load_speech(self):
        if self.audio_loading or self.audio_active or self.audio_stopping:
            return
        self.audio_loading = True
        self.load_audio_button.configure(state="disabled")
        name = self.speech_model.get()
        self.audio_status.set("Loading " + name + " locally; first use downloads model weights…")
        def work():
            try:
                self.listener.load_model(name)
                self.events.put(("speech_ready", None))
            except Exception as exc:
                self.events.put(("speech_failed", str(exc)))
        threading.Thread(target=work, daemon=True).start()

    def refresh_inputs(self):
        if self.audio_active or self.audio_stopping:
            self.audio_status.set("Stop listening before changing audio devices.")
            return
        selected = self.device.get()
        self.device_map = {"System output (default)": ("system", None), "Default microphone": ("input", None)}
        warnings = []
        for enumerate_devices, backend, prefix in ((input_devices, "input", "Input"),
                                                    (loopback_devices, "system", "System output")):
            try:
                self.device_map.update({f"{prefix}: {name} [{index}]": (backend, index)
                                        for index, name in enumerate_devices()})
            except Exception as exc:
                warnings.append(str(exc))
        self.device_combo.configure(values=list(self.device_map))
        if selected not in self.device_map:
            self.device.set("")
            warnings.append("The selected device is unavailable. Choose an input or output.")
        self.audio_status.set(" ".join(warnings) if warnings else "Select the output carrying the meeting audio, then start listening.")

    def toggle_listening(self):
        if self.audio_active:
            self.audio_active = False
            self.audio_stopping = True
            self.pending_speech = ""
            self.audio_answers.clear()
            self.questions.clear()
            if self.busy and self.current_audio_request is not None:
                self.cancel.set()
            self.listen_button.configure(state="disabled", text="Finishing…")
            self.audio_status.set("Finishing the last transcription…")
            threading.Thread(target=self.listener.stop, daemon=True).start()
        else:
            try:
                self.audio_answers.clear()
                self.questions.clear()
                route = self.device_map.get(self.device.get())
                if route is None:
                    raise CopilotError("Select a system output or microphone/virtual input before listening.")
                self.listener.start(route, float(self.threshold.get()))
                self.audio_active = True
                self.listen_button.configure(text="Stop listening")
                self.load_audio_button.configure(state="disabled")
                self.device_combo.configure(state="disabled")
                self.audio_status.set("Listening. Rolling transcripts can trigger drafts before the speaker pauses.")
            except Exception as exc:
                self.audio_status.set(str(exc))

    def use_transcript(self):
        text = self.last_transcript
        try:
            text = self.transcript.get("sel.first", "sel.last")
        except tk.TclError:
            pass
        if text:
            self.question.delete("1.0", "end")
            self.question.insert("1.0", text)
            self.notebook.select(0)
        else:
            self.audio_status.set("No speech transcribed yet.")

    def auto_generate(self):
        self.auto_timer = None
        if not self.auto.get() or not self.audio_active:
            self.pending_speech = ""
            self.audio_answers.clear()
            return
        if self.busy:
            return
        request = self.audio_answers.next_request()
        if request is not None:
            state = self.questions.groups.get(request.utterance_id)
            if state is None or state.revision != request.revision:
                return
            if not request.draft and not self.questions.can_submit(request.utterance_id, request.revision, time.monotonic()):
                self.audio_answers.defer(request)
                if self.audio_answers.pending:
                    self.auto_timer = self.root.after(150, self.auto_generate)
                return
            self.question.delete("1.0", "end")
            self.question.insert("1.0", request.text)
            self.generate(speech_end=request.speech_at, draft=request.draft, audio_request=request)

    def update_audio_question(self, data, *, final):
        request = self.questions.ingest(data["utterance_id"], data["text"], data["speech_end"], final=final)
        if request is None:
            return
        if (self.current_audio_request is not None and request.utterance_id == self.current_audio_request.utterance_id
                and request.revision > self.current_audio_request.revision):
            self.current_obsolete = True
            self.answer_heading.set("EARLIER WORDING · listening before revising this answer")
        if self.auto.get() and self.live_drafts.get() and self.audio_active and not final:
            self.audio_answers.offer_partial(request.utterance_id, request.text, request.speech_at,
                                             request.revision, request.segments)
        return request

    def discard_audio_utterance(self, utterance_id):
        question_id = self.questions.complete_empty(utterance_id)
        if question_id is not None and self.current_audio_request is not None and self.current_audio_request.utterance_id == question_id:
            self.current_obsolete = True
            self._replace(self.answer, "")
            self.answer_heading.set("Earlier draft cleared · waiting for verified speech")
            if self.busy:
                self.superseded = True
                self.cancel.set()

    def poll_audio_answers(self):
        if not self.auto.get() or not self.audio_active:
            return
        try:
            self.questions.pause_seconds = min(3.0, max(0.6, float(self.question_pause.get())))
        except (ValueError, tk.TclError):
            self.questions.pause_seconds = 1.2
        for request in self.questions.poll_ready(time.monotonic()):
            if not self.audio_answers.offer_final(request.utterance_id, request.text, request.speech_at,
                                                  request.revision, request.segments):
                self.audio_status.set("The model is behind; an older queued question was skipped.")
            if (self.busy and self.current_audio_request is not None
                    and request.utterance_id == self.current_audio_request.utterance_id
                    and (self.current_draft or request.revision > self.current_audio_request.revision)):
                self.superseded = True
                self.cancel.set()
                self.answer_heading.set("UPDATING · complete question is replacing the earlier prediction")
        if self.audio_answers.pending and not self.busy and self.auto_timer is None:
            self.auto_timer = self.root.after(100, self.auto_generate)

    @staticmethod
    def _replace(widget, text):
        widget.configure(state="normal")
        widget.delete("1.0", "end")
        widget.insert("1.0", text)
        widget.configure(state="disabled")

    @staticmethod
    def _append(widget, text):
        widget.configure(state="normal")
        widget.insert("end", text)
        widget.see("end")
        widget.configure(state="disabled")

    def pump(self):
        # One bounded batch keeps rapid token streams from starving the UI.
        for _ in range(200):
            try:
                kind, data = self.events.get_nowait()
            except queue.Empty:
                break
            if kind in ("token", "semantic_meta", "done"):
                generation, data = data
                if not self.busy or generation is not self.cancel:
                    continue
                if kind != "done" and (self.cancel.is_set() or self.current_obsolete):
                    continue
            if kind == "token":
                if self.first_token_seconds is None:
                    now = time.monotonic()
                    self.first_token_seconds = now - self.started
                    self.latency_note = f"First text: {self.first_token_seconds:.1f}s from request"
                    if self.current_speech_end is not None:
                        self.latency_note += f" · {now - self.current_speech_end:.1f}s from last speech"
                    self.status.set(self.latency_note)
                self.current_answer += data
                self._append(self.answer, data)
            elif kind == "semantic_meta":
                self.current_semantic = data
                if not self.current_obsolete:
                    if data["relation"] == "incomplete":
                        self.answer_heading.set("QUESTION INCOMPLETE · waiting for more speech")
                    else:
                        self.answer_heading.set("Question checked · continuation" if data["relation"] == "continue"
                                                else "Question checked · new question")
            elif kind == "done":
                answer, error = data
                cancelled = self.cancel.is_set()
                request = self.current_audio_request
                if request is not None and not request.draft and not (cancelled or error or self.current_obsolete):
                    if not self.questions.can_submit(request.utterance_id, request.revision, time.monotonic()):
                        self.audio_answers.defer(request)
                        self.current_obsolete = True
                        self._replace(self.answer, "")
                        self.answer_heading.set("Speech resumed · waiting to check the complete question")
                self.busy = False
                self.generate_button.configure(state="normal")
                self.stop_button.configure(state="disabled")
                if cancelled:
                    self._replace(self.answer, "")
                    self.status.set("Updating the earlier prediction with the complete question…" if self.superseded
                                    else "Stopped. The partial response was not added to conversation history.")
                elif error:
                    self._replace(self.answer, "")
                    self.answer_heading.set("Answer unavailable")
                    self.status.set(error)
                elif answer:
                    incomplete = self.current_semantic and self.current_semantic["relation"] == "incomplete"
                    if request is not None and self.current_semantic is not None and not self.current_obsolete:
                        self.questions.mark_semantic_status(request.utterance_id, incomplete)
                    committed = commit_checked_answer(self.history, self.current_question, answer,
                                metadata=self.current_semantic, previous=self.previous_final_snapshot,
                                draft=self.current_draft, obsolete=self.current_obsolete)
                    if committed:
                        self.last_final_question = committed
                        if self.current_audio_request is not None:
                            self.questions.compact_committed(self.current_audio_request.utterance_id,
                                                              self.current_audio_request.revision,
                                                              committed["question"])
                    self.last_answer = answer
                    if self.current_draft and self.current_audio_request is not None:
                        self.audio_answers.note_draft_result(self.current_audio_request, answer)
                        state = self.questions.groups.get(self.current_audio_request.utterance_id)
                        if state and self.live_drafts.get() and self.auto.get() and self.audio_active:
                            self.audio_answers.offer_partial(state.question_id, state.text, state.speech_at,
                                                             state.revision, tuple(state.fragments.values()))
                    stage = ("Listening for the rest of the question" if incomplete else
                             "Earlier wording; revision pending" if self.current_obsolete else
                             "Draft complete; waiting for the complete question" if self.current_draft else "Answer complete")
                    self.status.set(f"{self.latency_note} · {stage} · {time.monotonic() - self.started:.1f}s total")
                else:
                    self.status.set("The model returned no answer. Try another question.")
                if self.audio_answers.pending and self.auto.get() and self.audio_active and not error and (not cancelled or self.superseded):
                    self.auto_timer = self.root.after(100, self.auto_generate)
            elif kind == "status":
                self.status.set(data)
            elif kind == "claude_signin":
                self.claude_signing_in = False
                self.claude_signin_button.configure(state="normal")
                if self.provider.get() == PROVIDERS[0]:
                    if data:
                        self.status.set(data)
                    else:
                        self.check_model()
            elif kind == "error":
                self.status.set(data)
            elif kind == "import":
                key, text, too_long = data
                self.context_boxes[key].delete("1.0", "end")
                self.context_boxes[key].insert("1.0", text)
                self.status.set("Imported locally. Shorten this field before asking or saving." if too_long else "Imported locally. Review the text, then save context if wanted.")
            elif kind == "speech_ready":
                self.audio_loading = False
                self.load_audio_button.configure(state="normal")
                self.listen_button.configure(state="normal")
                self.audio_status.set("Local speech model ready. Select an input, then start listening.")
                self.refresh_inputs()
            elif kind == "speech_failed":
                self.audio_loading = False
                self.load_audio_button.configure(state="normal")
                self.audio_status.set(data)
            elif kind == "speech_activity":
                self.questions.observe_activity(data["utterance_id"], data["at"])
            elif kind == "speech_empty":
                self.discard_audio_utterance(data["utterance_id"])
            elif kind == "speech_dropped":
                self.discard_audio_utterance(data)
                self.live_question.set("An earlier utterance was skipped. Listening for the next segment.")
                self.audio_status.set("Transcription is behind; an utterance was skipped. Try tiny.en.")
            elif kind == "partial_transcript":
                if self.update_audio_question(data, final=False) is not None:
                    self.live_question.set("Live: " + data["text"][-240:])
            elif kind == "transcript":
                if self.update_audio_question(data, final=True) is None:
                    continue
                self.last_transcript = text = data["text"]
                self.last_speech_end = data["speech_end"]
                stamp = time.strftime("%H:%M:%S")
                self.live_question.set("Completed segment: " + text[-240:])
                self._append(self.transcript, f"[{stamp} · STT {data['transcription_seconds']:.1f}s] {text}\n\n")
            elif kind in ("audio_status", "audio_error"):
                self.audio_status.set(data)
                if kind == "audio_error":
                    self.audio_active = False
                    self.audio_answers.clear()
                    if self.busy and self.current_audio_request is not None:
                        self.cancel.set()
                    threading.Thread(target=self.listener.stop, daemon=True).start()
            elif kind == "audio_finished":
                self.audio_active = self.audio_stopping = False
                self.listen_button.configure(text="Start listening", state="normal")
                self.load_audio_button.configure(state="normal")
                self.device_combo.configure(state="readonly")
                if not self.listener.worker_error:
                    self.audio_status.set("Listening stopped. Transcription stays in this session until cleared.")
        self.poll_audio_answers()
        self.root.after(60, self.pump)

    def close(self):
        self.cancel.set()
        self.listener.stop_event.set()
        threading.Thread(target=self.listener.stop, daemon=True).start()
        self.root.destroy()


def main():
    try:
        root = tk.Tk()
    except tk.TclError as exc:
        raise SystemExit("A desktop display with Tk support is required. See README.md for platform setup.") from exc
    if sys.platform == "win32":
        # Apply capture affinity before the main assistant window is shown.
        root.withdraw()
        root.attributes("-toolwindow", True)
        root.tk.call("rename", "bell", "")
        root.tk.call("proc", "bell", "args", "")
    app = App(root)
    if sys.platform == "win32":
        root.deiconify()
    if "--prepare" in sys.argv:
        root.after_idle(app.load_speech)
        root.after_idle(app.check_model)
    root.mainloop()


if __name__ == "__main__":
    main()
