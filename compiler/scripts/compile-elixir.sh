#!/bin/bash
set -e

# Elixir compilation to escript

TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

mkdir -p "$TEMP_DIR/lib"
cp "$SOURCE_PATH" "$TEMP_DIR/lib/main.ex"
cd "$TEMP_DIR"

# Create mix.exs
cat > mix.exs << 'EOF'
defmodule Main.MixProject do
  use Mix.Project

  def project do
    [
      app: :main,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: false,
      escript: [main_module: Main]
    ]
  end

  def application do
    [extra_applications: [:logger]]
  end
end
EOF

# Get dependencies and compile
mix local.hex --force 2>/dev/null || true
mix deps.get 2>&1 || true

# Build escript
mix escript.build 2>&1

cp "$TEMP_DIR/main" "$OUTPUT_PATH"
