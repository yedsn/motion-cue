param(
  [switch] $AllowPartial
)

$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [Text.Encoding]::UTF8

$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$exe = Join-Path $projectRoot "src-tauri\target\release\motion-cue.exe"
if (-not (Test-Path -LiteralPath $exe)) {
  throw "Release executable not found: $exe"
}

$appDataDir = Join-Path $env:APPDATA "com.motioncue.desktop"
$configPath = Join-Path $appDataDir "config.json"
$originalConfig = if (Test-Path -LiteralPath $configPath) { Get-Content -LiteralPath $configPath -Raw -Encoding UTF8 } else { $null }

Add-Type -AssemblyName System.Windows.Forms

$nativeSource = @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class MotionCueDesktopProbeNative {
  public const int GWL_EXSTYLE = -20;
  public const int WS_EX_TRANSPARENT = 0x20;
  public const int WS_EX_NOACTIVATE = 0x08000000;
  public const int WS_EX_TOOLWINDOW = 0x80;
  public const int WS_EX_TOPMOST = 0x8;
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
  public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
  [DllImport("user32.dll")] public static extern int GetWindowThreadProcessId(IntPtr hWnd, out int processId);
  [DllImport("user32.dll", SetLastError=true)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int count);
  [DllImport("user32.dll", SetLastError=true)] public static extern IntPtr GetWindowLongPtr(IntPtr hWnd, int nIndex);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT point);
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT point);
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X; public int Y; }
}
"@
Add-Type -TypeDefinition $nativeSource

function Stop-MotionCue {
  Get-CimInstance Win32_Process -Filter "Name='motion-cue.exe'" | Where-Object {
    $_.ExecutablePath -and $_.ExecutablePath.Equals($exe, [StringComparison]::OrdinalIgnoreCase)
  } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force }
}

function Invoke-MotionCue([string[]] $Arguments) {
  $stdoutPath = [IO.Path]::GetTempFileName()
  $stderrPath = [IO.Path]::GetTempFileName()
  try {
    $process = Start-Process -FilePath $exe -ArgumentList $Arguments -WindowStyle Hidden -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath -PassThru
    if (-not $process.WaitForExit(15000)) {
      Stop-Process -Id $process.Id -Force
      throw "Command timed out: motion-cue $($Arguments -join ' ')"
    }
    [pscustomobject]@{
      ExitCode = $process.ExitCode
      Output = (@(
        (Get-Content -LiteralPath $stdoutPath -Raw -Encoding UTF8 -ErrorAction SilentlyContinue),
        (Get-Content -LiteralPath $stderrPath -Raw -Encoding UTF8 -ErrorAction SilentlyContinue)
      ) -join "`n").Trim()
    }
  } finally {
    Remove-Item -LiteralPath $stdoutPath, $stderrPath -Force -ErrorAction SilentlyContinue
  }
}

function Set-OverlayTopmost([bool] $Enabled) {
  if (-not (Test-Path -LiteralPath $configPath)) {
    $list = Invoke-MotionCue @("list", "--json")
    if ($list.ExitCode -ne 0) { throw "Unable to initialize MotionCue config: $($list.Output)" }
  }
  $config = Get-Content -LiteralPath $configPath -Raw -Encoding UTF8 | ConvertFrom-Json
  $config.settings.overlayTopmost = $Enabled
  $config | ConvertTo-Json -Depth 16 | Set-Content -LiteralPath $configPath -Encoding UTF8
}

function Get-OverlayProbe {
  $pids = @(Get-Process -Name motion-cue -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id)
  $overlayWindows = New-Object System.Collections.Generic.List[object]
  foreach ($pidValue in $pids) {
    [MotionCueDesktopProbeNative]::EnumWindows({ param($hWnd, $lParam)
      $windowPid = 0
      [void][MotionCueDesktopProbeNative]::GetWindowThreadProcessId($hWnd, [ref]$windowPid)
      if ($windowPid -eq $pidValue) {
        $title = [Text.StringBuilder]::new(256)
        [void][MotionCueDesktopProbeNative]::GetWindowText($hWnd, $title, 256)
        if ($title.ToString() -like '*Overlay*') {
          $ex = [MotionCueDesktopProbeNative]::GetWindowLongPtr($hWnd, [MotionCueDesktopProbeNative]::GWL_EXSTYLE).ToInt64()
          $overlayWindows.Add([pscustomobject]@{
            ProcessId = $windowPid
            Visible = [MotionCueDesktopProbeNative]::IsWindowVisible($hWnd)
            Title = $title.ToString()
            Transparent = (($ex -band [MotionCueDesktopProbeNative]::WS_EX_TRANSPARENT) -ne 0)
            NoActivate = (($ex -band [MotionCueDesktopProbeNative]::WS_EX_NOACTIVATE) -ne 0)
            ToolWindow = (($ex -band [MotionCueDesktopProbeNative]::WS_EX_TOOLWINDOW) -ne 0)
            Topmost = (($ex -band [MotionCueDesktopProbeNative]::WS_EX_TOPMOST) -ne 0)
          }) | Out-Null
        }
      }
      $true
    }, [IntPtr]::Zero) | Out-Null
  }

  $point = New-Object MotionCueDesktopProbeNative+POINT
  [void][MotionCueDesktopProbeNative]::GetCursorPos([ref]$point)
  $hit = [MotionCueDesktopProbeNative]::WindowFromPoint($point)
  $hitPid = 0
  [void][MotionCueDesktopProbeNative]::GetWindowThreadProcessId($hit, [ref]$hitPid)
  $hitProcessName = (Get-Process -Id $hitPid -ErrorAction SilentlyContinue).ProcessName

  [pscustomobject]@{
    overlayWindows = @($overlayWindows.ToArray())
    cursorHit = [pscustomobject]@{ processId = $hitPid; processName = $hitProcessName }
  }
}

function Invoke-OverlayPlaybackProbe([bool] $Topmost) {
  Set-OverlayTopmost $Topmost
  Stop-MotionCue
  Start-Sleep -Milliseconds 300
  $play = Invoke-MotionCue @("play", "task-complete", "--text", "desktop-acceptance", "--duration", "8000")
  Start-Sleep -Milliseconds 1800
  $probe = Get-OverlayProbe
  $stop = Invoke-MotionCue @("stop")
  Start-Sleep -Milliseconds 500
  $afterStopProbe = Get-OverlayProbe
  [pscustomobject]@{
    topmostSetting = $Topmost
    play = $play
    stop = $stop
    overlayWindows = $probe.overlayWindows
    cursorHit = $probe.cursorHit
    afterStopOverlayWindows = $afterStopProbe.overlayWindows
  }
}

Stop-MotionCue
Start-Sleep -Milliseconds 300

$screens = @([System.Windows.Forms.Screen]::AllScreens | ForEach-Object {
  [pscustomobject]@{
    DeviceName = $_.DeviceName
    Primary = $_.Primary
    X = $_.Bounds.X
    Y = $_.Bounds.Y
    Width = $_.Bounds.Width
    Height = $_.Bounds.Height
  }
})

try {
  $topmostProbe = Invoke-OverlayPlaybackProbe $true
  $normalProbe = Invoke-OverlayPlaybackProbe $false
} finally {
  Stop-MotionCue
  if ($null -ne $originalConfig) {
    Set-Content -LiteralPath $configPath -Value $originalConfig -Encoding UTF8
  }
}

$topmostOverlayWindows = @($topmostProbe.overlayWindows)
$normalOverlayWindows = @($normalProbe.overlayWindows)
$topmostAfterStopOverlayWindows = @($topmostProbe.afterStopOverlayWindows)
$normalAfterStopOverlayWindows = @($normalProbe.afterStopOverlayWindows)

$checks = [ordered]@{
  hasReleaseExecutable = $true
  playSubmitted = [bool]$topmostProbe.play.Output -and [bool]$normalProbe.play.Output
  overlayCreated = $topmostOverlayWindows.Count -gt 0 -and $normalOverlayWindows.Count -gt 0
  visibleOverlay = @($topmostOverlayWindows | Where-Object Visible).Count -gt 0 -and @($normalOverlayWindows | Where-Object Visible).Count -gt 0
  allOverlaysTransparent = $topmostOverlayWindows.Count -gt 0 -and @($topmostOverlayWindows | Where-Object { -not $_.Transparent }).Count -eq 0
  allOverlaysNoActivate = $topmostOverlayWindows.Count -gt 0 -and @($topmostOverlayWindows | Where-Object { -not $_.NoActivate }).Count -eq 0
  cursorDoesNotHitMotionCue = $topmostProbe.cursorHit.processName -ne "motion-cue" -and $normalProbe.cursorHit.processName -ne "motion-cue"
  topmostModeAppliesTopmost = $topmostOverlayWindows.Count -gt 0 -and @($topmostOverlayWindows | Where-Object { -not $_.Topmost }).Count -eq 0
  nonTopmostModeClearsTopmost = $normalOverlayWindows.Count -gt 0 -and @($normalOverlayWindows | Where-Object { $_.Topmost }).Count -eq 0
  nonTopmostModeStillTransparent = $normalOverlayWindows.Count -gt 0 -and @($normalOverlayWindows | Where-Object { -not $_.Transparent -or -not $_.NoActivate }).Count -eq 0
  stopSubmitted = [bool]$topmostProbe.stop.Output -and [bool]$normalProbe.stop.Output
  stopHidesOverlays = @($topmostAfterStopOverlayWindows | Where-Object Visible).Count -eq 0 -and @($normalAfterStopOverlayWindows | Where-Object Visible).Count -eq 0
  multiMonitorAvailable = $screens.Count -ge 2
}

$required = @("playSubmitted", "overlayCreated", "visibleOverlay", "allOverlaysTransparent", "allOverlaysNoActivate", "cursorDoesNotHitMotionCue", "topmostModeAppliesTopmost", "nonTopmostModeClearsTopmost", "nonTopmostModeStillTransparent", "stopSubmitted", "stopHidesOverlays")
$failed = @($required | Where-Object { -not $checks[$_] })
$status = if ($failed.Count -eq 0 -and $checks.multiMonitorAvailable) { "passed" } elseif ($failed.Count -eq 0) { "partial" } else { "failed" }

$limitations = @()
if ($screens.Count -lt 2) {
  $limitations += "Mixed-DPI and hot-plug checks require at least two displays; current environment exposed $($screens.Count)."
}

$result = [pscustomobject]@{
  status = $status
  checks = [pscustomobject]$checks
  failed = @($failed)
  screens = @($screens)
  topmostOverlayWindows = @($topmostOverlayWindows)
  normalOverlayWindows = @($normalOverlayWindows)
  topmostAfterStopOverlayWindows = @($topmostAfterStopOverlayWindows)
  normalAfterStopOverlayWindows = @($normalAfterStopOverlayWindows)
  topmostCursorHit = $topmostProbe.cursorHit
  normalCursorHit = $normalProbe.cursorHit
  limitations = @($limitations)
}

$result | ConvertTo-Json -Depth 8
if ($status -eq "failed" -or ($status -eq "partial" -and -not $AllowPartial)) {
  exit 1
}
