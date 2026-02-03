#!/bin/bash
set -e

# C# compilation with .NET AOT (Native AOT)

TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Create minimal project
cat > "$TEMP_DIR/solution.csproj" << 'EOF'
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net8.0</TargetFramework>
    <ImplicitUsings>enable</ImplicitUsings>
    <PublishAot>true</PublishAot>
    <StripSymbols>true</StripSymbols>
    <InvariantGlobalization>true</InvariantGlobalization>
    <IlcOptimizationPreference>Size</IlcOptimizationPreference>
    <SelfContained>true</SelfContained>
  </PropertyGroup>
</Project>
EOF

cp "$SOURCE_PATH" "$TEMP_DIR/Program.cs"
cd "$TEMP_DIR"

dotnet publish -c Release -r linux-x64 -o out 2>&1

cp "$TEMP_DIR/out/solution" "$OUTPUT_PATH"
