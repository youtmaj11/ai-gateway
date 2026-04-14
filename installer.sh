#!/usr/bin/env bash
set -euo pipefail

PROG="$(basename "$0")"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
CONFIG_DIR="$REPO_ROOT/config"
CONFIG_FILE_ROOT="$REPO_ROOT/config.toml"
CONFIG_FILE_DIR="$CONFIG_DIR/config.toml"
EXAMPLE_FILE="$REPO_ROOT/config.toml.example"
INSTALL_DIR="$HOME/.local/bin"
OLLAMA_BINARY="ollama"

print_help() {
    cat <<EOF
Usage: $PROG [--help]

Idempotent installer for ai-gateway local mode.

Options:
  -h, --help     Show this help message and exit.

What this installer does:
  - Ensures Ollama is installed or installs it if missing.
  - Creates a config directory under ./config.
  - Creates a default config.toml from config.toml.example.
  - Leaves existing configuration intact.
EOF
}

info() {
    printf "[ai-gateway] %s\n" "$1"
}

warn() {
    printf "[ai-gateway] WARNING: %s\n" "$1"
}

error() {
    printf "[ai-gateway] ERROR: %s\n" "$1" >&2
    exit 1
}

require_command() {
    if ! command -v "$1" >/dev/null 2>&1; then
        error "Required command '$1' is not available. Please install it and retry."
    fi
}

detect_ollama_asset() {
    local os arch asset
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)
            case "$arch" in
                x86_64) asset="ollama-linux-amd64.tar.gz" ;;
                aarch64|arm64) asset="ollama-linux-arm64.tar.gz" ;;
                *) error "Unsupported architecture: $arch" ;;
            esac
            ;;
        Darwin)
            case "$arch" in
                x86_64) asset="ollama-macos-amd64.tar.gz" ;;
                arm64) asset="ollama-macos-arm64.tar.gz" ;;
                *) error "Unsupported architecture: $arch" ;;
            esac
            ;;
        *) error "Unsupported OS: $os" ;;
    esac

    printf '%s' "$asset"
}

install_ollama() {
    if command -v "$OLLAMA_BINARY" >/dev/null 2>&1; then
        info "Ollama already installed at $(command -v "$OLLAMA_BINARY")"
        return 0
    fi

    info "Ollama not found; installing Ollama..."

    local asset url tmpdir tarball target_dir
    asset="$(detect_ollama_asset)"
    url="https://github.com/ollama/ollama/releases/latest/download/$asset"
    tmpdir="$(mktemp -d)"
    tarball="$tmpdir/$asset"

    if command -v curl >/dev/null 2>&1; then
        info "Downloading Ollama from $url"
        curl -fsSL -o "$tarball" "$url"
    elif command -v wget >/dev/null 2>&1; then
        info "Downloading Ollama from $url"
        wget -qO "$tarball" "$url"
    else
        error "Neither curl nor wget are available for downloading Ollama."
    fi

    info "Extracting Ollama package"
    tar -xzf "$tarball" -C "$tmpdir"

    if [ ! -f "$tmpdir/ollama" ]; then
        error "Downloaded archive did not contain the Ollama binary."
    fi

    if [ -w "/usr/local/bin" ] 2>/dev/null; then
        target_dir="/usr/local/bin"
    else
        target_dir="$INSTALL_DIR"
        mkdir -p "$target_dir"
    fi

    install -m 0755 "$tmpdir/ollama" "$target_dir/ollama"
    info "Installed Ollama to $target_dir/ollama"

    if ! command -v "$OLLAMA_BINARY" >/dev/null 2>&1; then
        warn "$target_dir is not in your PATH. Add it with:\n  export PATH=\"$target_dir:\$PATH\""
    fi
}

setup_config() {
    info "Ensuring configuration directory exists at $CONFIG_DIR"
    mkdir -p "$CONFIG_DIR"

    if [ -f "$CONFIG_FILE_ROOT" ]; then
        info "Root config.toml already exists."
    elif [ -f "$CONFIG_FILE_DIR" ]; then
        info "Found config/config.toml, copying to root config.toml"
        cp "$CONFIG_FILE_DIR" "$CONFIG_FILE_ROOT"
    elif [ -f "$EXAMPLE_FILE" ]; then
        info "Creating default configuration from example."
        cp "$EXAMPLE_FILE" "$CONFIG_FILE_DIR"
        cp "$EXAMPLE_FILE" "$CONFIG_FILE_ROOT"
    else
        error "Missing example configuration file: $EXAMPLE_FILE"
    fi

    if [ -f "$CONFIG_FILE_DIR" ]; then
        info "Config file already exists at $CONFIG_FILE_DIR"
    elif [ -f "$CONFIG_FILE_ROOT" ]; then
        info "Copying root config.toml into $CONFIG_FILE_DIR"
        cp "$CONFIG_FILE_ROOT" "$CONFIG_FILE_DIR"
    fi

    info "Configuration setup complete."
}

main() {
    if [ "${1:-}" = "-h" ] || [ "${1:-}" = "--help" ]; then
        print_help
        exit 0
    fi

    info "Starting ai-gateway installer"
    setup_config
    install_ollama
    info "Installer finished successfully."
    info "Next steps: edit config.toml and run 'cargo build' or start the gateway."
}

main "$@"
