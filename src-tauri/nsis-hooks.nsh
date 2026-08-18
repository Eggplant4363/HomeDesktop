; HomeDesktop NSIS 安装钩子
; 把 WebView2Loader.dll 从 resources/ 复制到安装根目录（exe 同目录）
; 原因：exe 在进程启动时静态导入该 DLL，必须与 exe 同目录才能解析。

!macro NSIS_HOOK_POSTINSTALL
  IfFileExists "$INSTDIR\resources\WebView2Loader.dll" 0 +3
    CopyFiles /SILENT "$INSTDIR\resources\WebView2Loader.dll" "$INSTDIR\"
    DetailPrint "已复制 WebView2Loader.dll 到应用目录"
!macroend
