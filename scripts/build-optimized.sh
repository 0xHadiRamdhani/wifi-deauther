#!/bin/bash

# Wi-Fi Deauther Optimized Build Script
# This script builds the application with maximum size optimization for all platforms

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
PROJECT_NAME="wifi-deauther"
TARGET_SIZE_MB=3
TARGET_SIZE_BYTES=$((TARGET_SIZE_MB * 1024 * 1024))

echo -e "${GREEN}=== Wi-Fi Deauther Optimized Build Script ===${NC}"
echo -e "${YELLOW}Target binary size: <${TARGET_SIZE_MB}MB${NC}"
echo ""

# Function to build for a specific target
build_target() {
    local target=$1
    local output_name=$2
    
    echo -e "${YELLOW}Building for ${target}...${NC}"
    
    # Clean previous build
    cargo clean
    
    # Build with optimizations
    if [[ "$target" == *"musl"* ]]; then
        # For musl targets, use vendored OpenSSL if needed
        RUSTFLAGS="-C target-feature=+crt-static -C opt-level=z -C lto=fat -C codegen-units=1" \
        cargo build --release --target "$target" --features vendored
    else
        RUSTFLAGS="-C opt-level=z -C lto=fat -C codegen-units=1" \
        cargo build --release --target "$target"
    fi
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ Build successful for ${target}${NC}"
        
        # Get binary path
        local binary_path="target/${target}/release/${PROJECT_NAME}"
        if [[ "$target" == *"windows"* ]]; then
            binary_path="${binary_path}.exe"
        fi
        
        # Check if binary exists
        if [ -f "$binary_path" ]; then
            # Strip symbols
            echo -e "${YELLOW}Stripping symbols...${NC}"
            if [[ "$target" == *"windows"* ]]; then
                x86_64-w64-mingw32-strip "$binary_path" 2>/dev/null || true
            else
                strip "$binary_path" 2>/dev/null || true
            fi
            
            # Get final size
            local size_bytes=$(stat -c%s "$binary_path" 2>/dev/null || stat -f%z "$binary_path" 2>/dev/null || echo "0")
            local size_mb=$(echo "scale=2; $size_bytes / 1024 / 1024" | bc -l 2>/dev/null || echo "0")
            
            echo -e "${YELLOW}Binary size: ${size_mb}MB (${size_bytes} bytes)${NC}"
            
            # Check if size target is met
            if [ "$size_bytes" -lt "$TARGET_SIZE_BYTES" ]; then
                echo -e "${GREEN}✓ Size target met!${NC}"
            else
                echo -e "${RED}✗ Size target exceeded by $((size_bytes - TARGET_SIZE_BYTES)) bytes${NC}"
            fi
            
            # Copy to output directory
            local output_dir="release/${output_name}"
            mkdir -p "$output_dir"
            cp "$binary_path" "$output_dir/"
            
            # Create compressed version
            echo -e "${YELLOW}Creating compressed version...${NC}"
            if command -v upx >/dev/null 2>&1; then
                upx --best --lzma "$output_dir/${PROJECT_NAME}" -o "$output_dir/${PROJECT_NAME}-compressed" 2>/dev/null || true
                local compressed_size=$(stat -c%s "$output_dir/${PROJECT_NAME}-compressed" 2>/dev/null || stat -f%z "$output_dir/${PROJECT_NAME}-compressed" 2>/dev/null || echo "0")
                local compressed_mb=$(echo "scale=2; $compressed_size / 1024 / 1024" | bc -l 2>/dev/null || echo "0")
                echo -e "${YELLOW}Compressed size: ${compressed_mb}MB${NC}"
            fi
            
            echo ""
        else
            echo -e "${RED}✗ Binary not found: ${binary_path}${NC}"
            return 1
        fi
    else
        echo -e "${RED}✗ Build failed for ${target}${NC}"
        return 1
    fi
}

# Install required targets
echo -e "${YELLOW}Installing Rust targets...${NC}"
rustup target add x86_64-unknown-linux-musl
rustup target add x86_64-pc-windows-gnu
rustup target add x86_64-apple-darwin
rustup target add aarch64-unknown-linux-musl
rustup target add aarch64-apple-darwin

# Build for different platforms
echo -e "${GREEN}=== Building for Linux (x64) ===${NC}"
build_target "x86_64-unknown-linux-musl" "linux-x64"

echo -e "${GREEN}=== Building for Windows (x64) ===${NC}"
build_target "x86_64-pc-windows-gnu" "windows-x64"

echo -e "${GREEN}=== Building for macOS (x64) ===${NC}"
build_target "x86_64-apple-darwin" "macos-x64"

echo -e "${GREEN}=== Building for Linux (ARM64) ===${NC}"
build_target "aarch64-unknown-linux-musl" "linux-arm64"

echo -e "${GREEN}=== Building for macOS (ARM64) ===${NC}"
build_target "aarch64-apple-darwin" "macos-arm64"

# Create release archive
echo -e "${YELLOW}Creating release archives...${NC}"
cd release

# Create tar.gz for Unix systems
for dir in linux-x64 macos-x64 linux-arm64 macos-arm64; do
    if [ -d "$dir" ]; then
        tar -czf "../${PROJECT_NAME}-${dir}.tar.gz" "$dir"
        echo -e "${GREEN}✓ Created ${PROJECT_NAME}-${dir}.tar.gz${NC}"
    fi
done

# Create zip for Windows
if [ -d "windows-x64" ]; then
    zip -r "../${PROJECT_NAME}-windows-x64.zip" "windows-x64"
    echo -e "${GREEN}✓ Created ${PROJECT_NAME}-windows-x64.zip${NC}"
fi

cd ..

# Generate checksums
echo -e "${YELLOW}Generating checksums...${NC}"
if command -v sha256sum >/dev/null 2>&1; then
    sha256sum ${PROJECT_NAME}-*.tar.gz ${PROJECT_NAME}-*.zip > checksums.sha256
    echo -e "${GREEN}✓ Generated checksums.sha256${NC}"
fi

# Final summary
echo ""
echo -e "${GREEN}=== Build Summary ===${NC}"
echo -e "${YELLOW}Release artifacts created in ./release/${NC}"
ls -lh ${PROJECT_NAME}-*.{tar.gz,zip} 2>/dev/null || true
echo ""
echo -e "${GREEN}Build completed successfully!${NC}"

# Optional: Run size analysis
if command -v bloaty >/dev/null 2>&1; then
    echo -e "${YELLOW}Running size analysis...${NC}"
    for binary in release/*/*/${PROJECT_NAME}; do
        if [ -f "$binary" ]; then
            echo -e "${YELLOW}Analyzing $(basename $(dirname $binary))...${NC}"
            bloaty "$binary" --tsv | head -20
            echo ""
        fi
    done
fi