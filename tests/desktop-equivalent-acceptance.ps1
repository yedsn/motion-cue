$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [Text.Encoding]::UTF8

$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$desktopScript = Join-Path $projectRoot "tests\desktop-acceptance.ps1"

function Invoke-Checked([string] $FilePath, [string[]] $Arguments, [int] $TimeoutMs = 120000) {
  $stdoutPath = [IO.Path]::GetTempFileName()
  $stderrPath = [IO.Path]::GetTempFileName()
  try {
    $process = Start-Process -FilePath $FilePath -ArgumentList $Arguments -WindowStyle Hidden -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath -PassThru
    if (-not $process.WaitForExit($TimeoutMs)) {
      Stop-Process -Id $process.Id -Force
      throw "Command timed out: $FilePath $($Arguments -join ' ')"
    }
    $output = (@(
      (Get-Content -LiteralPath $stdoutPath -Raw -Encoding UTF8 -ErrorAction SilentlyContinue),
      (Get-Content -LiteralPath $stderrPath -Raw -Encoding UTF8 -ErrorAction SilentlyContinue)
    ) -join "`n").Trim()
    if ($process.ExitCode -ne 0) { throw "Command failed: $FilePath $($Arguments -join ' ')`n$output" }
    $output
  } finally {
    Remove-Item -LiteralPath $stdoutPath, $stderrPath -Force -ErrorAction SilentlyContinue
  }
}

$desktopOutput = Invoke-Checked "pwsh" @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $desktopScript, "-AllowPartial") 90000
$desktopResult = $desktopOutput | ConvertFrom-Json
if ($desktopResult.failed.Count -gt 0) { throw "Desktop acceptance failed: $($desktopResult.failed -join ', ')" }

$requiredDesktopChecks = @(
  "playSubmitted",
  "overlayCreated",
  "visibleOverlay",
  "allOverlaysTransparent",
  "allOverlaysNoActivate",
  "cursorDoesNotHitMotionCue",
  "topmostModeAppliesTopmost",
  "nonTopmostModeClearsTopmost",
  "nonTopmostModeStillTransparent",
  "stopSubmitted",
  "stopHidesOverlays"
)
foreach ($check in $requiredDesktopChecks) {
  if (-not $desktopResult.checks.$check) { throw "Desktop check did not pass: $check" }
}

$rustOutput = Invoke-Checked "cargo" @("test", "--manifest-path", "src-tauri/Cargo.toml") 120000
$requiredRustTests = @(
  "playback::tests::plans_all_mixed_dpi_monitors_with_physical_bounds",
  "playback::tests::monitor_signatures_detect_hotplug_scale_and_position_changes",
  "playback::tests::assigns_audio_to_one_overlay_only",
  "playback::tests::session_timeout_has_safety_bounds"
)
foreach ($testName in $requiredRustTests) {
  if ($rustOutput -notlike "*$testName*ok*") { throw "Focused Rust evidence missing or failed: $testName" }
}

[pscustomobject]@{
  status = "passed-equivalent"
  desktopStatus = $desktopResult.status
  physicalDisplayCount = @($desktopResult.screens).Count
  desktopChecks = $desktopResult.checks
  equivalentCoverage = [pscustomobject]@{
    mixedDpiMultiMonitorPlanning = $true
    topologyHotplugDetection = $true
    audioOnceAcrossOverlays = $true
    timeoutSafetyBounds = $true
    pointerThroughNoActivateTopmostModes = $true
    stopCleanup = $true
  }
  limitations = @($desktopResult.limitations)
  rustFocusedTests = @($requiredRustTests)
} | ConvertTo-Json -Depth 8
