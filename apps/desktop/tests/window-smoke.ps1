param(
    [Parameter(Mandatory = $true)][int]$AppPid,
    [ValidateSet('Hidden', 'Visible', 'Overlay', 'Close', 'Shortcut')][string]$Check = 'Hidden',
    [switch]$RestoreOverlay,
    [uint32]$CaptureAffinity
)

$ErrorActionPreference = 'Stop'
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;

public static class CopilotWindowProbe {
    private delegate bool EnumWindow(IntPtr hwnd, IntPtr data);

    [DllImport("user32.dll")]
    private static extern bool EnumWindows(EnumWindow visit, IntPtr data);

    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint process);

    [DllImport("user32.dll", CharSet=CharSet.Unicode)]
    private static extern int GetWindowText(IntPtr hwnd, StringBuilder title, int capacity);

    [DllImport("user32.dll", EntryPoint="GetWindowLongPtrW")]
    public static extern IntPtr GetWindowStyle(IntPtr hwnd, int index);

    [DllImport("user32.dll")]
    public static extern bool IsWindowVisible(IntPtr hwnd);

    [DllImport("user32.dll", SetLastError=true)]
    public static extern bool GetWindowDisplayAffinity(IntPtr hwnd, out uint affinity);

    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();

    [DllImport("user32.dll")]
    private static extern bool PostMessageW(IntPtr hwnd, uint message, IntPtr wParam, IntPtr lParam);

    public static bool CloseWindow(IntPtr hwnd) {
        return PostMessageW(hwnd, 0x10, IntPtr.Zero, IntPtr.Zero);
    }

    [DllImport("user32.dll", SetLastError=true)]
    private static extern uint SendInput(uint count, Input[] inputs, int size);
    [StructLayout(LayoutKind.Sequential)]
    private struct Input {
        public uint Kind;
        public InputData Data;
    }
    [StructLayout(LayoutKind.Explicit)]
    private struct InputData {
        [FieldOffset(0)] public KeyInput Key;
        [FieldOffset(0)] public MouseInput Mouse;
    }
    [StructLayout(LayoutKind.Sequential)]
    private struct KeyInput {
        public ushort Key, Scan;
        public uint Flags, Time;
        public UIntPtr Extra;
    }
    [StructLayout(LayoutKind.Sequential)]
    private struct MouseInput {
        public int X, Y;
        public uint Data, Flags, Time;
        public UIntPtr Extra;
    }

    public static IntPtr Find(int process, bool focus) {
        IntPtr match = IntPtr.Zero;
        EnumWindows((hwnd, data) => {
            uint owner;
            GetWindowThreadProcessId(hwnd, out owner);
            if (owner != process) return true;
            var title = new StringBuilder(256);
            GetWindowText(hwnd, title, title.Capacity);
            var value = title.ToString();
            if (focus ? value.StartsWith("OpenKeet") && value.EndsWith("Focus") : value == "OpenKeet") {
                match = hwnd;
                return false;
            }
            return true;
        }, IntPtr.Zero);
        return match;
    }

    public static void RevealShortcut() {
        const ushort Ctrl = 0xA2, Alt = 0xA4, I = 0x49;
        try {
            Send(Key(Ctrl, false), Key(Alt, false), Key(I, false));
            Thread.Sleep(120);
        } finally {
            Send(Key(I, true), Key(Alt, true), Key(Ctrl, true));
        }
    }

    private static Input Key(ushort code, bool released) {
        return new Input { Kind = 1, Data = new InputData { Key = new KeyInput { Key = code, Flags = released ? 2u : 0u } } };
    }

    private static void Send(params Input[] keys) {
        if (SendInput((uint)keys.Length, keys, Marshal.SizeOf(typeof(Input))) != keys.Length)
            throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
    }
}
'@

$main = [CopilotWindowProbe]::Find($AppPid, $false)
$focus = [CopilotWindowProbe]::Find($AppPid, $true)
function Assert-ToolWindow([IntPtr]$Handle, [string]$Name) {
    if ($Handle -eq [IntPtr]::Zero) { throw "Could not locate $Name for the smoke test." }
    $style = [CopilotWindowProbe]::GetWindowStyle($Handle, -20).ToInt64()
    if (($style -band 0x80) -eq 0 -or ($style -band 0x40000) -ne 0) {
        throw ('Expected WS_EX_TOOLWINDOW and no WS_EX_APPWINDOW for {0}. Actual style 0x{1:X}.' -f $Name, $style)
    }
}
Assert-ToolWindow $main 'workspace'
Assert-ToolWindow $focus 'overlay'
if (([CopilotWindowProbe]::GetWindowStyle($focus, -20).ToInt64() -band 0x8) -eq 0) {
    throw 'The overlay must stay above regular windows.'
}
if ($Check -eq 'Close' -and ![CopilotWindowProbe]::CloseWindow($main)) {
    throw 'Could not send Close to the workspace.'
}

$clock = [Diagnostics.Stopwatch]::StartNew()
$showMain = $Check -eq 'Visible'
$showOverlay = $Check -eq 'Overlay'
function Test-Visibility {
    return ([CopilotWindowProbe]::IsWindowVisible($main) -eq $showMain) -and ([CopilotWindowProbe]::IsWindowVisible($focus) -eq $showOverlay)
}
while (!(Test-Visibility) -and $clock.ElapsedMilliseconds -lt 3000) {
    Start-Sleep -Milliseconds 30
}
if (!(Test-Visibility)) {
    throw "Companion did not reach the expected $Check state."
}
if ($PSBoundParameters.ContainsKey('CaptureAffinity')) {
    # A hidden window can report no affinity while retaining its setting on reveal.
    $handle = if ($showOverlay) { $focus } elseif ($showMain) { $main } else { throw 'Show a window before testing capture affinity.' }
    $actual = [uint32]0
    if (![CopilotWindowProbe]::GetWindowDisplayAffinity($handle, [ref]$actual)) {
        throw [System.ComponentModel.Win32Exception]::new([Runtime.InteropServices.Marshal]::GetLastWin32Error())
    }
    if ($actual -ne $CaptureAffinity) {
        throw ('{0} window has capture affinity 0x{1:X}, expected 0x{2:X}.' -f $Check, $actual, $CaptureAffinity)
    }
    '{0} window capture affinity 0x{1:X} verified.' -f $Check, $CaptureAffinity
}
if ($Check -ne 'Shortcut') {
    "Companion $Check state, both tool-window styles and overlay topmost flag verified."
    exit 0
}
[CopilotWindowProbe]::RevealShortcut()
$expected = if ($RestoreOverlay) { $focus } else { $main }
$clock.Restart()
do {
    if ([CopilotWindowProbe]::IsWindowVisible($expected) -and [CopilotWindowProbe]::GetForegroundWindow() -eq $expected) {
        Assert-ToolWindow $expected 'companion after shortcut'
        'Ctrl+Alt+I restored and focused the hidden companion.'
        exit 0
    }
    Start-Sleep -Milliseconds 40
} while ($clock.ElapsedMilliseconds -lt 5000)
throw ('Ctrl+Alt+I did not restore and focus the companion. Visible={0}, foreground=0x{1:X}, expected=0x{2:X}.' -f
    [CopilotWindowProbe]::IsWindowVisible($expected), [CopilotWindowProbe]::GetForegroundWindow().ToInt64(), $expected.ToInt64())
