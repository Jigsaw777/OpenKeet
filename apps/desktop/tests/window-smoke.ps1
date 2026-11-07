param(
    [Parameter(Mandatory = $true)][int]$AppPid,
    [ValidateSet('Hidden', 'Visible', 'Shortcut')][string]$Check = 'Hidden'
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

    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();

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
            if (focus ? value.StartsWith("Personal Copilot") && value.EndsWith("Focus") : value == "Personal Copilot") {
                match = hwnd;
                return false;
            }
            return true;
        }, IntPtr.Zero);
        return match;
    }

    public static void RevealShortcut() {
        const ushort Ctrl = 0xA2, Shift = 0xA0, I = 0x49;
        try {
            Send(Key(Ctrl, false), Key(Shift, false), Key(I, false));
            Thread.Sleep(120);
        } finally {
            Send(Key(I, true), Key(Shift, true), Key(Ctrl, true));
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
Assert-ToolWindow $focus 'Focus after pin and unpin'

$clock = [Diagnostics.Stopwatch]::StartNew()
$expectedVisible = $Check -eq 'Visible'
while ([CopilotWindowProbe]::IsWindowVisible($main) -ne $expectedVisible -and $clock.ElapsedMilliseconds -lt 2000) {
    Start-Sleep -Milliseconds 30
}
if ([CopilotWindowProbe]::IsWindowVisible($main) -ne $expectedVisible) {
    throw "Workspace did not reach the expected $Check state."
}
if ($Check -ne 'Shortcut') {
    "Workspace $Check state and both tool-window styles verified."
    exit 0
}
[CopilotWindowProbe]::RevealShortcut()
$clock.Restart()
do {
    if ([CopilotWindowProbe]::IsWindowVisible($main) -and [CopilotWindowProbe]::GetForegroundWindow() -eq $main) {
        Assert-ToolWindow $main 'workspace after shortcut'
        'Ctrl+Shift+I restored and focused the hidden workspace.'
        exit 0
    }
    Start-Sleep -Milliseconds 40
} while ($clock.ElapsedMilliseconds -lt 5000)
throw ('Ctrl+Shift+I did not restore and focus the workspace. Visible={0}, foreground=0x{1:X}, main=0x{2:X}.' -f
    [CopilotWindowProbe]::IsWindowVisible($main), [CopilotWindowProbe]::GetForegroundWindow().ToInt64(), $main.ToInt64())
