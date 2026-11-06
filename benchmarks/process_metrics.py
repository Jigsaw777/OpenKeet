"""Measure a single Windows child process, including model initialization."""
import ctypes
from ctypes import wintypes
import statistics
import subprocess
import tempfile
import time


class MemoryCounters(ctypes.Structure):
    _fields_ = [
        ("cb", wintypes.DWORD), ("PageFaultCount", wintypes.DWORD),
        *((field, ctypes.c_size_t) for field in (
            "PeakWorkingSetSize", "WorkingSetSize", "QuotaPeakPagedPoolUsage", "QuotaPagedPoolUsage",
            "QuotaPeakNonPagedPoolUsage", "QuotaNonPagedPoolUsage", "PagefileUsage", "PeakPagefileUsage", "PrivateUsage",
        )),
    ]


def measure(command, timeout, *, env=None):
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    memory = ctypes.WinDLL("psapi", use_last_error=True).GetProcessMemoryInfo
    kernel.OpenProcess.argtypes = (wintypes.DWORD, wintypes.BOOL, wintypes.DWORD)
    kernel.OpenProcess.restype = wintypes.HANDLE
    kernel.CloseHandle.argtypes = (wintypes.HANDLE,)
    memory.argtypes = (wintypes.HANDLE, ctypes.POINTER(MemoryCounters), wintypes.DWORD)
    memory.restype = wintypes.BOOL
    kernel.GetProcessTimes.argtypes = (wintypes.HANDLE, *([ctypes.POINTER(wintypes.FILETIME)] * 4))
    kernel.GetProcessTimes.restype = wintypes.BOOL
    working_sets, peak_working_set, peak_commit = [], 0, 0
    mib = 1024 * 1024
    started = time.monotonic()

    with tempfile.TemporaryFile() as stdout, tempfile.TemporaryFile() as stderr:
        process = subprocess.Popen(command, stdout=stdout, stderr=stderr, env=env,
                                   creationflags=subprocess.CREATE_NO_WINDOW)
        handle = kernel.OpenProcess(0x0400 | 0x0010, False, process.pid)
        try:
            if not handle:
                raise ctypes.WinError(ctypes.get_last_error())
            while process.poll() is None:
                counters = MemoryCounters(cb=ctypes.sizeof(MemoryCounters))
                if memory(handle, ctypes.byref(counters), counters.cb):
                    working_sets.append(counters.WorkingSetSize)
                    peak_working_set = max(peak_working_set, counters.PeakWorkingSetSize)
                    peak_commit = max(peak_commit, counters.PeakPagefileUsage)
                if time.monotonic() - started >= timeout:
                    raise subprocess.TimeoutExpired(command, timeout)
                time.sleep(0.1)
            wall = time.monotonic() - started
            if not working_sets:
                raise RuntimeError("No Windows process memory samples were captured.")
            times = [wintypes.FILETIME() for _ in range(4)]
            if not kernel.GetProcessTimes(handle, *(ctypes.byref(value) for value in times)):
                raise ctypes.WinError(ctypes.get_last_error())
            cpu = sum((value.dwHighDateTime << 32) + value.dwLowDateTime for value in times[2:]) / 10_000_000
            stdout.seek(0)
            stderr.seek(0)
            completed = subprocess.CompletedProcess(command, process.returncode,
                                                    stdout.read().decode("utf-8", errors="replace"),
                                                    stderr.read().decode("utf-8", errors="replace"))
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            if handle:
                kernel.CloseHandle(handle)
    return completed, {"wall_s": round(wall, 3), "cpu_s": round(cpu, 3),
                       "peak_working_set_mib": round(peak_working_set / mib, 1),
                       "median_working_set_mib": round(statistics.median(working_sets) / mib, 1),
                       "peak_private_commit_mib": round(peak_commit / mib, 1),
                       "samples": len(working_sets)}
