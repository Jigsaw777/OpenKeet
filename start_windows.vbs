Option Explicit
Dim shell, files, folder, launcher, command
Set shell = CreateObject("WScript.Shell")
Set files = CreateObject("Scripting.FileSystemObject")
folder = files.GetParentFolderName(WScript.ScriptFullName)
shell.CurrentDirectory = folder
launcher = files.BuildPath(folder, ".venv\Scripts\pythonw.exe")
If files.FileExists(launcher) Then
    command = Quote(launcher)
Else
    command = "pyw.exe -3.12"
End If
On Error Resume Next
shell.Run command & " " & Quote(files.BuildPath(folder, "app.py")), 1, False
If Err.Number <> 0 Then
    WScript.Echo "Personal Interview Assistant could not start. Run setup_windows.bat, then try again."
End If

Function Quote(value)
    Quote = Chr(34) & value & Chr(34)
End Function
