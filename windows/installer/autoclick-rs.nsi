; ===========================================================================
; AutoClick-RS — NSIS Installer
; Produces a single setup .exe with a standard install wizard.
;
; Build:
;   makensis autoclick-rs.nsi
;
; Inputs (place next to this script before building):
;   autoclick-rs.exe        (the Rust binary)
;   autoclick-rs.ico        (app icon for shortcuts + installer)
; ===========================================================================

Unicode true
ManifestDPIAware true

!define APPNAME        "AutoClick-RS"
!define COMPANY        "AutoClick-RS"
!define VERSION        "1.4.5"
!define VERSION_MAJOR  1
!define VERSION_MINOR  4
!define VERSION_PATCH  5
!define EXE            "autoclick-rs.exe"
!define LIC            "LICENSE.txt"

Name "${APPNAME} ${VERSION}"
OutFile "AutoClick-RS-Setup-${VERSION}.exe"
InstallDir "$LOCALAPPDATA\Programs\${APPNAME}"
InstallDirRegKey HKCU "Software\${APPNAME}" "InstallDir"
RequestExecutionLevel user
ShowInstDetails   show
ShowUnInstDetails show
SetCompressor /SOLID lzma
BrandingText " "

; -------- Modern UI 2 --------
!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "FileFunc.nsh"

!define MUI_ICON                 "autoclick-rs.ico"
!define MUI_UNICON               "autoclick-rs.ico"
!define MUI_ABORTWARNING
!define MUI_COMPONENTSPAGE_SMALLDESC
!define MUI_FINISHPAGE_RUN
!define MUI_FINISHPAGE_RUN_TEXT  "Launch ${APPNAME} now"
!define MUI_FINISHPAGE_RUN_FUNCTION LaunchApp
!define MUI_FINISHPAGE_SHOWREADME "$INSTDIR\README.txt"
!define MUI_FINISHPAGE_SHOWREADME_TEXT "View README"
!define MUI_FINISHPAGE_SHOWREADME_NOTCHECKED

; Pages — install
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${LIC}"
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

; Pages — uninstall
!insertmacro MUI_UNPAGE_WELCOME
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

; Language
!insertmacro MUI_LANGUAGE "English"

; -------- Language strings --------
LangString DESC_core   ${LANG_ENGLISH} "The ${APPNAME} application binary (required)."
LangString DESC_shortcut ${LANG_ENGLISH} "Create Start Menu and Desktop shortcuts."
LangString DESC_assoc   ${LANG_ENGLISH} "Add to the application search index so ${APPNAME} appears in Start menu search."

; -------- Functions --------
Function LaunchApp
    Exec '"$INSTDIR\${EXE}"'
FunctionEnd

Function .onInit
    ; Auto-uninstall old version if found in the install dir.
    ReadRegStr $0 HKCU "Software\${APPNAME}" "InstallDir"
    ${If} $0 != ""
        IfFileExists "$0\Uninstall.exe" 0 +3
        MessageBox MB_YESNO|MB_ICONQUESTION "An older ${APPNAME} is installed at $0.$\nUninstall it first?" IDYES +2 IDNO 0
        Abort
        ExecWait "$0\Uninstall.exe /S _?=$0"
        Delete "$0\Uninstall.exe"
    ${EndIf}
FunctionEnd

; ===========================================================================
; Sections
; ===========================================================================

Section "${APPNAME}" SecCore
    SectionIn RO  ; Read-only (always installed)
    SetOutPath "$INSTDIR"
    ; Main binary
    File "${EXE}"
    ; Icon (used by shortcuts and uninstaller)
    File "autoclick-rs.ico"
    ; README
    File /nonfatal "README.txt"

    ; Write uninstaller
    WriteUninstaller "$INSTDIR\Uninstall.exe"

    ; Registry — install dir + Add/Remove Programs entry (per-user)
    WriteRegStr HKCU "Software\${APPNAME}" "InstallDir" "$INSTDIR"
    WriteRegStr HKCU "Software\${APPNAME}" "Version"    "${VERSION}"

    WriteRegStr   HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "DisplayName"     "${APPNAME}"
    WriteRegStr   HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "DisplayVersion"  "${VERSION}"
    WriteRegStr   HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "Publisher"       "${COMPANY}"
    WriteRegStr   HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "InstallLocation" "$INSTDIR"
    WriteRegStr   HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "DisplayIcon"     "$INSTDIR\autoclick-rs.ico"
    WriteRegStr   HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
    WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "NoModify" 1
    WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "NoRepair" 1

    ; Estimate installed size for Add/Remove Programs
    ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
    IntFmt $0 "0x%08X" $0
    WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "EstimatedSize" "$0"
SectionEnd

Section "Shortcuts" SecShortcut
    SectionIn 1  ; default-enabled optional component

    ; Start Menu folder
    CreateDirectory "$SMPROGRAMS\${APPNAME}"
    CreateShortcut  "$SMPROGRAMS\${APPNAME}\${APPNAME}.lnk" \
                    "$INSTDIR\${EXE}" "" \
                    "$INSTDIR\autoclick-rs.ico" 0 \
                    "" "" "${APPNAME} — automatic key presser"

    ; Desktop shortcut (optional but convenient)
    CreateShortcut "$DESKTOP\${APPNAME}.lnk" \
                   "$INSTDIR\${EXE}" "" \
                   "$INSTDIR\autoclick-rs.ico" 0 \
                   "" "" "${APPNAME} — automatic key presser"
SectionEnd

Section "Add to Start Menu search" SecSearch
    ; App Paths registry entry — makes the exe discoverable from Start menu
    ; search box (Win+R or cortana) when typed by name.
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\App Paths\${EXE}" "" "$INSTDIR\${EXE}"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\App Paths\${EXE}" "Path" "$INSTDIR"
SectionEnd

; -------- Section descriptions --------
!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
!insertmacro MUI_DESCRIPTION_TEXT ${SecCore}     $(DESC_core)
!insertmacro MUI_DESCRIPTION_TEXT ${SecShortcut} $(DESC_shortcut)
!insertmacro MUI_DESCRIPTION_TEXT ${SecSearch}   $(DESC_assoc)
!insertmacro MUI_FUNCTION_DESCRIPTION_END

; ===========================================================================
; Uninstaller
; ===========================================================================
Section "Uninstall"
    ; Kill running instance (best-effort)
    ExecWait 'taskkill /F /IM ${EXE}'

    ; Remove files
    Delete "$INSTDIR\${EXE}"
    Delete "$INSTDIR\autoclick-rs.ico"
    Delete "$INSTDIR\README.txt"
    Delete "$INSTDIR\Uninstall.exe"

    ; Remove shortcuts
    Delete "$SMPROGRAMS\${APPNAME}\${APPNAME}.lnk"
    RMDir  "$SMPROGRAMS\${APPNAME}"
    Delete "$DESKTOP\${APPNAME}.lnk"

    ; Clean registry
    DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}"
    DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\App Paths\${EXE}"
    DeleteRegKey HKCU "Software\${APPNAME}"

    RMDir "$INSTDIR"
SectionEnd

; Version info embedded in the installer .exe itself
VIProductVersion "${VERSION_MAJOR}.${VERSION_MINOR}.${VERSION_PATCH}.0"
VIAddVersionKey "ProductName"      "${APPNAME}"
VIAddVersionKey "FileDescription"  "${APPNAME} installer"
VIAddVersionKey "CompanyName"       "${COMPANY}"
VIAddVersionKey "LegalCopyright"   "MIT License"
VIAddVersionKey "FileVersion"       "${VERSION}"
VIAddVersionKey "ProductVersion"    "${VERSION}"
