Add-Type -AssemblyName System.Drawing

$proc = Get-Process app -ErrorAction Stop | Select-Object -First 1
$hwnd = $proc.MainWindowHandle

Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class Win32Print {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, int nFlags);
  public struct RECT {
    public int Left;
    public int Top;
    public int Right;
    public int Bottom;
  }
}
"@

$rect = New-Object Win32Print+RECT
[Win32Print]::GetWindowRect($hwnd, [ref]$rect) | Out-Null

$width = $rect.Right - $rect.Left
$height = $rect.Bottom - $rect.Top

if ($width -le 0 -or $height -le 0) {
  throw "Native window bounds are invalid: ${width}x${height}"
}

$bmp = New-Object System.Drawing.Bitmap $width, $height
$graphics = [System.Drawing.Graphics]::FromImage($bmp)
$hdc = $graphics.GetHdc()

[void][Win32Print]::PrintWindow($hwnd, $hdc, 0)

$graphics.ReleaseHdc($hdc)
$graphics.Dispose()

$path = "D:\source\ssh-ssistant-tauri\.playwright-cli\g4-native-window-print.png"
$bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()

Write-Output $path
