#!/bin/bash
# Cross-platform target installation and build verification script for Horizon-Sockets
# Works on Linux, macOS, and Windows (with WSL/Git Bash)

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
GRAY='\033[0;37m'
NC='\033[0m' # No Color

# Supported targets for Horizon-Sockets
TARGETS=(
    "x86_64-pc-windows-msvc"      # Windows 64-bit
    "x86_64-apple-darwin"         # macOS 64-bit Intel  
    "aarch64-apple-darwin"        # macOS ARM64 (Apple Silicon)
    "x86_64-unknown-linux-gnu"    # Linux 64-bit GNU
    "x86_64-unknown-linux-musl"   # Linux 64-bit musl (Alpine)
    "aarch64-unknown-linux-gnu"   # Linux ARM64 GNU
    "x86_64-unknown-freebsd"      # FreeBSD 64-bit
    "x86_64-unknown-netbsd"       # NetBSD 64-bit
    "x86_64-unknown-openbsd"      # OpenBSD 64-bit
)

# Feature configurations to test
declare -A CONFIGS
CONFIGS[default]=""
CONFIGS[full]="--features full"
CONFIGS[mio-only]="--no-default-features --features mio-runtime"

# Parse command line arguments
INSTALL_ONLY=false
CHECK_ONLY=false
VERBOSE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --install-only)
            INSTALL_ONLY=true
            shift
            ;;
        --check-only)
            CHECK_ONLY=true
            shift
            ;;
        --verbose|-v)
            VERBOSE=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "OPTIONS:"
            echo "  --install-only    Only install targets, don't run builds"
            echo "  --check-only      Only run build checks, don't install targets"
            echo "  --verbose, -v     Show detailed output"
            echo "  --help, -h        Show this help message"
            echo ""
            echo "Supported targets:"
            printf '  %s\n' "${TARGETS[@]}"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Global variables for results tracking
declare -a RESULTS
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

echo -e "${CYAN}🚀 Horizon-Sockets Cross-Platform Build Checker${NC}"
echo -e "${CYAN}=================================================${NC}"

install_targets() {
    echo -e "\n${YELLOW}📦 Installing Rust targets...${NC}"
    
    for target in "${TARGETS[@]}"; do
        echo -n "  Installing $target..."
        if output=$(rustup target add "$target" 2>&1); then
            echo -e " ${GREEN}✅${NC}"
            if [[ "$VERBOSE" == "true" ]]; then
                echo -e "    ${GRAY}$output${NC}"
            fi
        else
            echo -e " ${RED}❌${NC}"
            echo -e "    ${RED}Error: $output${NC}"
        fi
    done
}

test_target() {
    local target="$1"
    local config_name="$2"
    local features="$3"
    
    local cmd="cargo build --target $target $features"
    
    if [[ "$VERBOSE" == "true" ]]; then
        echo -e "    ${GRAY}Command: $cmd${NC}"
    fi
    
    if output=$(eval "$cmd" 2>&1); then
        return 0
    else
        if [[ "$VERBOSE" == "true" ]]; then
            echo -e "    ${RED}Error output:${NC}"
            echo -e "    ${GRAY}$output${NC}"
        fi
        return 1
    fi
}

check_builds() {
    echo -e "\n${YELLOW}🔨 Testing builds across targets and feature configurations...${NC}"
    
    local total_configs=${#CONFIGS[@]}
    TOTAL_TESTS=$((${#TARGETS[@]} * total_configs))
    local current_test=0
    
    for target in "${TARGETS[@]}"; do
        echo -e "\n${MAGENTA}📋 Target: $target${NC}"
        
        for config_name in "${!CONFIGS[@]}"; do
            current_test=$((current_test + 1))
            local progress=$(( (current_test * 100) / TOTAL_TESTS ))
            
            echo -n "  [$progress%] Testing $config_name..."
            
            if test_target "$target" "$config_name" "${CONFIGS[$config_name]}"; then
                echo -e " ${GREEN}✅${NC}"
                RESULTS+=("$target|$config_name|PASS")
                PASSED_TESTS=$((PASSED_TESTS + 1))
            else
                echo -e " ${RED}❌${NC}"
                RESULTS+=("$target|$config_name|FAIL")
                FAILED_TESTS=$((FAILED_TESTS + 1))
            fi
        done
    done
}

show_summary() {
    echo -e "\n${CYAN}📊 BUILD SUMMARY${NC}"
    echo -e "${CYAN}=================${NC}"
    
    # Group results by target
    declare -A target_results
    for result in "${RESULTS[@]}"; do
        IFS='|' read -r target config status <<< "$result"
        if [[ -z "${target_results[$target]}" ]]; then
            target_results[$target]=""
        fi
        target_results[$target]+="$config:$status "
    done
    
    # Display results by target
    for target in "${TARGETS[@]}"; do
        if [[ -n "${target_results[$target]}" ]]; then
            local passed=0
            local failed=0
            local tests="${target_results[$target]}"
            
            # Count passes and failures
            for test in $tests; do
                IFS=':' read -r config status <<< "$test"
                if [[ "$status" == "PASS" ]]; then
                    passed=$((passed + 1))
                else
                    failed=$((failed + 1))
                fi
            done
            
            local color="${GREEN}"
            if [[ $failed -gt 0 ]]; then
                color="${YELLOW}"
            fi
            
            echo -e "\n${color}$target${NC}"
            echo -e "  ${GREEN}✅ Passed: $passed${NC}"
            if [[ $failed -gt 0 ]]; then
                echo -e "  ${RED}❌ Failed: $failed${NC}"
            fi
            
            # Show individual test results
            for test in $tests; do
                IFS=':' read -r config status <<< "$test"
                local symbol="${GREEN}✅${NC}"
                if [[ "$status" == "FAIL" ]]; then
                    symbol="${RED}❌${NC}"
                fi
                echo -e "    $symbol ${GRAY}$config${NC}"
            done
        fi
    done
    
    # Overall statistics
    local success_rate=$(( (PASSED_TESTS * 100) / TOTAL_TESTS ))
    
    echo -e "\n${CYAN}🎯 OVERALL RESULTS${NC}"
    echo -e "${CYAN}==================${NC}"
    echo -e "Total Tests: $TOTAL_TESTS"
    echo -e "${GREEN}Passed: $PASSED_TESTS${NC}"
    if [[ $FAILED_TESTS -gt 0 ]]; then
        echo -e "${RED}Failed: $FAILED_TESTS${NC}"
        echo -e "${YELLOW}Success Rate: $success_rate%${NC}"
    else
        echo -e "${GREEN}Failed: $FAILED_TESTS${NC}"
        echo -e "${GREEN}Success Rate: $success_rate%${NC}"
    fi
}

# Main execution
if [[ "$CHECK_ONLY" != "true" ]]; then
    install_targets
fi

if [[ "$INSTALL_ONLY" != "true" ]]; then
    check_builds
    show_summary
    
    # Exit with appropriate code
    if [[ $FAILED_TESTS -eq 0 ]]; then
        echo -e "\n${GREEN}🎉 All tests passed!${NC}"
        exit 0
    else
        echo -e "\n${RED}⚠️  Some tests failed. Check the summary above.${NC}"
        exit 1
    fi
fi