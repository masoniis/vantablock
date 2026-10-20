# INFO: -----------------------
#         configuration
# -----------------------------

set shell := ["bash", "-c"]
set windows-shell := ["powershell.exe", "-NoProfile", "-Command"]

os_family := os()
open_cmd := if os_family == "macos" { \
    "open" \
} else if os_family == "windows" { \
    "Start-Process" \
} else { \
    "xdg-open" \
}

# crate names
project := "vantablock"
client  := "vantablock-client"
server  := "vantablock-server"
app     := "vantablock-app"
baker   := "vantablock-baker"

# drops the default bevy dynamic linking
distribution_features := "--no-default-features --features client,distribution"

# lists all available recipes, in file order so niche groups stay at the bottom
[private]
default:
    @just --list --unsorted

# INFO: ------------------------
#         core execution
# ------------------------------

# runs the client via debug profile
[group("core")]
client *args:
    cargo run -p {{app}} --bin vantablock {{args}}

# runs the server via debug profile
[group("core")]
server *args:
    cargo run -p {{app}} --bin vantablock-server --no-default-features --features server,bevy/dynamic_linking {{args}}

# runs the client via max-optimization release profile
[group("core")]
release *args:
    cargo run -p {{app}} --bin vantablock --profile distribution {{distribution_features}} {{args}}

# runs the client with the bevy remote protocol (BRP) enabled for agent/tool control
[group("core")]
brp *args:
    cargo run -p {{app}} --bin vantablock --features brp {{args}}

# builds the BRP-enabled client so the bevy_brp_mcp server can launch it
[group("core")]
brp-build *args:
    cargo build -p {{app}} --bin vantablock --features brp {{args}}

alias run := client
alias run-fast := release

# INFO: -------------------------------
#         development & linting
# -------------------------------------

# runs cargo check across the workspace
[group("development")]
check *args:
    cargo check --all-targets {{args}}

# runs clippy across the workspace
[group("development")]
clippy *args:
    cargo clippy {{args}}

# fix compiler-fixable issues
[group("development")]
fix *args:
    cargo fix --all-targets --allow-dirty {{args}}

# runs nix fmt (rust, nix, wgsl/wesl, and prettier formatters)
[unix]
[group("development")]
fmt:
    nix fmt

# runs rustfmt (nix fmt, which also formats wgsl/wesl, nix, and markdown, needs unix or WSL)
[windows]
[group("development")]
fmt:
    cargo fmt --all
    @Write-Host "Formatted Rust only: WGSL/WESL, nix, and markdown need nix fmt (unix or WSL)."

# cleans build artifacts and local dev data
[unix]
[group("development")]
clean:
    cargo clean
    rm -rf target/
    rm -rf vantadev/

# cleans build artifacts and local dev data
[windows]
[group("development")]
clean:
    if (Test-Path target) { Remove-Item -Recurse -Force target }
    if (Test-Path vantadev) { Remove-Item -Recurse -Force vantadev }

alias lint := check
alias clip := clippy

# INFO: ------------------------------
#         testing & validation
# ------------------------------------

# runs all workspace tests
[group("testing")]
test *args:
    cargo nextest run {{args}}

# compiles benchmarks as tests to check for runtime errors
[group("testing")]
test-bench:
    cargo check --benches
    cargo test --benches

# runs the CPU traversal benchmarks on the real assets and opens the html report
[group("testing")]
bench *args:
    cargo bench -p {{baker}} --bench traversal {{args}}
    @echo "Opening Criterion report..."
    -{{ open_cmd }} target/criterion/report/index.html

# runs the shader validation and GPU conformance tests (GPU checks skip without an adapter)
[group("testing")]
test-gpu *args:
    cargo test -p {{client}} --test gpu_conformance -- --nocapture {{args}}

# reports the driver's register, spill, and shared-memory statistics for the voxel kernels (Vulkan)
[group("testing")]
shader-stats *args:
    cargo test --release -p {{client}} --test shader_statistics -- --ignored --nocapture {{args}}

# full pre-push verification suite
[group("testing")]
ready *args:
    cargo check --benches
    cargo clippy -- -D warnings
    cargo test {{args}}

# INFO: ------------------------
#         build & assets
# ------------------------------

# packages the client for distribution
[group("build")]
package profile="distribution":
    cargo build -p {{app}} --bin vantablock --profile {{profile}} {{distribution_features}}
    cargo packager -p {{app}} --bin vantablock --profile {{ if profile == "dev" { "debug" } else { profile } }}

# packages the server for distribution
[group("build")]
server-package profile="distribution":
    cargo build -p {{app}} --bin vantablock-server --no-default-features --features server --profile {{profile}}

# bakes asset recipes to .vanta files with 64-tree stats (all of assets/shared/vanta by default)
[group("build")]
bake *args:
    cargo run --release -p {{baker}} --bin vanta_baker -- --stats {{args}}

# strips macOS quarantine attributes from the app bundle
[macos]
[group("build")]
sign:
    xattr -cr /Applications/Vantablock.app

# INFO: -----------------------------
#         profiling & tracing
# -----------------------------------

# shows the ASM associated with a rust file (requires cargo-show-asm)
[group("profiling")]
asm path:
    cargo asm -p {{app}} --bin vantablock --color {{path}}

# launches tracy if it isn't already running, then runs the client with tracing
[unix]
[group("profiling")]
trace *args:
    #!/usr/bin/env bash
    if pgrep -x "tracy" > /dev/null; then
        echo -e "\033[1;32mTracy profiler is already running.\033[0m"
    else
        echo -e "\033[1;36mStarting Tracy profiler...\033[0m"
        TRACY_ENABLE_MEMORY=1
        tracy &
    fi
    cargo run -p {{app}} --bin vantablock --features tracy {{args}}

# launches tracy (tracy-profiler or tracy on PATH) if it isn't already running, then runs the client with tracing
[windows]
[group("profiling")]
[script("powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File")]
[extension(".ps1")]
trace *args:
    if (Get-Process tracy-profiler, tracy -ErrorAction SilentlyContinue) {
        Write-Host "Tracy profiler is already running." -ForegroundColor Green
    } else {
        $tracy = Get-Command tracy-profiler, tracy -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($tracy) {
            Write-Host "Starting Tracy profiler..." -ForegroundColor Cyan
            $env:TRACY_ENABLE_MEMORY = "1"
            Start-Process $tracy.Source
        } else {
            Write-Host "Tracy not found on PATH; start it manually to connect." -ForegroundColor Yellow
        }
    }
    cargo run -p {{app}} --bin vantablock --features tracy {{args}}
    exit $LASTEXITCODE

# runs the client with Bevy-specific debug features
[group("profiling")]
debug_bevy *args:
    cargo run -p {{app}} --bin vantablock --features bevy/trace,bevy/track_location,bevy/debug {{args}}

# runs the client with verbose wgpu logging
[unix]
[group("profiling")]
debug_wgpu *args:
    RUST_LOG=wgpu=trace cargo run -p {{app}} --bin vantablock {{args}}

# runs the client with verbose wgpu logging
[windows]
[group("profiling")]
debug_wgpu *args:
    $env:RUST_LOG = "wgpu=trace"; cargo run -p {{app}} --bin vantablock {{args}}

# targeted tracing, call without args to list targets found in source
[unix]
[group("profiling")]
debug *args:
    #!/usr/bin/env bash
    set -euo pipefail
    set -- {{args}}
    if [ "$#" -eq 0 ]; then
        echo -e "\033[1;33mNo debug targets specified. Available targets are:\033[0m"
        rg --no-heading -o --replace '$f:$1' 'target\s*:\s*"([^"]+)"' crates/ \
            | awk -F: '{print $NF}' | sort | uniq -c | sort -rn \
            | while read -r count target; do
                printf '  - \033[1;35m%s\033[0m (%sx)\n' "$target" "$count"
            done
        exit 0
    fi

    log_targets=""
    for target in "$@"; do
        log_targets="$log_targets$target=trace,"
    done
    export RUST_LOG="${log_targets%,},{{project}}=info"
    echo -e "\033[1;32mRunning with RUST_LOG=\033[0m$RUST_LOG"
    cargo run -p {{app}} --bin vantablock

# targeted tracing, call without args to list targets found in source
[windows]
[group("profiling")]
[script("powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File")]
[extension(".ps1")]
debug *args:
    $targets = @("{{args}}".Split(" ", [StringSplitOptions]::RemoveEmptyEntries))
    if ($targets.Count -eq 0) {
        Write-Host "No debug targets specified. Available targets are:" -ForegroundColor Yellow
        Get-ChildItem crates -Recurse -Filter *.rs |
            Select-String -Pattern 'target\s*:\s*"([^"]+)"' -AllMatches |
            ForEach-Object { $_.Matches } | ForEach-Object { $_.Groups[1].Value } |
            Group-Object | Sort-Object Count -Descending |
            ForEach-Object { Write-Host ("  - {0} ({1}x)" -f $_.Name, $_.Count) }
        exit 0
    }
    $env:RUST_LOG = (($targets | ForEach-Object { "$_=trace" }) -join ",") + ",{{project}}=info"
    Write-Host "Running with RUST_LOG=$env:RUST_LOG" -ForegroundColor Green
    cargo run -p {{app}} --bin vantablock
    exit $LASTEXITCODE

# INFO: ---------------------------------
#         advanced/niche commands
# ---------------------------------------

# target wsl -> windows builds to happen in the windows temp dir
windows_temp := if os_family == "windows" { "" \
} else { \
    `if [ -d /mnt/c ]; then cd /mnt/c && cmd.exe /c "echo %TEMP%" 2>/dev/null | tr -d '\r\n'; fi` \
}
windows_wsl_target_dir := windows_temp + "\\vantablock-target"

# requires cargo, just, and VS build tools on Windows host:
# - winget install just
# - winget install rustup
# - winget install --id Microsoft.VisualStudio.2022.BuildTools --override "--passive --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"

# compiles and runs the client natively on Windows from within WSL
[linux]
[group("advanced")]
wsl *args:
    @echo "Compiling Windows target ({{windows_wsl_target_dir}})"
    @WSL_PATH=$(wslpath -w .)
    @powershell.exe -Command "if (!(Test-Path '{{windows_wsl_target_dir}}')) { New-Item -ItemType Directory -Force -Path '{{windows_wsl_target_dir}}' }; cd '$WSL_PATH'; \$env:CARGO_TARGET_DIR='{{windows_wsl_target_dir}}'; just {{args}}"
