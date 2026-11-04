@echo off
setlocal
cd /d "%~dp0"

echo Personal Interview Assistant - Windows setup
echo This creates a local Python environment and downloads the app dependencies.
echo It does not install Claude Code or sign you in automatically.
echo.

if exist ".venv\Scripts\python.exe" goto validate_python

py -3.12 -c "import sys, tkinter; assert sys.maxsize > 2**32" >nul 2>&1
if not errorlevel 1 (
  py -3.12 -m venv .venv
  goto validate_python
)

py -3.11 -c "import sys, tkinter; assert sys.maxsize > 2**32" >nul 2>&1
if not errorlevel 1 (
  py -3.11 -m venv .venv
  goto validate_python
)

echo Python 3.11 or 3.12, 64-bit, with Tk and the Python launcher is needed.
echo Install the supported Python version, then run this setup again.
echo See WINDOWS_QUICKSTART.md for the setup steps.
goto failed

:validate_python
if not exist ".venv\Scripts\python.exe" (
  echo The Python environment could not be created.
  goto failed
)

".venv\Scripts\python.exe" -c "import sys, tkinter; assert sys.version_info[:2] in [(3,11),(3,12)]; assert sys.maxsize > 2**32" >nul 2>&1
if errorlevel 1 (
  echo The existing .venv needs Python 3.11 or 3.12, 64-bit, with Tk support.
  echo Rename that environment folder if you want to keep it, then retry setup.
  goto failed
)

".venv\Scripts\python.exe" -m pip install --upgrade pip
if errorlevel 1 goto failed

".venv\Scripts\python.exe" -m pip install -r requirements.txt
if errorlevel 1 goto failed

echo.
echo App dependencies installed.
where claude >nul 2>&1
if errorlevel 1 (
  echo Claude Code was not found on PATH in this window.
  echo See WINDOWS_QUICKSTART.md to install it and sign in.
)
echo Next: sign in to Claude Code, then double-click start_windows.vbs.
echo Choose System output in the Audio tab to transcribe the other person's audio.
pause
exit /b 0

:failed
echo.
echo Setup did not finish. Keep the error text above when asking for help.
pause
exit /b 1
