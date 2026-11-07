Option Explicit
Dim shell, files, folder, bootstrap, command
Set shell = CreateObject("WScript.Shell")
Set files = CreateObject("Scripting.FileSystemObject")
folder = files.GetParentFolderName(WScript.ScriptFullName)
shell.CurrentDirectory = folder
bootstrap = files.BuildPath(folder, "start_windows.ps1")
command = "powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File " & Quote(bootstrap)
On Error Resume Next
shell.Run command, 0, False
If Err.Number <> 0 Then
    WScript.Echo "OpenKeet (Legacy) could not start. Keep the project files together and try again."
End If

Function Quote(value)
    Quote = Chr(34) & value & Chr(34)
End Function
