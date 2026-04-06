default: client

# INFO: -----------------------------
#         basic cargo aliases
# -----------------------------------

# runs the client via debug profile
run *args:
	cargo run -p client {{args}}

# runs the client via debug profile
client *args:
	cargo run -p client {{args}}

# runs the server via debug profile
server *args:
	cargo run -p server {{args}}

# runs the client via max-optimization release profile
alias run-fast := release
release *args:
	cargo run -p client --profile distribution --features client/final_release {{args}}

# runs benchmarks and opens html report once finished
bench *args:
	cargo bench -p client {{args}}
	echo "Opening Criterion report..."
	open target/criterion/report/index.html

check *args:
	cargo check {{args}}

clippy *args:
	cargo clippy {{args}}

clean *args:
	cargo clean {{args}}

fix *args:
	cargo fix --allow-dirty

texture:
	cargo run -p client --bin texture_processor

package:
	cargo packager -p client --profile distribution

# INFO: ---------------------
#         small utils
# ---------------------------

fmt:
	nix fmt

ready *args:
	nix fmt
	cargo clippy -- -D warnings
	cargo test {{args}}

sign:
	xattr -cr /Applications/Vantablock.app

# INFO: ---------------------------
#         advanced commands
# ---------------------------------

# Shows the ASM associated with a rust file.
# requires https://crates.io/crates/cargo-show-asm
asm path:
    cargo asm -p client --color {{path}}

trace *args:
	#!/usr/bin/env bash

	# launch tracy if it isn't already running
	if pgrep -x "tracy" > /dev/null; then
			echo -e "\033[1;32mTracy profiler is already running.\033[0m"
	else
			echo -e "\033[1;36mStarting Tracy profiler...\033[0m"
			TRACY_ENABLE_MEMORY=1
			tracy &
	fi

	cargo run -p client --features client/tracy {{args}}

debug_bevy *args:
  cargo run -p client --features bevy/trace,bevy/track_location,bevy/debug {{args}}

debug_wgpu *args:
	RUST_LOG=wgpu=trace cargo run -p client {{args}}

debug *args:
	#!/usr/bin/env bash
	set -euo pipefail
	set -- {{args}}
	if [ "$#" -eq 0 ]; then
		echo -e "\033[1;33mNo debug targets specified. Available targets are:\033[0m"
		rg --no-heading -o --replace '$f:$1' 'target\s*:\s*"([^"]+)"' crates/client/src/ \
			| awk -F: '{print $NF}' \
			| sort \
			| uniq -c \
			| sort -rn \
			| while read -r count target; do
					printf '  - \033[1;35m%s\033[0m (%sx)\n' "$target" "$count"
				done
		exit 0
	fi

	# Add targets to the rust log env variable
	log_targets=""
	for target in "$@"; do
	        log_targets="$log_targets$target=trace,"
	done
	export RUST_LOG="${log_targets%,},vantablock=info"

	echo -e "\033[1;32mRunning with RUST_LOG=\033[0m$RUST_LOG"
	cargo run -p client
