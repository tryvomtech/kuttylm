; Extra steps for the KuttyLM Windows installer (Tauri NSIS hooks).

!macro NSIS_HOOK_POSTINSTALL
  ; The model engine (llama-server) needs the Microsoft Visual C++ runtime.
  ; Install it only if it is missing or older than 14.44.
  SetRegView 64
  ReadRegDWORD $0 HKLM "SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64" "Installed"
  ReadRegDWORD $1 HKLM "SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64" "Minor"
  SetRegView default
  ${If} $0 != 1
  ${OrIf} $1 < 44
    DetailPrint "Installing the Microsoft Visual C++ runtime..."
    ExecWait '"$INSTDIR\redist\vc_redist.x64.exe" /install /quiet /norestart' $2
    DetailPrint "Visual C++ runtime setup finished (code $2)."
  ${EndIf}
  ; Not needed after install.
  Delete "$INSTDIR\redist\vc_redist.x64.exe"
  RMDir "$INSTDIR\redist"
!macroend
