param(
  [string]$RepoName = "xixi",
  [switch]$Private
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Require-Command {
  param([string]$Name)
  if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
    throw "Required command not found: $Name"
  }
}

Require-Command git
Require-Command gh

$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

$authOutput = & gh auth status -h github.com 2>&1
if ($LASTEXITCODE -ne 0) {
  Write-Host "GitHub auth not ready. Run: gh auth login" -ForegroundColor Yellow
  Write-Host $authOutput
  exit 1
}

$remote = & git remote
if ($remote -match "origin") {
  Write-Host "Origin already exists. Pushing current branch..." -ForegroundColor Cyan
  & git push -u origin main
  exit $LASTEXITCODE
}

$visibilityFlag = if ($Private) { "--private" } else { "--public" }
Write-Host "Creating and pushing GitHub repo '$RepoName'..." -ForegroundColor Cyan
& gh repo create $RepoName $visibilityFlag --source . --remote origin --push
