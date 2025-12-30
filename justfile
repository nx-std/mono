# Display available commands (default target)
default:
    @just --list


## Workspace configuration

# Build directory (can be overridden with just builddir=<path> <task>)
builddir := "buildDir"

# Target platform for Rust builds
target := "aarch64-nintendo-switch-freestanding"


## Code formatting

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


## Check

alias check := check-rs

# Check Rust code (cargo check)
[group: 'check']
check-rs *EXTRA_FLAGS:
    cargo check --target {{target}} {{EXTRA_FLAGS}}

# Check specific crate (cargo check -p <crate>)
[group: 'check']
check-crate CRATE *EXTRA_FLAGS:
    cargo check --target {{target}} --package {{CRATE}} {{EXTRA_FLAGS}}

# Lint Rust code (cargo clippy)
[group: 'check']
clippy *EXTRA_FLAGS:
    cargo clippy --target {{target}} {{EXTRA_FLAGS}}

# Lint specific crate (cargo clippy -p <crate> --no-deps)
[group: 'check']
clippy-crate CRATE *EXTRA_FLAGS:
    cargo clippy --target {{target}} --package {{CRATE}} --no-deps {{EXTRA_FLAGS}}


## Build (Meson)

alias setup := meson-setup
alias compile := meson-compile
alias build := meson-compile

# Setup meson build directory (meson setup)
[group: 'build']
meson-setup *EXTRA_FLAGS:
    meson setup --cross-file devkitpro.txt --cross-file cross.txt {{builddir}} {{EXTRA_FLAGS}}

# Compile the project (meson compile)
[group: 'build']
meson-compile *TARGETS:
    meson compile -C {{builddir}} {{TARGETS}}

# Setup meson build with test configuration options
[group: 'build']
setup-tests *EXTRA_FLAGS:
    meson setup --cross-file devkitpro.txt --cross-file cross.txt {{builddir}} {{EXTRA_FLAGS}}

# Build the nx-tests NRO (Switch homebrew test executable)
[group: 'build']
build-tests:
    meson compile -C {{builddir}} nx-tests.nro


## Clean

alias clean := meson-clean

# Clean the meson build directory (meson compile --clean)
[group: 'clean']
meson-clean:
    meson compile -C {{builddir}} --clean

# Clean cargo workspace (cargo clean)
[group: 'clean']
cargo-clean:
    cargo clean

# Remove the build directory entirely
[group: 'clean']
clean-all:
    @rm -rf {{builddir}}


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
