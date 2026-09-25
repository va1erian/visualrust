# Rebase, verify, push and rebase-merge one or more PRs, then remove their
# worktrees. One landing at a time (guarded by a lock file).
#
#   scripts/land.ps1 -Pr 12, 13

param([Parameter(Mandatory = $true)][int[]]$Pr)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$lock = Join-Path $repo '.git\land.lock'
if (Test-Path $lock) { throw "another landing is in progress ($lock); remove it only if stale" }
New-Item -ItemType File -Path $lock | Out-Null

try {
    foreach ($n in $Pr) {
        $json = gh pr view $n --repo va1erian/visualrust --json state,headRefName,mergeable 2>&1 | ConvertFrom-Json
        if ($json.state -ne 'OPEN') { Write-Output "PR ${n}: SKIP ($($json.state))"; continue }

        $branch = $json.headRefName
        $wt = (git -C $repo worktree list --porcelain |
            Select-String -Pattern '^worktree ' | ForEach-Object { $_.Line.Substring(9) } |
            Where-Object { (git -C $_ rev-parse --abbrev-ref HEAD) -eq $branch } | Select-Object -First 1)
        if (-not $wt) { Write-Output "PR ${n}: SKIP (no worktree for $branch)"; continue }

        git -C $wt fetch origin --prune | Out-Null
        if (-not (git -C $wt rebase "origin/main")) {
            git -C $wt rebase --abort 2>$null
            Write-Output "PR ${n}: CONFLICT needs manual resolution"
            continue
        }

        $target = Join-Path (Split-Path $wt) "target-$((Split-Path $wt -Leaf) -replace 'issue-','')"
        $env:CARGO_TARGET_DIR = $target
        Push-Location $wt
        $ok = $false
        try {
            $ok = (cargo fmt --all --check) -and
                  (cargo clippy --workspace --all-targets -- -D warnings) -and
                  (cargo test --workspace)
        } finally { Pop-Location; Remove-Item Env:\CARGO_TARGET_DIR -ErrorAction SilentlyContinue }
        if (-not $ok) { Write-Output "PR ${n}: CHECKS FAILED"; continue }

        git -C $wt push --force-with-lease origin $branch
        gh pr merge $n --repo va1erian/visualrust --rebase --delete-branch 2>&1 | Out-Null
        git -C $repo worktree remove --force $wt
        Write-Output "PR ${n}: MERGED"
    }
} finally {
    Remove-Item $lock -Force -ErrorAction SilentlyContinue
}
