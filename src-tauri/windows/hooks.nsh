; Terminate running instances before install, update, or uninstall.

!macro StopDailyHuddle
  DetailPrint "Stopping Daily Team Huddle..."
  ExecWait 'taskkill /F /T /IM "Daily Team Huddle.exe"' $0
  ExecWait 'taskkill /F /T /IM "daily-huddle.exe"' $0
  Sleep 1000
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro StopDailyHuddle
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro StopDailyHuddle
!macroend
