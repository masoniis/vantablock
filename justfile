# INFO: -----------------------
#         configuration
# -----------------------------

set shell := ["bash", "-c"]
set windows-shell := ["powershell.exe", "-NoProfile", "-Command"]

os_family := os()
open_cmd := if os_family == "macos" { \
    "open" \
} else if os_family == "windows" { \
    "explorer.exe" \
} else { \
    "xdg-open" \
}

# crate names
project := "vantablock"
client  := "vantablock-client"
server  := "vantablock-server"
runner  := "vantablock-runner"

# INFO: ------------------------
#         core execution
# ------------------------------

default: client

# runs the client via debug profile
client *args:
    cargo run -p {{runner}} --bin vantablock {{args}}

# runs the server via debug profile
server *args:
    cargo run -p {{runner}} --bin vantablock-server --no-default-features --features server {{args}}

# runs the client via max-optimization release profile
release *args:
    cargo run -p {{runner}} --bin vantablock --profile distribution --features distribution {{args}}


alias run := client
alias run-fast := release

# INFO: -------------------------------
#         development & linting
# -------------------------------------

# runs cargo check across the workspace
check *args:
    cargo check --all-targets {{args}}

# runs clippy across the workspace
clippy *args:
    cargo clippy {{args}}

# fix compiler-fixable issues
fix *args:
    cargo fix --all-targets --allow-dirty {{args}}

# runs nix fmt (rustfmt + nixpkgs-fmt)
fmt:
    nix fmt

# cleans ephemeral dirs
clean:
	rm -rf target/
	rm -rf .dev_data/

alias lint := check
alias clip := clippy

# INFO: ------------------------------
#         testing & validation
# ------------------------------------

# runs all workspace tests
test *args:
    cargo nextest run {{args}}

# compiles benchmarks as tests to check for runtime errors
test-bench:
    cargo check --benches
    cargo test --benches

# runs benchmarks and opens html report once finished
bench-client *args:
    cargo bench -p {{client}} {{args}}
    @echo "Opening Criterion report..."
    -{{ open_cmd }} target/criterion/report/index.html

bench-server *args:
    cargo bench -p {{server}} {{args}}
    @echo "Opening Criterion report..."
    -{{ open_cmd }} target/criterion/report/index.html

# full pre-push verification suite
ready *args:
    cargo check --benches
    cargo clippy -- -D warnings
    cargo test {{args}}

# INFO: ------------------------
#         build & assets
# ------------------------------

# packages the client for distribution
package profile="distribution":
    cargo build -p {{runner}} --bin vantablock --profile {{profile}} --features distribution
    cargo packager -p {{runner}} --bin vantablock --profile {{ if profile == "dev" { "debug" } else { profile } }}

# packages the server for distribution
server-package profile="distribution":
    cargo build -p {{runner}} --bin vantablock-server --no-default-features --features server --profile {{profile}}

# runs the texture processor utility
texture:
    cargo run -p {{client}} --bin texture_processor

# strips macOS quarantine attributes from the app bundle
sign:
    xattr -cr /Applications/Vantablock.app

# INFO: -----------------------------
#         profiling & tracing
# -----------------------------------

# Shows the ASM associated with a rust file.
# requires https://crates.io/crates/cargo-show-asm
asm path:
    cargo asm -p {{runner}} --bin vantablock --color {{path}}

# launch tracy if it isn't already running
trace *args:
    #!/usr/bin/env bash
    if pgrep -x "tracy" > /dev/null; then
        echo -e "\033[1;32mTracy profiler is already running.\033[0m"
    else
        echo -e "\033[1;36mStarting Tracy profiler...\033[0m"
        TRACY_ENABLE_MEMORY=1
        tracy &
    fi
    cargo run -p {{runner}} --bin vantablock --features tracy {{args}}

# runs the client with Bevy-specific debug features
debug_bevy *args:
    cargo run -p {{runner}} --bin vantablock --features bevy/trace,bevy/track_location,bevy/debug {{args}}

# runs the client with verbose wgpu logging
debug_wgpu *args:
    RUST_LOG=wgpu=trace cargo run -p {{runner}} --bin vantablock {{args}}

# targeted tracing, call without args to list targets found in source.
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
    cargo run -p {{runner}} --bin vantablock

# INFO: ---------------------------------
#         advanced/niche commands
# ---------------------------------------

# target wsl -> windows builds to happen in the windows temp dir
windows_temp := if os_family == "windows" { "" \
} else { \
    `if [ -d /mnt/c ]; then cd /mnt/c && cmd.exe /c "echo %TEMP%" 2>/dev/null | tr -d '\r\n'; fi` \
}
windows_wsl_target_dir := windows_temp + "\\vantablock-target"

# compiles and runs the client natively on Windows from within WSL
# requires cargo, just, and VS build tools on Windows host:
# - winget install just
# - winget install rustup
# - winget install --id Microsoft.VisualStudio.2022.BuildTools --override "--passive --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
wsl *args:
    @echo "Compiling Windows target ({{windows_wsl_target_dir}})"
    @WSL_PATH=$(wslpath -w .)
    @powershell.exe -Command "if (!(Test-Path '{{windows_wsl_target_dir}}')) { New-Item -ItemType Directory -Force -Path '{{windows_wsl_target_dir}}' }; cd '$WSL_PATH'; \$env:CARGO_TARGET_DIR='{{windows_wsl_target_dir}}'; just {{args}}"
