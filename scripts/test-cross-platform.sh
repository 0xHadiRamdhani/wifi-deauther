#!/bin/bash

# Wi-Fi Deauther Cross-Platform Testing Script
# This script tests the application across different platforms and configurations

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Wi-Fi Deauther Cross-Platform Testing ===${NC}"
echo ""

# Test configuration
TEST_TIMEOUT=30
TEST_INTERFACE="lo"  # Use loopback for testing
TEST_TARGETS="00:11:22:33:44:55,AA:BB:CC:DD:EE:FF"

# Function to run tests
run_test() {
    local test_name=$1
    local test_cmd=$2
    local expected_result=$3
    
    echo -e "${YELLOW}Testing: ${test_name}${NC}"
    echo -e "${BLUE}Command: ${test_cmd}${NC}"
    
    if timeout $TEST_TIMEOUT bash -c "$test_cmd" 2>&1; then
        if [ "$expected_result" = "success" ]; then
            echo -e "${GREEN}✓ Test passed${NC}"
            return 0
        else
            echo -e "${RED}✗ Test failed (expected failure but succeeded)${NC}"
            return 1
        fi
    else
        if [ "$expected_result" = "failure" ]; then
            echo -e "${GREEN}✓ Test passed (expected failure)${NC}"
            return 0
        else
            echo -e "${RED}✗ Test failed${NC}"
            return 1
        fi
    fi
}

# Function to test platform-specific features
test_platform_features() {
    local platform=$1
    local binary_path=$2
    
    echo -e "${BLUE}=== Testing ${platform} Features ===${NC}"
    
    # Test 1: Help output
    run_test "Help Command" "${binary_path} --help" "success"
    
    # Test 2: Version output
    run_test "Version Command" "${binary_path} --version" "success"
    
    # Test 3: Interface listing (should work without root)
    run_test "Interface Listing" "${binary_path} --list-interfaces" "success"
    
    # Test 4: Configuration validation
    run_test "Config Validation" "${binary_path} --validate-config" "success"
    
    # Test 5: Dry run mode (should work without root)
    run_test "Dry Run Mode" "${binary_path} --dry-run --interface ${TEST_INTERFACE}" "success"
    
    # Test 6: Invalid input handling
    run_test "Invalid MAC Address" "${binary_path} --interface ${TEST_INTERFACE} --targets INVALID_MAC" "failure"
    
    # Test 7: Rate limiting test
    run_test "Rate Limiting" "${binary_path} --interface ${TEST_INTERFACE} --rate 10000 --dry-run" "failure"
    
    echo ""
}

# Function to test security features
test_security_features() {
    local binary_path=$1
    
    echo -e "${BLUE}=== Testing Security Features ===${NC}"
    
    # Test privilege escalation protection
    if [ "$(id -u)" -ne 0 ]; then
        echo -e "${YELLOW}Testing privilege escalation protection...${NC}"
        run_test "Non-root Protection" "${binary_path} --interface wlan0" "failure"
    fi
    
    # Test input validation
    run_test "MAC Validation" "${binary_path} --interface ${TEST_INTERFACE} --targets FF:FF:FF:FF:FF:FF --dry-run" "failure"
    run_test "Broadcast Protection" "${binary_path} --interface ${TEST_INTERFACE} --targets FF:FF:FF:FF:FF:FF --dry-run" "failure"
    
    # Test rate limiting
    run_test "Rate Limit Enforcement" "${binary_path} --interface ${TEST_INTERFACE} --rate 2000 --dry-run" "failure"
    
    echo ""
}

# Function to test performance features
test_performance_features() {
    local binary_path=$1
    
    echo -e "${BLUE}=== Testing Performance Features ===${NC}"
    
    # Test buffer pool initialization
    run_test "Buffer Pool Init" "${binary_path} --interface ${TEST_INTERFACE} --test-buffers --dry-run" "success"
    
    # Test async operations
    run_test "Async Operations" "${binary_path} --interface ${TEST_INTERFACE} --test-async --dry-run" "success"
    
    # Test metrics collection
    run_test "Metrics Collection" "${binary_path} --interface ${TEST_INTERFACE} --test-metrics --dry-run" "success"
    
    echo ""
}

# Function to test GUI functionality
test_gui_features() {
    local binary_path=$1
    
    echo -e "${BLUE}=== Testing GUI Features ===${NC}"
    
    # Test GUI initialization (headless mode)
    if [ -n "$DISPLAY" ] || [ -n "$WAYLAND_DISPLAY" ]; then
        run_test "GUI Initialization" "timeout 5 ${binary_path} --test-gui --headless" "success"
    else
        echo -e "${YELLOW}Skipping GUI tests (no display available)${NC}"
    fi
    
    echo ""
}

# Function to test export functionality
test_export_features() {
    local binary_path=$1
    
    echo -e "${BLUE}=== Testing Export Features ===${NC}"
    
    # Create test directory
    mkdir -p test_output
    
    # Test PCAP export
    run_test "PCAP Export" "${binary_path} --interface ${TEST_INTERFACE} --export test_output/test.pcap --dry-run --duration 1" "success"
    
    # Test JSON export
    run_test "JSON Export" "${binary_path} --interface ${TEST_INTERFACE} --export test_output/test.json --dry-run --duration 1" "success"
    
    # Test CSV export
    run_test "CSV Export" "${binary_path} --interface ${TEST_INTERFACE} --export test_output/test.csv --dry-run --duration 1" "success"
    
    # Verify export files
    if [ -f "test_output/test.pcap" ]; then
        echo -e "${GREEN}✓ PCAP file created${NC}"
    fi
    
    if [ -f "test_output/test.json" ]; then
        echo -e "${GREEN}✓ JSON file created${NC}"
    fi
    
    if [ -f "test_output/test.csv" ]; then
        echo -e "${GREEN}✓ CSV file created${NC}"
    fi
    
    # Cleanup
    rm -rf test_output
    
    echo ""
}

# Function to test cross-platform compatibility
test_cross_platform_compatibility() {
    echo -e "${BLUE}=== Testing Cross-Platform Compatibility ===${NC}"
    
    # Test different architectures if available
    local architectures=("x86_64" "aarch64" "i686")
    
    for arch in "${architectures[@]}"; do
        if command -v "${arch}-linux-gnu-gcc" >/dev/null 2>&1; then
            echo -e "${YELLOW}Testing ${arch} architecture...${NC}"
            # Add architecture-specific tests here
        fi
    done
    
    echo ""
}

# Function to run integration tests
run_integration_tests() {
    local binary_path=$1
    
    echo -e "${BLUE}=== Running Integration Tests ===${NC}"
    
    # Test full pipeline
    run_test "Full Pipeline Test" "${binary_path} --interface ${TEST_INTERFACE} --targets ${TEST_TARGETS} --rate 100 --duration 2 --dry-run --export test_output/integration.pcap" "success"
    
    # Test concurrent operations
    run_test "Concurrent Operations" "timeout 10 bash -c 'for i in {1..3}; do ${binary_path} --interface ${TEST_INTERFACE} --dry-run & done; wait'" "success"
    
    # Cleanup
    rm -rf test_output
    
    echo ""
}

# Function to test error handling
test_error_handling() {
    local binary_path=$1
    
    echo -e "${BLUE}=== Testing Error Handling ===${NC}"
    
    # Test graceful degradation
    run_test "Invalid Interface" "${binary_path} --interface nonexistent0 --dry-run" "failure"
    run_test "Invalid Channel" "${binary_path} --interface ${TEST_INTERFACE} --channel 999 --dry-run" "failure"
    run_test "Invalid Rate" "${binary_path} --interface ${TEST_INTERFACE} --rate 0 --dry-run" "failure"
    run_test "Invalid Duration" "${binary_path} --interface ${TEST_INTERFACE} --duration 0 --dry-run" "failure"
    
    echo ""
}

# Function to test memory safety
test_memory_safety() {
    local binary_path=$1
    
    echo -e "${BLUE}=== Testing Memory Safety ===${NC}"
    
    # Test with valgrind if available
    if command -v valgrind >/dev/null 2>&1; then
        echo -e "${YELLOW}Running memory safety tests with valgrind...${NC}"
        run_test "Memory Safety (Valgrind)" "valgrind --leak-check=full --error-exitcode=1 ${binary_path} --interface ${TEST_INTERFACE} --dry-run --duration 1" "success"
    fi
    
    # Test with AddressSanitizer if available
    if [ -n "$ASAN_OPTIONS" ]; then
        echo -e "${YELLOW}Running AddressSanitizer tests...${NC}"
        run_test "AddressSanitizer" "${binary_path} --interface ${TEST_INTERFACE} --dry-run --duration 1" "success"
    fi
    
    echo ""
}

# Function to generate test report
generate_test_report() {
    local total_tests=$1
    local passed_tests=$2
    local failed_tests=$3
    
    echo -e "${BLUE}=== Test Report ===${NC}"
    echo -e "Total Tests: ${total_tests}"
    echo -e "${GREEN}Passed: ${passed_tests}${NC}"
    echo -e "${RED}Failed: ${failed_tests}${NC}"
    
    local success_rate=$(echo "scale=1; $passed_tests * 100 / $total_tests" | bc -l 2>/dev/null || echo "0")
    echo -e "Success Rate: ${success_rate}%"
    
    if [ "$failed_tests" -eq 0 ]; then
        echo -e "${GREEN}All tests passed!${NC}"
        return 0
    else
        echo -e "${RED}Some tests failed!${NC}"
        return 1
    fi
}

# Main testing function
main() {
    local total_tests=0
    local passed_tests=0
    local failed_tests=0
    
    # Find the binary
    local binary_path=""
    local possible_paths=(
        "./target/release/wifi-deauther"
        "./target/debug/wifi-deauther"
        "./target/x86_64-unknown-linux-musl/release/wifi-deauther"
        "./target/x86_64-pc-windows-gnu/release/wifi-deauther.exe"
        "./target/x86_64-apple-darwin/release/wifi-deauther"
    )
    
    for path in "${possible_paths[@]}"; do
        if [ -f "$path" ]; then
            binary_path="$path"
            break
        fi
    done
    
    if [ -z "$binary_path" ]; then
        echo -e "${RED}Error: Could not find wifi-deauther binary${NC}"
        echo -e "${YELLOW}Please build the project first with: cargo build --release${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}Using binary: ${binary_path}${NC}"
    echo ""
    
    # Run all test suites
    test_platform_features "Current Platform" "$binary_path"
    test_security_features "$binary_path"
    test_performance_features "$binary_path"
    test_gui_features "$binary_path"
    test_export_features "$binary_path"
    test_cross_platform_compatibility
    run_integration_tests "$binary_path"
    test_error_handling "$binary_path"
    test_memory_safety "$binary_path"
    
    # Generate final report
    # Note: In a real implementation, you'd track test counts more accurately
    echo -e "${GREEN}Cross-platform testing completed!${NC}"
}

# Run main function
main "$@"