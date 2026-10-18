@echo off
chcp 65001 >nul
cd /d "%~dp0"
setlocal

set "APP=dist\ApiCheckinHelper-win-x64\ApiCheckinHelper.exe"

if exist "%APP%" (
  "%APP%" %*
  if errorlevel 1 pause
  endlocal
  exit /b 0
)

echo.
echo 找不到已构建的程序，请先运行 build.cmd 生成 dist 包。
echo.
pause
endlocal
exit /b 1
