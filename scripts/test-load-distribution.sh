#!/bin/bash

# Load Distribution Performance Test
# Tests concurrent request handling across regions

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
BOLD='\033[1m'
RESET='\033[0m'

# Configuration
API_URL="https://api-proxy.admice.com/"
export AUTH_TOKEN
AUTH_TOKEN=$(grep 'AUTH_TOKEN=' .env | sed 's/^AUTH_TOKEN=//' | tr -d '\n')
REGIONS=("wnam" "enam" "weur" "eeur" "apac")

# Check dependencies
if ! command -v jq &> /dev/null; then
    echo -e "${RED}Error: jq is required but not installed${RESET}"
    exit 1
fi

if [ -z "$AUTH_TOKEN" ]; then
    echo -e "${RED}Error: AUTH_TOKEN not found in .env file${RESET}"
    exit 1
fi

# Print header
print_header() {
    echo -e "\n${BOLD}${CYAN}╔════════════════════════════════════════════════════════════╗${RESET}"
    echo -e "${BOLD}${CYAN}║        API Proxy Load Distribution Performance Test        ║${RESET}"
    echo -e "${BOLD}${CYAN}╚════════════════════════════════════════════════════════════╝${RESET}\n"
}

# Print test info
print_test_info() {
    local test_name=$1
    local requests_per_region=$2
    local total=$((requests_per_region * ${#REGIONS[@]}))

    echo -e "${BOLD}${PURPLE}═══════════════════════════════════════════════════════════${RESET}"
    echo -e "${BOLD}Test: ${YELLOW}$test_name${RESET}"
    echo -e "Regions: ${CYAN}${REGIONS[*]}${RESET}"
    echo -e "Requests per region: ${GREEN}$requests_per_region${RESET}"
    echo -e "Total concurrent requests: ${GREEN}$total${RESET}"
    echo -e "${BOLD}${PURPLE}═══════════════════════════════════════════════════════════${RESET}\n"
}

# Make a single request
make_request() {
    local region=$1
    local index=$2
    local token=$3

    curl -s -X POST "$API_URL" \
        -H "Content-Type: application/json" \
        -H "X-CF-Region: $region" \
        -H "Authorization: Bearer $token" \
        -d '{"url": "https://httpbin.org/get", "method": "get", "params": {"test": "'"$index"'", "region": "'"$region"'"}}' \
        2>/dev/null
}

# Run test with N requests per region
run_test() {
    local requests_per_region=$1
    local temp_dir=$(mktemp -d)
    local start_time=$(date +%s.%N)

    # Launch concurrent requests
    local total_requests=0
    for region in "${REGIONS[@]}"; do
        for i in $(seq 1 $requests_per_region); do
            make_request "$region" "$i" "$AUTH_TOKEN" > "$temp_dir/${region}_${i}.json" &
            ((total_requests++))
        done
    done

    # Show progress
    echo -ne "${CYAN}⏳ Processing $total_requests concurrent requests...${RESET}"

    # Wait for all requests to complete
    wait

    local end_time=$(date +%s.%N)
    local elapsed=$(echo "$end_time - $start_time" | bc)

    # Analyze results
    local success=0
    local failed=0

    for result_file in "$temp_dir"/*.json; do
        if [ -f "$result_file" ] && [ -s "$result_file" ]; then
            # Check if response contains "status":200
            if grep -q '"status":200' "$result_file"; then
                ((success++))
            else
                ((failed++))
            fi
        else
            ((failed++))
        fi
    done

    # Print results
    echo -e "\r${GREEN}✓ Completed in ${BOLD}${elapsed}s${RESET}${GREEN}${RESET}\n"

    echo -e "${BOLD}Results:${RESET}"
    printf "  ${GREEN}✓ Success:${RESET} %d/%d (%.1f%%)\n" $success $total_requests $(echo "scale=1; $success * 100 / $total_requests" | bc)

    if [ $failed -gt 0 ]; then
        printf "  ${RED}✗ Failed:${RESET}  %d/%d (%.1f%%)\n" $failed $total_requests $(echo "scale=1; $failed * 100 / $total_requests" | bc)
    fi

    local rps=$(echo "scale=2; $total_requests / $elapsed" | bc)
    echo -e "  ${CYAN}⚡ Throughput:${RESET} ${BOLD}$rps${RESET} req/s"
    echo -e "  ${YELLOW}⏱  Avg latency:${RESET} ${BOLD}$(echo "scale=0; $elapsed * 1000 / $total_requests" | bc)${RESET}ms per request"

    # Regional breakdown
    echo -e "\n${BOLD}Regional Breakdown:${RESET}"
    printf "  %-10s %10s %10s\n" "Region" "Success" "Avg Time"
    printf "  ${PURPLE}%-10s %10s %10s${RESET}\n" "──────" "───────" "────────"

    for region in "${REGIONS[@]}"; do
        local region_success=$(ls "$temp_dir/${region}"_*.json 2>/dev/null | wc -l | tr -d ' ')
        local region_time=$(echo "scale=0; $elapsed * 1000 / $requests_per_region" | bc)
        printf "  %-10s ${GREEN}%10d${RESET} %9dms\n" "$region" "$region_success" "$region_time"
    done

    # Cleanup
    rm -rf "$temp_dir"

    echo ""
}

# Main execution
print_header

# Test 1: Warmup - 1 request per region
print_test_info "Warmup Test (Cold Start)" 1
run_test 1

# Wait a bit
sleep 2

# Test 2: Heavy load - 100 requests per region
print_test_info "Heavy Load Test (Hot Start)" 100
run_test 100

# Final summary
echo -e "${BOLD}${GREEN}╔════════════════════════════════════════════════════════════╗${RESET}"
echo -e "${BOLD}${GREEN}║                    Tests Completed! ✓                      ║${RESET}"
echo -e "${BOLD}${GREEN}╚════════════════════════════════════════════════════════════╝${RESET}\n"
