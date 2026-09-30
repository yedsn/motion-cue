$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [Text.Encoding]::UTF8

$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$outputDir = Join-Path $projectRoot "artifacts"
New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

function New-AdversarialPackage([string] $SourceName, [string] $OutputName) {
  $source = Join-Path $projectRoot "tests\$SourceName"
  $output = Join-Path $outputDir $OutputName
  if (-not (Test-Path -LiteralPath (Join-Path $source "manifest.json"))) { throw "Missing manifest.json in $source" }
  if (-not (Test-Path -LiteralPath (Join-Path $source "index.html"))) { throw "Missing index.html in $source" }
  Remove-Item -LiteralPath $output -Force -ErrorAction SilentlyContinue
  Compress-Archive -Path (Join-Path $source "*") -DestinationPath $output -Force
  [pscustomobject]@{
    package = $output
    manifest = Join-Path $source "manifest.json"
    entry = Join-Path $source "index.html"
  }
}

$packages = @(
  New-AdversarialPackage "adversarial-plugin" "adversarial-plugin.zip"
  New-AdversarialPackage "adversarial-abuse-plugin" "adversarial-abuse-plugin.zip"
)

[pscustomobject]@{
  status = "passed"
  packages = @($packages)
} | ConvertTo-Json
