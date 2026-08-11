# Display available commands (default target)
default:
    @just --list


## Workspace configuration

# Build directory (can be overridden with just builddir=<path> <task>)
builddir := "buildDir"

# Cargo target directory (can be overridden with just cargo_target_dir=<path> <task>)
cargo_target_dir := builddir / "cargo-target"

# Target platform for Rust builds (repo-local spec; see .cargo/config.toml)
target := "aarch64-nintendo-horizon.json"


## Format

alias fmt := fmt-rs
alias fmt-check := fmt-rs-check

# Format Rust code (cargo fmt --all)
[group: 'format']
fmt-rs:
    cargo +nightly fmt --all

# Check Rust code format (cargo fmt --check)
[group: 'format']
fmt-rs-check:
    cargo +nightly fmt --all -- --check

# Format all meson files
[group: 'format']
fmt-meson:
    meson format --inplace --recursive .

# Check meson file formatting
[group: 'format']
fmt-meson-check:
    meson format --check-only --recursive .


## Submodules

# Initialize git submodules
[group: 'submodules']
submodule-init:
    git submodule update --init --recursive

# Deinitialize git submodules
[group: 'submodules']
submodule-deinit:
    git submodule deinit --all

# Update git submodules recursively with force
[group: 'submodules']
submodule-update:
    git submodule update --init --recursive --force


## Check

alias check := check-rs

# Check Rust code (cargo check)
[group: 'check']
check-rs *EXTRA_FLAGS:
    cargo check --target {{target}} --target-dir {{cargo_target_dir}} {{EXTRA_FLAGS}}

# Check specific crate (cargo check -p <crate>)
[group: 'check']
check-crate CRATE *EXTRA_FLAGS:
    cargo check --target {{target}} --target-dir {{cargo_target_dir}} --package {{CRATE}} {{EXTRA_FLAGS}}

# Lint Rust code (cargo clippy)
[group: 'check']
clippy *EXTRA_FLAGS:
    cargo clippy --target {{target}} --target-dir {{cargo_target_dir}} {{EXTRA_FLAGS}}

# Lint specific crate (cargo clippy -p <crate> --no-deps)
[group: 'check']
clippy-crate CRATE *EXTRA_FLAGS:
    cargo clippy --target {{target}} --target-dir {{cargo_target_dir}} --package {{CRATE}} --no-deps {{EXTRA_FLAGS}}

# Check the meson build definition (statically interprets every meson.build/meson.options)
[group: 'check']
check-meson:
    #!/usr/bin/env bash
    set -euo pipefail

    # `meson introspect` on the source tree runs meson's static interpreter over
    # the whole project (root + every subproject), so it catches syntax errors,
    # unknown functions and bad kwargs without needing the devkitPro toolchain.
    # The JSON payload is noise here; the diagnostics go to stderr.
    meson introspect --projectinfo meson.build > /dev/null
    meson introspect --buildoptions meson.build > /dev/null

alias check-deps := check-unused-deps

# Check for unused Rust dependencies (cargo machete)
[group: 'check']
check-unused-deps:
    cargo machete

# Verify the rustc-pipeline target JSON's link-script matches switch.ld
[group: 'check']
check-target-json:
    #!/usr/bin/env bash
    set -euo pipefail

    linker_script='subprojects/libnx/src/nx/switch.ld'
    target_json='aarch64-nintendo-horizon.json'

    command -v jq >/dev/null || { >&2 echo "error: 'jq' not found on PATH"; exit 1; }
    [[ -f "$linker_script" ]] || { >&2 echo "error: $linker_script not found (is the libnx submodule checked out?)"; exit 1; }
    [[ -f "$target_json" ]] || { >&2 echo "error: $target_json not found"; exit 1; }

    # switch.ld is the single source of truth for the section layout. The rustc
    # link pipeline has no implicit `-T` step, so that layout must ride verbatim
    # in the target JSON's `link-script` field. `--rawfile` reads switch.ld as a
    # single string; jq handles the JSON escaping.
    generated="$(jq --indent 2 --rawfile ls "$linker_script" '.["link-script"] = $ls' "$target_json")"

    if [[ "$(cat "$target_json")" == "$generated" ]]; then
        echo "target JSON's link-script is in sync with switch.ld"
        exit 0
    fi

    cat >&2 <<'MSG'
    error: aarch64-nintendo-horizon.json is out of sync with switch.ld

      The rustc-pipeline target JSON embeds switch.ld's section layout verbatim
      in its `link-script` field, because the rustc link has no implicit `-T`
      step. switch.ld has changed since that field was last embedded.

      switch.ld is a vendored libnx file, so this should be very rare. To
      resync, re-embed switch.ld into the JSON and commit both files together:

        jq --indent 2 --rawfile ls subprojects/libnx/src/nx/switch.ld \
          '.["link-script"] = $ls' aarch64-nintendo-horizon.json \
          > aarch64-nintendo-horizon.json.tmp
        mv aarch64-nintendo-horizon.json.tmp aarch64-nintendo-horizon.json

      Then review the diff and rebuild the rustc pipeline before committing.
    MSG
    exit 1


## Build (Meson)

alias configure := meson-configure
alias reconfigure := meson-reconfigure
alias compile := meson-compile
alias build := meson-compile

# Configure meson build directory (meson setup)
[group: 'build']
meson-configure *EXTRA_FLAGS:
    meson setup --cross-file devkitpro.txt --cross-file cross.txt --cross-file cargo-nx.txt {{builddir}} {{EXTRA_FLAGS}}

# Reconfigure meson build directory (meson setup --reconfigure)
[group: 'build']
meson-reconfigure *EXTRA_FLAGS:
    meson setup --cross-file devkitpro.txt --cross-file cross.txt --cross-file cargo-nx.txt {{builddir}} {{EXTRA_FLAGS}} --reconfigure

# Ensure build directory is configured (idempotent)
[group: 'build']
[private]
_ensure-configured:
    #!/usr/bin/env bash
    if [ ! -f "{{builddir}}/meson-private/coredata.dat" ]; then
        echo "Build directory not configured. Running configure..."
        just configure
    fi

# Compile the project (meson compile)
[group: 'build']
meson-compile *TARGETS: _ensure-configured
    meson compile -C {{builddir}} {{TARGETS}}

# Build the nx-tests loader NRO and every test suite NRO
[group: 'build']
build-tests: _ensure-configured
    meson compile -C {{builddir}} nx-tests.nro nx-tests-rand.nro nx-tests-rt.nro nx-tests-thread.nro nx-tests-sync.nro nx-tests-fs.nro nx-tests-net.nro nx-tests-applet-album.nro nx-tests-applet-err.nro

# List all build targets (meson introspect --targets)
[group: 'build']
list-targets: _ensure-configured
    meson introspect {{builddir}} --targets

# List all dependencies (meson introspect --dependencies)
[group: 'build']
list-dependencies: _ensure-configured
    meson introspect {{builddir}} --dependencies

# List all project options (from meson.options)
[group: 'build']
list-options:
    @meson introspect --buildoptions meson.build 2>/dev/null | jq -r '.[] | select(.name | startswith("use_")) | "\(.name) (\(.value)): \(.description)"'

# List configured project options (requires configured build)
[group: 'build']
list-options-configured: _ensure-configured
    @meson configure {{builddir}} | grep "use_" | sort -u


## Deploy

# Deploy an NRO file to the Nintendo Switch via cargo nx link
[group: 'deploy']
deploy NRO_FILE *EXTRA_FLAGS:
    cargo nx link {{NRO_FILE}} {{EXTRA_FLAGS}}


## devkitPro toolchain (aarch64-none-elf-*)
##
## Thin passthroughs for inspecting Switch homebrew artifacts (NRO/ELF) and
## connecting to Atmosphère's dmnt.gen2 GDB stub. Override the prefix path with
## `just dkp_prefix=/some/path/aarch64-none-elf <task>` if devkitPro lives
## elsewhere.

dkp_prefix := "/opt/devkitpro/devkitA64/bin/aarch64-none-elf"

# GDB for aarch64-none-elf (use against Atmosphère dmnt.gen2 on TCP 22225)
[group: 'devkitpro']
gdb *ARGS:
    {{dkp_prefix}}-gdb {{ARGS}}

# addr2line: resolve runtime addresses to source locations in an ELF
[group: 'devkitpro']
addr2line *ARGS:
    {{dkp_prefix}}-addr2line {{ARGS}}

# objdump: disassemble / dump section info from an ELF
[group: 'devkitpro']
objdump *ARGS:
    {{dkp_prefix}}-objdump {{ARGS}}

# nm: list symbols (sorted by address with -n) in an ELF
[group: 'devkitpro']
nm *ARGS:
    {{dkp_prefix}}-nm {{ARGS}}

# readelf: ELF header / section / segment / dynamic info
[group: 'devkitpro']
readelf *ARGS:
    {{dkp_prefix}}-readelf {{ARGS}}

# c++filt: demangle C++/Itanium symbols (Rust v0 names already readable)
[group: 'devkitpro']
cxxfilt *ARGS:
    {{dkp_prefix}}-c++filt {{ARGS}}

# strip: strip symbols from an ELF/NRO
[group: 'devkitpro']
strip *ARGS:
    {{dkp_prefix}}-strip {{ARGS}}

# size: section sizes summary for an ELF
[group: 'devkitpro']
size *ARGS:
    {{dkp_prefix}}-size {{ARGS}}


## Clean

# Clean both meson build directory and cargo workspace
[group: 'clean']
clean: meson-clean cargo-clean

# Clean the meson build directory (meson compile --clean)
[group: 'clean']
meson-clean:
    meson compile -C {{builddir}} --clean

# Clean cargo workspace (cargo clean)
[group: 'clean']
cargo-clean:
    cargo clean --target-dir {{cargo_target_dir}}

# Remove the build directory entirely
[group: 'clean']
clean-all:
    @rm -rf {{cargo_target_dir}} {{builddir}}


## Misc

PRECOMMIT_CONFIG := ".github/pre-commit-config.yaml"
PRECOMMIT_DEFAULT_HOOKS := "pre-commit pre-push"

# Install Git hooks
[group: 'misc']
install-git-hooks HOOKS=PRECOMMIT_DEFAULT_HOOKS:
    #!/usr/bin/env bash
    set -e # Exit on error

    # Check if pre-commit is installed
    if ! command -v "pre-commit" &> /dev/null; then
        >&2 echo "=============================================================="
        >&2 echo "Required command 'pre-commit' not available ❌"
        >&2 echo ""
        >&2 echo "Please install pre-commit using your preferred package manager"
        >&2 echo "  pip install pre-commit"
        >&2 echo "  pacman -S pre-commit"
        >&2 echo "  apt-get install pre-commit"
        >&2 echo "  brew install pre-commit"
        >&2 echo "=============================================================="
        exit 1
    fi

    # Install all Git hooks (see PRECOMMIT_DEFAULT_HOOKS for default hooks)
    pre-commit install --config {{PRECOMMIT_CONFIG}} {{replace_regex(HOOKS, "\\s*([a-z-]+)\\s*", "--hook-type $1 ")}}

# Remove Git hooks
[group: 'misc']
remove-git-hooks HOOKS=PRECOMMIT_DEFAULT_HOOKS:
    #!/usr/bin/env bash
    set -e # Exit on error

    # Check if pre-commit is installed
    if ! command -v "pre-commit" &> /dev/null; then
        >&2 echo "=============================================================="
        >&2 echo "Required command 'pre-commit' not available ❌"
        >&2 echo ""
        >&2 echo "Please install pre-commit using your preferred package manager"
        >&2 echo "  pip install pre-commit"
        >&2 echo "  pacman -S pre-commit"
        >&2 echo "  apt-get install pre-commit"
        >&2 echo "  brew install pre-commit"
        >&2 echo "=============================================================="
        exit 1
    fi

    # Remove all Git hooks (see PRECOMMIT_DEFAULT_HOOKS for default hooks)
    pre-commit uninstall --config {{PRECOMMIT_CONFIG}} {{replace_regex(HOOKS, "\\s*([a-z-]+)\\s*", "--hook-type $1 ")}}

# Install cargo-machete (unused dependency checker)
[group: 'misc']
install-cargo-machete:
    cargo install --locked cargo-machete

# Pinned cargo-nx revision from nx-std/tools (main @ 2026-05-19)
CARGO_NX_REV := "905708c97b1827cee93f455518b029e3d9296419"

# Install cargo-nx; pass --latest to install from the tip of main instead of the pinned revision.
[group: 'misc']
[arg("latest", long="latest", value="true")]
install-cargo-nx latest="false":
    cargo +stable install \
        --git https://github.com/nx-std/tools.git \
        {{ if latest == "true" { "--branch main" } else { "--rev " + CARGO_NX_REV } }} \
        --locked \
        --target host-tuple \
        cargo-nx
