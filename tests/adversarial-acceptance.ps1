$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [Text.Encoding]::UTF8

$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$exe = Join-Path $projectRoot "src-tauri\target\release\motion-cue.exe"
$buildScript = Join-Path $projectRoot "tests\build-adversarial-plugin.ps1"
$package = Join-Path $projectRoot "artifacts\adversarial-plugin.zip"
$abusePackage = Join-Path $projectRoot "artifacts\adversarial-abuse-plugin.zip"

if (-not (Test-Path -LiteralPath $exe)) { throw "Release executable not found: $exe" }

function Stop-MotionCue {
  Get-CimInstance Win32_Process -Filter "Name='motion-cue.exe'" | Where-Object {
    $_.ExecutablePath -and $_.ExecutablePath.Equals($exe, [StringComparison]::OrdinalIgnoreCase)
  } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force }
}

function Invoke-MotionCue([string[]] $Arguments, [int] $TimeoutMs = 20000) {
  $stdoutPath = [IO.Path]::GetTempFileName()
  $stderrPath = [IO.Path]::GetTempFileName()
  try {
    $process = Start-Process -FilePath $exe -ArgumentList $Arguments -WindowStyle Hidden -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath -PassThru
    if (-not $process.WaitForExit($TimeoutMs)) {
      Stop-Process -Id $process.Id -Force
      throw "Command timed out: motion-cue $($Arguments -join ' ')"
    }
    [pscustomobject]@{
      Arguments = ($Arguments -join " ")
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

function Wait-MotionCueReady([int] $TimeoutMs = 10000) {
  $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
  do {
    $result = Invoke-MotionCue @("list", "--json") 5000
    if ($result.ExitCode -eq 0 -and $result.Output.Trim().StartsWith("[")) { return }
    Start-Sleep -Milliseconds 300
  } while ([DateTime]::UtcNow -lt $deadline)
  throw "MotionCue did not become ready in time"
}

function Assert-Success($Result, [string] $Label) {
  if ($Result.ExitCode -ne 0) { throw "$Label failed: $($Result.Output)" }
}

function Get-Diagnostics {
  $result = Invoke-MotionCue @("diagnostics", "--json")
  Assert-Success $result "diagnostics"
  @($result.Output | ConvertFrom-Json)
}

function Wait-PluginError([string] $Needle, [long] $SinceTimestamp, [int] $TimeoutMs = 10000) {
  $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
  do {
    $entries = Get-Diagnostics
    $entry = @($entries | Where-Object { $_.timestamp -ge $SinceTimestamp -and $_.category -eq "plugin" -and $_.level -eq "error" -and $_.message -like "*$Needle*" } | Select-Object -First 1)
    if ($entry.Count -gt 0) { return $entry[0] }
    Start-Sleep -Milliseconds 300
  } while ([DateTime]::UtcNow -lt $deadline)
  throw "Timed out waiting for plugin diagnostic containing: $Needle"
}

function Assert-BlockedResults($Entry) {
  $results = $Entry.message | ConvertFrom-Json
  $required = @(
    "network",
    "websocket",
    "remoteImage",
    "remoteScript",
    "fileImage",
    "fileFrame",
    "tauri",
    "node",
    "parentDocument",
    "topDocument",
    "popup",
    "filePicker",
    "form",
    "navigation",
    "storage",
    "worker",
    "webgl"
  )
  $failed = @()
  foreach ($key in $required) {
    if ($results.$key -ne "blocked") { $failed += "$key=$($results.$key)" }
  }
  if ($failed.Count -gt 0) { throw "Adversarial plugin escaped policy: $($failed -join ', ')" }
  $results
}

try {
  Stop-MotionCue
  Start-Sleep -Milliseconds 300

  & pwsh -NoProfile -ExecutionPolicy Bypass -File $buildScript | Out-Null
  if (-not (Test-Path -LiteralPath $package)) { throw "Missing adversarial package: $package" }
  if (-not (Test-Path -LiteralPath $abusePackage)) { throw "Missing abuse package: $abusePackage" }

  Assert-Success (Invoke-MotionCue @("install-plugin", $package, "--allow-upgrade")) "install adversarial plugin"
  Wait-MotionCueReady
  Assert-Success (Invoke-MotionCue @("enable-plugin", "adversarial-probe")) "enable adversarial plugin"
  Wait-MotionCueReady
  $policyStarted = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds() - 1000
  Assert-Success (Invoke-MotionCue @("play", "adversarial-probe", "--duration", "12000")) "play adversarial plugin"
  $policyEntry = Wait-PluginError "network" $policyStarted
  $policyResults = Assert-BlockedResults $policyEntry
  Assert-Success (Invoke-MotionCue @("stop")) "stop adversarial plugin"

  Assert-Success (Invoke-MotionCue @("install-plugin", $abusePackage, "--allow-upgrade")) "install abuse plugin"
  Wait-MotionCueReady
  Assert-Success (Invoke-MotionCue @("enable-plugin", "adversarial-abuse-probe")) "enable abuse plugin"
  Wait-MotionCueReady
  $abuseStarted = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds() - 1000
  Assert-Success (Invoke-MotionCue @("play", "adversarial-abuse-probe", "--duration", "12000")) "play abuse plugin"
  $abuseEntry = Wait-PluginError "插件消息频率超过限制" $abuseStarted
  Assert-Success (Invoke-MotionCue @("stop")) "stop abuse plugin"

  $postStop = Invoke-MotionCue @("play", "success", "--duration", "800")
  Assert-Success $postStop "host stability play after adversarial plugins"
  Assert-Success (Invoke-MotionCue @("stop")) "final stop"

  [pscustomobject]@{
    status = "passed"
    policyResults = $policyResults
    policyDiagnosticId = $policyEntry.id
    abuseDiagnosticId = $abuseEntry.id
    hostStableAfterViolation = $true
  } | ConvertTo-Json -Depth 8
} finally {
  try { Invoke-MotionCue @("uninstall-plugin", "adversarial-probe") | Out-Null } catch {}
  try { Invoke-MotionCue @("uninstall-plugin", "adversarial-abuse-probe") | Out-Null } catch {}
  try { Invoke-MotionCue @("stop") | Out-Null } catch {}
  Stop-MotionCue
}
