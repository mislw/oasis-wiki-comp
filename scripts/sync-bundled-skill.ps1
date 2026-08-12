param(
    [string]$SkillRepository = (Join-Path $PSScriptRoot '..\..\oasis-wiki'),
    [switch]$DryRun
)

$ErrorActionPreference = 'Stop'

$companionRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$skillRepository = (Resolve-Path $SkillRepository).Path
$source = Join-Path $skillRepository 'oasis-wiki'
$target = Join-Path $companionRoot 'src-tauri\resources\skill'

if (-not (Test-Path (Join-Path $source 'SKILL.md'))) {
    throw "Skill repository is missing oasis-wiki\SKILL.md: $skillRepository"
}

$arguments = @(
    $source,
    $target,
    '/MIR',
    '/XF',
    'VERSION',
    '*.pyc',
    '/XD',
    '__pycache__',
    '/R:1',
    '/W:1',
    '/NFL',
    '/NDL',
    '/NJH',
    '/NJS',
    '/NP'
)

if ($DryRun) {
    $arguments += '/L'
}

& robocopy @arguments
$exitCode = $LASTEXITCODE
if ($exitCode -gt 7) {
    throw "Skill synchronization failed with robocopy exit code $exitCode"
}

Write-Host "Bundled Skill synchronized from $source"
