param(
    [Parameter(Mandatory = $true)]
    [string] $Query,

    [int] $MaxResults = 10,

    [int] $Context = 1,

    [switch] $LocationsOnly
)

$ErrorActionPreference = 'Stop'

[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$OutputEncoding = [System.Text.UTF8Encoding]::new($false)

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$skillRoot = Split-Path -Parent $scriptDir
$wikiRoot = Join-Path $skillRoot 'references\wiki'

if (-not (Test-Path -LiteralPath $wikiRoot)) {
    throw "Wiki root not found: $wikiRoot"
}

$effectiveContext = if ($LocationsOnly) { 0 } else { $Context }

$rg = Get-Command rg -ErrorAction SilentlyContinue
if ($rg) {
    $args = @(
        '--line-number',
        '--smart-case',
        '--glob', '*.md'
    )
    if ($effectiveContext -gt 0) {
        $args += @('--context', [string]$effectiveContext)
    }
    $args += @($Query, $wikiRoot)
    $results = & rg @args
    $rgExit = $LASTEXITCODE
    $results | Select-Object -First $MaxResults | ForEach-Object {
        if ($LocationsOnly) {
            $_ -replace '^(.+):(\d+):.*$', '$1:$2'
        } else {
            $_
        }
    }
    if ($rgExit -eq 1 -and -not $results) {
        exit 1
    }
    exit 0
}

Get-ChildItem -LiteralPath $wikiRoot -Filter '*.md' -Recurse |
    Select-String -Pattern $Query -CaseSensitive:$false -Context $effectiveContext |
    Select-Object -First $MaxResults |
    ForEach-Object {
        if ($LocationsOnly) {
            "$($_.Path):$($_.LineNumber)"
        } else {
            "$($_.Path):$($_.LineNumber):$($_.Line)"
        }
    }
