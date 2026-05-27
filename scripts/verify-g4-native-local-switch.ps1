Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class Win32NativeFlow {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint dwFlags, uint dx, uint dy, uint dwData, UIntPtr dwExtraInfo);
  public const uint MOUSEEVENTF_LEFTDOWN = 0x0002;
  public const uint MOUSEEVENTF_LEFTUP = 0x0004;
  public struct RECT {
    public int Left;
    public int Top;
    public int Right;
    public int Bottom;
  }
}
"@

function Get-AppWindowRect {
  $proc = Get-Process app -ErrorAction Stop | Select-Object -First 1
  $hwnd = $proc.MainWindowHandle
  [Win32NativeFlow]::ShowWindow($hwnd, 5) | Out-Null
  [Win32NativeFlow]::SetForegroundWindow($hwnd) | Out-Null
  Start-Sleep -Milliseconds 600
  $rect = New-Object Win32NativeFlow+RECT
  [Win32NativeFlow]::GetWindowRect($hwnd, [ref]$rect) | Out-Null
  return @{
    Handle = $hwnd
    Left = $rect.Left
    Top = $rect.Top
    Width = $rect.Right - $rect.Left
    Height = $rect.Bottom - $rect.Top
  }
}

function Capture-Window($path) {
  $rect = Get-AppWindowRect
  $bmp = New-Object System.Drawing.Bitmap $rect.Width, $rect.Height
  $graphics = [System.Drawing.Graphics]::FromImage($bmp)
  $graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bmp.Size)
  $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
  $graphics.Dispose()
  $bmp.Dispose()
}

function Click-Relative([int]$relativeX, [int]$relativeY) {
  $rect = Get-AppWindowRect
  $absoluteX = $rect.Left + $relativeX
  $absoluteY = $rect.Top + $relativeY
  [Win32NativeFlow]::SetCursorPos($absoluteX, $absoluteY) | Out-Null
  Start-Sleep -Milliseconds 150
  [Win32NativeFlow]::mouse_event([Win32NativeFlow]::MOUSEEVENTF_LEFTDOWN, 0, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 50
  [Win32NativeFlow]::mouse_event([Win32NativeFlow]::MOUSEEVENTF_LEFTUP, 0, 0, 0, [UIntPtr]::Zero)
}

$base = "D:\source\ssh-ssistant-tauri\.playwright-cli"
$beforePath = Join-Path $base "g4-native-before-local-switch.png"
$localPath = Join-Path $base "g4-native-local-workbench.png"
$afterPath = Join-Path $base "g4-native-back-to-gateway.png"

Capture-Window $beforePath

# LoginGateway: "切换为本地模式" button center
Click-Relative 1088 800
Start-Sleep -Seconds 2
Capture-Window $localPath

# Workbench sidebar bottom-left "Switch" button center
Click-Relative 28 902
Start-Sleep -Seconds 2
Capture-Window $afterPath

[PSCustomObject]@{
  before = $beforePath
  local = $localPath
  after = $afterPath
} | ConvertTo-Json -Compress
