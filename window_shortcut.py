"""Windows shortcut that brings the assistant forward without a taskbar button."""
import ctypes
from ctypes import wintypes
import threading


class ShowShortcut:
    LABEL = "Ctrl+Alt+I"

    def __init__(self, emit):
        self.emit = emit
        self.stopped = threading.Event()
        self.thread_id = None
        self.thread = threading.Thread(target=self.run, daemon=True)
        self.thread.start()

    def run(self):
        user32 = ctypes.WinDLL("user32", use_last_error=True)
        hotkey_id, wm_hotkey = 1, 0x0312
        modifiers = 0x0001 | 0x0002 | 0x4000  # Alt, Ctrl, no auto-repeat.
        if not user32.RegisterHotKey(None, hotkey_id, modifiers, ord("I")):
            self.emit("shortcut_status", f"{self.LABEL} unavailable")
            return

        self.thread_id = ctypes.windll.kernel32.GetCurrentThreadId()
        message = wintypes.MSG()
        try:
            while not self.stopped.is_set() and user32.GetMessageW(ctypes.byref(message), None, 0, 0) > 0:
                if message.message == wm_hotkey and message.wParam == hotkey_id:
                    self.emit("show_window", None)
        finally:
            user32.UnregisterHotKey(None, hotkey_id)
            self.thread_id = None

    def close(self):
        self.stopped.set()
        thread_id = self.thread_id
        if thread_id is not None:
            ctypes.windll.user32.PostThreadMessageW(thread_id, 0x0012, 0, 0)  # WM_QUIT
        self.thread.join(timeout=0.2)
