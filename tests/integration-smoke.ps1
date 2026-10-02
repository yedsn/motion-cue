$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [Text.Encoding]::UTF8

$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$exe = Join-Path $projectRoot "src-tauri\target\release\motion-cue.exe"
if (-not (Test-Path -LiteralPath $exe)) {
  throw "Release executable not found: $exe"
}

$running = @(Get-CimInstance Win32_Process -Filter "Name='motion-cue.exe'" | Where-Object {
  $_.ExecutablePath -and $_.ExecutablePath.Equals($exe, [StringComparison]::OrdinalIgnoreCase)
})
foreach ($process in $running) {
  Stop-Process -Id $process.ProcessId -Force
}
Start-Sleep -Milliseconds 300

$runtimeDir = Join-Path $env:APPDATA "com.motioncue.desktop\runtime"
$endpointPath = Join-Path $runtimeDir "ipc.json"
Remove-Item -LiteralPath $endpointPath -Force -ErrorAction SilentlyContinue

$appDataDir = Join-Path $env:APPDATA "com.motioncue.desktop"
$configPath = Join-Path $appDataDir "config.json"
$backupPath = Join-Path $appDataDir "config.backup.json"

function Invoke-MotionCue([string[]] $Arguments) {
  Write-Host ("Running: motion-cue " + ($Arguments -join " "))
  $stdoutPath = [IO.Path]::GetTempFileName()
  $stderrPath = [IO.Path]::GetTempFileName()
  try {
    $process = Start-Process -FilePath $exe -ArgumentList $Arguments -WindowStyle Hidden -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath -PassThru
    if (-not $process.WaitForExit(15000)) {
      Stop-Process -Id $process.Id -Force
      throw "Command timed out after 15 seconds: motion-cue $($Arguments -join ' ')"
    }
    $output = @(
      (Get-Content -LiteralPath $stdoutPath -Raw -Encoding UTF8 -ErrorAction SilentlyContinue),
      (Get-Content -LiteralPath $stderrPath -Raw -Encoding UTF8 -ErrorAction SilentlyContinue)
    ) -join "`n"
    [pscustomobject]@{
      Arguments = ($Arguments -join " ")
      ExitCode = $process.ExitCode
      Output = $output.Trim()
    }
  } finally {
    Remove-Item -LiteralPath $stdoutPath, $stderrPath -Force -ErrorAction SilentlyContinue
  }
}

$list = Invoke-MotionCue @("list", "--json")
$catalog = $list.Output | ConvertFrom-Json
if (@($catalog).Count -ne 9) { throw "Expected nine built-in animations, got $(@($catalog).Count)" }
$commands = @($catalog | ForEach-Object { $_.command })
foreach ($expected in @("confetti", "task-complete", "success", "focus-start", "error", "silent-confetti", "material-flow", "corner-fireworks", "focus-spotlight")) {
  if ($commands -notcontains $expected) { throw "Missing built-in animation: $expected" }
}
if ($commands -contains "milestone") { throw "Fresh catalog should not include milestone" }

$validConfig = Get-Content -LiteralPath $configPath -Raw -Encoding UTF8
Set-Content -LiteralPath $backupPath -Value $validConfig -Encoding UTF8
Set-Content -LiteralPath $configPath -Value "{ invalid json" -Encoding UTF8
$running = @(Get-CimInstance Win32_Process -Filter "Name='motion-cue.exe'" | Where-Object {
  $_.ExecutablePath -and $_.ExecutablePath.Equals($exe, [StringComparison]::OrdinalIgnoreCase)
})
foreach ($process in $running) { Stop-Process -Id $process.ProcessId -Force }
Start-Sleep -Milliseconds 300
Remove-Item -LiteralPath $endpointPath -Force -ErrorAction SilentlyContinue
$recovered = Invoke-MotionCue @("list", "--json")
$recoveredCatalog = $recovered.Output | ConvertFrom-Json
if (@($recoveredCatalog).Count -ne 9) { throw "Config recovery failed; got $(@($recoveredCatalog).Count) animations" }

$badPackageRoot = Join-Path ([IO.Path]::GetTempPath()) ("motioncue-bad-package-" + [Guid]::NewGuid())
$badPackage = Join-Path ([IO.Path]::GetTempPath()) ("motioncue-bad-package-" + [Guid]::NewGuid() + ".zip")
New-Item -ItemType Directory -Force -Path $badPackageRoot | Out-Null
Set-Content -LiteralPath (Join-Path $badPackageRoot "package.json") -Value '{"schemaVersion":999}' -Encoding UTF8
Set-Content -LiteralPath (Join-Path $badPackageRoot "animation.json") -Value '{}' -Encoding UTF8
Compress-Archive -Path (Join-Path $badPackageRoot "*") -DestinationPath $badPackage -Force
$badImport = Invoke-MotionCue @("import-animation", $badPackage)
if ($badImport.ExitCode -ne 1) { throw "unsupported animation package should fail, got $($badImport.ExitCode)" }
$afterBadImport = Invoke-MotionCue @("list", "--json")
$afterBadImportCatalog = $afterBadImport.Output | ConvertFrom-Json
if (@($afterBadImportCatalog).Count -ne 9) { throw "bad package import changed catalog" }
Remove-Item -LiteralPath $badPackage -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $badPackageRoot -Recurse -Force -ErrorAction SilentlyContinue

$pluginRoot = Join-Path ([IO.Path]::GetTempPath()) ("motioncue-plugin-" + [Guid]::NewGuid())
$pluginPackage = Join-Path ([IO.Path]::GetTempPath()) ("motioncue-plugin-" + [Guid]::NewGuid() + ".zip")
New-Item -ItemType Directory -Force -Path $pluginRoot | Out-Null
Set-Content -LiteralPath (Join-Path $pluginRoot "manifest.json") -Encoding UTF8 -Value '{"schemaVersion":1,"id":"smoke-plugin","name":"Smoke Plugin","version":"1.0.0","entry":"index.html","command":"smoke-plugin","aliases":[],"resources":[],"sounds":{},"capabilities":{"webgl":false,"worker":false}}'
Set-Content -LiteralPath (Join-Path $pluginRoot "index.html") -Encoding UTF8 -Value '<!doctype html><html><body><script>motionCue.onPlay(()=>motionCue.complete());motionCue.ready();</script></body></html>'
Compress-Archive -Path (Join-Path $pluginRoot "*") -DestinationPath $pluginPackage -Force
$installPlugin = Invoke-MotionCue @("install-plugin", $pluginPackage, "--allow-upgrade")
if ($installPlugin.ExitCode -ne 0 -and -not $installPlugin.Output) { throw "plugin install failed: $($installPlugin.Output)" }
$enablePlugin = Invoke-MotionCue @("enable-plugin", "smoke-plugin")
if ($enablePlugin.ExitCode -ne 0 -and -not $enablePlugin.Output) { throw "plugin enable failed: $($enablePlugin.Output)" }
$playPlugin = Invoke-MotionCue @("play", "smoke-plugin", "--duration", "4000")
if ($playPlugin.ExitCode -ne 0 -and -not $playPlugin.Output) { throw "plugin play failed: $($playPlugin.Output)" }
Start-Sleep -Milliseconds 500
$uninstallPlugin = Invoke-MotionCue @("uninstall-plugin", "smoke-plugin")
if ($uninstallPlugin.ExitCode -ne 0 -and -not $uninstallPlugin.Output) { throw "plugin uninstall failed: $($uninstallPlugin.Output)" }
$afterPlugin = Invoke-MotionCue @("play", "smoke-plugin")
if ($afterPlugin.ExitCode -ne 1) { throw "plugin command should be unavailable after uninstall" }
Remove-Item -LiteralPath $pluginPackage -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $pluginRoot -Recurse -Force -ErrorAction SilentlyContinue

$play = Invoke-MotionCue @("play", "success", "--text", "integration-smoke", "--duration", "1200")
if ($play.ExitCode -ne 0 -and -not $play.Output) { throw "play failed: $($play.Output)" }

$protocol = Invoke-MotionCue @("motioncue://play/task-complete?text=protocol-smoke")
if ($protocol.ExitCode -ne 0 -and -not $protocol.Output) { throw "protocol invocation failed: $($protocol.Output)" }

$stop = Invoke-MotionCue @("stop")
if ($stop.ExitCode -ne 0 -and -not $stop.Output) { throw "stop failed: $($stop.Output)" }

$unknown = Invoke-MotionCue @("play", "missing-animation")
if ($unknown.ExitCode -ne 1) { throw "unknown command should exit 1, got $($unknown.ExitCode)" }

$instances = @(Get-CimInstance Win32_Process -Filter "Name='motion-cue.exe'" | Where-Object {
  $_.ExecutablePath -and $_.ExecutablePath.Equals($exe, [StringComparison]::OrdinalIgnoreCase)
})
if ($instances.Count -ne 1) { throw "Expected one resident instance, got $($instances.Count)" }

[pscustomobject]@{
  status = "passed"
  executable = $exe
  builtInAnimations = @($catalog).Count
  residentInstances = $instances.Count
  checks = @("list-json", "config-recovery", "package-import-rollback", "plugin-teardown", "play", "protocol", "stop", "unknown-command-exit-code")
} | ConvertTo-Json
