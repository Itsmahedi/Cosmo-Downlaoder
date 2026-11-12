@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0set-version.ps1" %*
