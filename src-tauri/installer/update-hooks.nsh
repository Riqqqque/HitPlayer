!include WinMessages.nsh

!macro NSIS_HOOK_PREINSTALL
  DetailPrint "Preparing HitPlayer for update..."

  FindWindow $0 "" "HitPlayer"
  IntCmp $0 0 hitplayer_cleanup_old_files
    MessageBox MB_OKCANCEL|MB_ICONINFORMATION "HitPlayer is currently running. The installer needs to close it before updating." IDOK hitplayer_close_running IDCANCEL hitplayer_abort_update

  hitplayer_close_running:
    SendMessage $0 ${WM_CLOSE} 0 0
    Sleep 1500
    FindWindow $0 "" "HitPlayer"
    IntCmp $0 0 hitplayer_cleanup_old_files
      DetailPrint "HitPlayer did not exit after a normal close request. Closing the process for update..."
      nsExec::ExecToStack 'taskkill /IM "hitplayer.exe" /T /F'
      Pop $1
      Pop $2
      Sleep 750
      Goto hitplayer_cleanup_old_files

  hitplayer_abort_update:
    Abort "Update canceled. Close HitPlayer and run the installer again."

  hitplayer_cleanup_old_files:
    IfFileExists "$INSTDIR\hitplayer.exe" 0 hitplayer_cleanup_sidecars
      ClearErrors
      Delete "$INSTDIR\hitplayer.exe"
      IfErrors hitplayer_locked hitplayer_cleanup_sidecars

  hitplayer_locked:
    MessageBox MB_ICONSTOP|MB_OK "HitPlayer could not be updated because the existing app is still running or locked. Close HitPlayer and run this installer again."
    Abort

  hitplayer_cleanup_sidecars:
    Delete "$SMPROGRAMS\HitPlayer.lnk"
    Delete "$SMPROGRAMS\HitPlayer\HitPlayer.lnk"
    RMDir "$SMPROGRAMS\HitPlayer"
    Delete "$DESKTOP\HitPlayer.lnk"
    Delete "$INSTDIR\ffmpeg.exe"
    Delete "$INSTDIR\ffprobe.exe"
    RMDir /r "$INSTDIR\resources"
!macroend

!macro NSIS_HOOK_POSTINSTALL
  DetailPrint "Refreshing HitPlayer shell icons..."
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0x0000, p 0, p 0)'
!macroend
