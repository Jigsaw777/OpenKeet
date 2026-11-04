"""Windows capture exclusion and readback for an app-owned top-level window."""
from __future__ import annotations

import sys

from core import CopilotError


def set_capture_exclusion(root, enabled: bool, *, path=None, prepare=True):
    if sys.platform != "win32":
        raise CopilotError("This capture-exclusion switch supports Windows only. Share a specific app window "
                           "and keep the assistant outside that window on this platform.")
    if sys.getwindowsversion().build < 19041:
        raise CopilotError("Capture exclusion needs Windows 10 version 2004 or newer.")
    import ctypes
    from ctypes import wintypes
    user32 = ctypes.WinDLL("user32", use_last_error=True)
    user32.GetAncestor.argtypes = [wintypes.HWND, wintypes.UINT]
    user32.GetAncestor.restype = wintypes.HWND
    user32.SetWindowDisplayAffinity.argtypes = [wintypes.HWND, wintypes.DWORD]
    user32.SetWindowDisplayAffinity.restype = wintypes.BOOL
    user32.GetWindowDisplayAffinity.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.DWORD)]
    user32.GetWindowDisplayAffinity.restype = wintypes.BOOL
    if prepare:
        root.update_idletasks()
    widget_id = root.winfo_id() if path is None else int(root.tk.call("winfo", "id", path), 16)
    handle = user32.GetAncestor(widget_id, 2)  # GA_ROOT: Tk's top-level native wrapper.
    affinity = 0x11 if enabled else 0
    if not handle or not user32.SetWindowDisplayAffinity(handle, affinity):
        raise CopilotError(f"Windows rejected capture exclusion (error {ctypes.get_last_error()}).")
    actual = wintypes.DWORD()
    if not user32.GetWindowDisplayAffinity(handle, ctypes.byref(actual)) or actual.value != affinity:
        raise CopilotError("Windows did not confirm the requested capture setting. Verify from another device.")
    return "Windows capture exclusion is enabled. Verify in the receiving screen-share view; support varies." if enabled else "Windows capture exclusion is disabled."
