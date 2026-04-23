# platform-specific configuration
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

# where to place wsl windows builds
wsl_target := "C:/temp/vantablock-build"

default: run

# INFO: -------------------------
#          core execution
# -------------------------------

# runs the client via debug profile
run *args:
    cargo run -p {{runner}} --bin game {{args}}

# runs the server via debug profile
server *args:
    cargo run -p {{runner}} --bin server --no-default-features --features dedicated {{args}}

# runs the client via max-optimization release profile
release *args:
    cargo run -p {{runner}} --bin game --profile distribution --features distribution {{args}}

# compiles and runs the client natively on Windows from within WSL
# requires cargo and just on Windows host:
# - winget install just
# - winget install rustup
wsl *args:
    @WSL_PATH=$(wslpath -w .)
    @powershell.exe -Command "if (!(Test-Path '{{wsl_target}}')) { New-Item -ItemType Directory -Force -Path '{{wsl_target}}' }; cd '$WSL_PATH'; \$env:CARGO_TARGET_DIR='{{wsl_target}}'; cargo run -p {{runner}} --bin game {{args}}"

alias run-fast := release

# INFO: -------------------------------
#         development & linting
# -------------------------------------

# runs cargo check across the workspace
check *args:
    cargo check {{args}}
    cargo check --benches

# runs clippy across the workspace
clippy *args:
    cargo clippy {{args}}

# fix compiler-fixable issues
fix:
    cargo fix --allow-dirty

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
    cargo test {{args}}

# compiles benchmarks as tests to check for runtime errors
test-bench:
    cargo check --benches
    cargo test --benches

# runs benchmarks and opens html report once finished
bench *args:
    cargo bench -p {{client}} {{args}}
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
    cargo build -p {{runner}} --bin game --profile {{profile}} --features distribution
    cargo packager -p {{runner}} --bin game --profile {{ if profile == "dev" { "debug" } else { profile } }}

# packages the server for distribution
server-package profile="distribution":
    cargo build -p {{runner}} --bin server --no-default-features --features dedicated --profile {{profile}}

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
    cargo asm -p {{runner}} --bin game --color {{path}}

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
    cargo run -p {{runner}} --bin game --features tracy {{args}}

# runs the client with Bevy-specific debug features
debug_bevy *args:
    cargo run -p {{runner}} --bin game --features bevy/trace,bevy/track_location,bevy/debug {{args}}

# runs the client with verbose wgpu logging
debug_wgpu *args:
    RUST_LOG=wgpu=trace cargo run -p {{runner}} --bin game {{args}}

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
    cargo run -p {{runner}} --bin game
