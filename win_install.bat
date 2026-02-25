@echo off
set "PLUGIN_NAME=emojikanban"
set "BUILD_SOURCE_TRIPLE=.\target\x86_64-pc-windows-msvc\release\%PLUGIN_NAME%.dll"
set "BUILD_SOURCE=.\target\release\%PLUGIN_NAME%.dll"
set "BIN_SOURCE=.\bin\%PLUGIN_NAME%.dll"
set "PLUGIN_DIR=C:\ProgramData\obs-studio\plugins\%PLUGIN_NAME%\bin\64bit"

if exist "%BUILD_SOURCE_TRIPLE%" (
    set "SOURCE=%BUILD_SOURCE_TRIPLE%"
    echo [INFO] Using development build from .\target\x86_64-pc-windows-msvc\release
) else if exist "%BUILD_SOURCE%" (
    set "SOURCE=%BUILD_SOURCE%"
    echo [INFO] Using development build from .\target\release
) else if exist "%BIN_SOURCE%" (
    set "SOURCE=%BIN_SOURCE%"
    echo [INFO] Build not found. Using pre-compiled fallback from .\bin
    echo [INFO] Run 'cargo build -r --target x86_64-pc-windows-msvc' to build for yourself.
) else (
    echo [ERROR] Plugin file emojikanban.dll not found. Did you run 'cargo build -r --target x86_64-pc-windows-msvc'?
    pause
    exit /b 1
)

:: Ensure plugin directory exists
if not exist "%PLUGIN_DIR%" mkdir "%PLUGIN_DIR%"

:: Copy only if the source is newer (/D) and overwrite without asking (/Y)
xcopy /D /Y "%SOURCE%" "%PLUGIN_DIR%\"

if %ERRORLEVEL% EQU 0 (
    echo [SUCCESS] OBS Plugin emojikanban has been installed to "%PLUGIN_DIR%.
) else (
    echo [ERROR] Copy failed. Is OBS Studio still open?
)

pause
