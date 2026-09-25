# Creates a worktree + branch for one issue, without launching an agent.
# From an OpenCode session, dispatch the actual work with the Task tool.
#
#   scripts/dispatch.ps1 -Issue 42 -Slug vr-core-manifest [-Base main]
#
# Each worktree gets its own CARGO_TARGET_DIR so parallel builds never collide.

param(
    [Parameter(Mandatory = $true)][int]$Issue,
    [Parameter(Mandatory = $true)][string]$Slug,
    [string]$Base = 'main'
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$cfg = Get-Content (Join-Path $repo '.agentloop') | Where-Object { $_ -match '=' } |
    ForEach-Object { $k, $v = $_ -split '=', 2; Set-Variable -Name $k.Trim() -Value $v.Trim() -Scope Script }
$branch = "feat/$Issue-$Slug"
$wt = Join-Path $WT_ROOT "issue-$Issue"

git -C $repo fetch origin --prune | Out-Null
if (Test-Path $wt) { throw "worktree already exists: $wt" }

git -C $repo worktree add -b $branch $wt "origin/$Base"

$target = Join-Path $WT_ROOT "target-$Issue"
Set-Content -Path (Join-Path $wt '.cargo-target') -Value $target
Write-Output "WORKTREE $wt BRANCH $branch"
Write-Output "Set CARGO_TARGET_DIR=$target when building this worktree."
