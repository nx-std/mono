# Display available commands (default target)
default:
    @just --list


## Workspace configuration

# Build directory (can be overridden with just builddir=<path> <task>)
builddir := "buildDir"

# Cargo target directory (can be overridden with just cargo_target_dir=<path> <task>)
cargo_target_dir := builddir / "cargo-target"

# Target platform for Rust builds
target := "aarch64-nintendo-switch-freestanding"


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


## Build (Meson)

alias configure := meson-configure
alias compile := meson-compile
alias build := meson-compile

# Configure meson build directory (meson setup)
[group: 'build']
meson-configure *EXTRA_FLAGS:
    meson setup --cross-file devkitpro.txt --cross-file cross.txt {{builddir}} {{EXTRA_FLAGS}}

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

# Build the nx-tests NRO (Switch homebrew test executable)
[group: 'build']
build-tests: _ensure-configured
    meson compile -C {{builddir}} nx-tests.nro

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

# Install cargo-nx from the submodule (override workspace config to build for host)
[group: 'deploy']
install-cargo-nx:
    #!/usr/bin/env bash
    host_target=$(rustc +stable -vV | sed -n 's|host: ||p')
    cargo +stable install --path subprojects/cargo-nx --target "$host_target"

# Deploy an NRO file to the Nintendo Switch via cargo nx link
[group: 'deploy']
deploy NRO_FILE *EXTRA_FLAGS:
    cargo nx link {{NRO_FILE}} {{EXTRA_FLAGS}}


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
