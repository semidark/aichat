#!/bin/bash

# Kindle AI Chat - Manual Curl Test Suite
# This script runs comprehensive curl tests to verify server functionality
# Use this for manual verification of HTTP endpoints and real-world behavior

# set -e  # Exit on any error - disabled for better error handling

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SERVER_PORT=8000
SERVER_URL="http://localhost:${SERVER_PORT}"
STARTUP_WAIT=8
COMPILATION_WAIT=30
MAX_STARTUP_RETRIES=12
RETRY_INTERVAL=2

# Test counters
TESTS_PASSED=0
TESTS_FAILED=0
TOTAL_TESTS=0

# Utility functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
    ((TESTS_PASSED++))
}

log_error() {
    echo -e "${RED}[FAIL]${NC} $1"
    ((TESTS_FAILED++))
}

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# Check if server is running
check_server() {
    if curl -s --connect-timeout 3 --max-time 5 "${SERVER_URL}/" > /dev/null 2>&1; then
        return 0
    else
        return 1
    fi
}

# Wait for server to be ready with retries
wait_for_server() {
    local retries=0
    log_info "Waiting for server to be ready..."
    
    while [ $retries -lt $MAX_STARTUP_RETRIES ]; do
        if check_server; then
            log_success "Server is responding after $((retries * RETRY_INTERVAL)) seconds"
            return 0
        fi
        
        retries=$((retries + 1))
        log_info "Server not ready yet, retry $retries/$MAX_STARTUP_RETRIES in ${RETRY_INTERVAL}s..."
        sleep $RETRY_INTERVAL
    done
    
    log_error "Server failed to respond after $((MAX_STARTUP_RETRIES * RETRY_INTERVAL)) seconds"
    return 1
}

# Start the server
start_server() {
    log_info "Starting Kindle AI Chat server..."
    
    # Kill any existing server processes and free the port
    pkill -f "cargo run" 2>/dev/null || true
    lsof -ti:${SERVER_PORT} | xargs kill -9 2>/dev/null || true
    sleep 2
    
    # Check if port is still in use
    if lsof -i:${SERVER_PORT} >/dev/null 2>&1; then
        log_error "Port ${SERVER_PORT} is still in use by another process"
        lsof -i:${SERVER_PORT}
        exit 1
    fi
    
    # Start server in background
    log_info "Starting cargo run (this may take time for compilation)..."
    nohup cargo run > /tmp/kindle-server.log 2>&1 &
    SERVER_PID=$!
    
    log_info "Server started with PID: ${SERVER_PID}"
    
    # Give initial time for compilation and startup
    log_info "Waiting ${STARTUP_WAIT}s for initial compilation and startup..."
    
    # Monitor compilation progress
    local wait_time=0
    while [ $wait_time -lt $STARTUP_WAIT ]; do
        if [ -f /tmp/kindle-server.log ] && grep -q "Compiling" /tmp/kindle-server.log; then
            log_info "Compilation in progress..."
        elif [ -f /tmp/kindle-server.log ] && grep -q "Finished" /tmp/kindle-server.log; then
            log_info "Compilation finished, server should be starting..."
        fi
        sleep 2
        wait_time=$((wait_time + 2))
    done
    
    # Wait for server to be ready with retries
    if wait_for_server; then
        log_success "Server startup completed successfully"
    else
        log_error "Server failed to start or is not responding"
        log_info "Server log (last 30 lines):"
        tail -30 /tmp/kindle-server.log
        stop_server
        exit 1
    fi
}

# Stop the server
stop_server() {
    log_info "Stopping server..."
    
    # Try graceful shutdown first
    if [ ! -z "${SERVER_PID}" ]; then
        kill ${SERVER_PID} 2>/dev/null || true
        sleep 2
    fi
    
    # Force kill any remaining cargo run processes
    pkill -f "cargo run" 2>/dev/null || true
    
    # Force kill anything on our port
    lsof -ti:${SERVER_PORT} | xargs kill -9 2>/dev/null || true
    
    # Wait a moment for cleanup
    sleep 1
    
    log_info "Server stopped"
}

# Test basic GET endpoint
test_basic_get() {
    ((TOTAL_TESTS++))
    log_info "Testing GET / endpoint..."
    
    local response=$(curl -s -w "HTTPSTATUS:%{http_code};TIME:%{time_total}" "${SERVER_URL}/")
    local http_code=$(echo "$response" | grep -o "HTTPSTATUS:[0-9]*" | cut -d: -f2)
    local time_total=$(echo "$response" | grep -o "TIME:[0-9.]*" | cut -d: -f2)
    local body=$(echo "$response" | sed -E 's/HTTPSTATUS:[0-9]*;TIME:[0-9.]*$//')
    
    if [ "$http_code" = "200" ]; then
        if echo "$body" | grep -q "Kindle AI Chat"; then
            log_success "GET / returned 200 OK with correct content (${time_total}s)"
        else
            log_error "GET / returned 200 but content is incorrect"
        fi
    else
        log_error "GET / returned HTTP $http_code (expected 200)"
    fi
}

# Test chat endpoint
test_chat_endpoint() {
    ((TOTAL_TESTS++))
    log_info "Testing POST /api/chat endpoint..."
    
    local response=$(curl -s -w "HTTPSTATUS:%{http_code};TIME:%{time_total}" \
        -X POST "${SERVER_URL}/api/chat" \
        -H "Content-Type: application/json" \
        -d '{"message": "Hello, AI! This is a test message."}')
    
    local http_code=$(echo "$response" | grep -o "HTTPSTATUS:[0-9]*" | cut -d: -f2)
    local time_total=$(echo "$response" | grep -o "TIME:[0-9.]*" | cut -d: -f2)
    local body=$(echo "$response" | sed -E 's/HTTPSTATUS:[0-9]*;TIME:[0-9.]*$//')
    
    if [ "$http_code" = "200" ]; then
        if echo "$body" | grep -q '"status"'; then
            log_success "POST /api/chat returned 200 OK with JSON response (${time_total}s)"
            
            # Check response time (should be reasonable for Kindle)
            local time_numeric=$(echo "$time_total" | cut -d. -f1)
            if [ "$time_numeric" -le 10 ]; then
                log_success "Response time is acceptable for Kindle (${time_total}s)"
            else
                log_warning "Response time is slow for Kindle: ${time_total}s"
            fi
        else
            log_error "POST /api/chat returned 200 but JSON response is malformed"
        fi
    else
        log_error "POST /api/chat returned HTTP $http_code (expected 200)"
    fi
}

# Test session persistence
test_session_persistence() {
    ((TOTAL_TESTS++))
    log_info "Testing session persistence with cookies..."
    
    # Clean up any existing cookie file
    rm -f /tmp/test-cookies.txt
    
    # First request - should create a session
    local response1=$(curl -s -w "HTTPSTATUS:%{http_code}" \
        -c /tmp/test-cookies.txt \
        -X POST "${SERVER_URL}/api/chat" \
        -H "Content-Type: application/json" \
        -d '{"message": "First message"}')
    
    local http_code1=$(echo "$response1" | grep -o "HTTPSTATUS:[0-9]*" | cut -d: -f2)
    
    if [ "$http_code1" = "200" ] && [ -f /tmp/test-cookies.txt ]; then
        # Check if session cookie was created
        if grep -q "session_id" /tmp/test-cookies.txt; then
            log_success "Session cookie created successfully"
            
            # Second request - should use existing session
            local response2=$(curl -s -w "HTTPSTATUS:%{http_code}" \
                -b /tmp/test-cookies.txt \
                -X POST "${SERVER_URL}/api/chat" \
                -H "Content-Type: application/json" \
                -d '{"message": "Second message"}')
            
            local http_code2=$(echo "$response2" | grep -o "HTTPSTATUS:[0-9]*" | cut -d: -f2)
            
            if [ "$http_code2" = "200" ]; then
                log_success "Session persistence working correctly"
            else
                log_error "Second request with cookie failed (HTTP $http_code2)"
            fi
        else
            log_error "Session cookie was not created"
        fi
    else
        log_error "First request failed or cookie file not created"
    fi
    
    # Clean up
    rm -f /tmp/test-cookies.txt
}

# Test Kindle user agent
test_kindle_user_agent() {
    ((TOTAL_TESTS++))
    log_info "Testing with Kindle user agent..."
    
    local response=$(curl -s -w "HTTPSTATUS:%{http_code};TIME:%{time_total}" \
        -H "User-Agent: Kindle/3.0+" \
        "${SERVER_URL}/")
    
    local http_code=$(echo "$response" | grep -o "HTTPSTATUS:[0-9]*" | cut -d: -f2)
    local time_total=$(echo "$response" | grep -o "TIME:[0-9.]*" | cut -d: -f2)
    
    if [ "$http_code" = "200" ]; then
        log_success "Kindle user agent request successful (${time_total}s)"
    else
        log_error "Kindle user agent request failed (HTTP $http_code)"
    fi
}

# Test session file creation
test_session_files() {
    ((TOTAL_TESTS++))
    log_info "Testing session file creation..."
    
    # Send a message to create a session file
    curl -s "${SERVER_URL}/api/chat" \
        -H "Content-Type: application/json" \
        -d '{"message": "Test session file creation"}' > /dev/null
    
    # Check if session files exist
    if [ -d "data" ] && [ "$(ls -A data/*.json 2>/dev/null | wc -l)" -gt 0 ]; then
        local session_count=$(ls data/*.json 2>/dev/null | wc -l)
        log_success "Session files are being created (found $session_count files)"
        
        # Check if session file content is valid JSON
        local latest_file=$(ls -t data/*.json | head -n1)
        if python3 -m json.tool "$latest_file" > /dev/null 2>&1; then
            log_success "Session file contains valid JSON"
        else
            log_error "Session file contains invalid JSON"
        fi
    else
        log_error "No session files found in data/ directory"
    fi
}

# Print test summary
print_summary() {
    echo
    echo "================================================"
    echo "           CURL TEST SUMMARY"
    echo "================================================"
    echo "Total Tests: $TOTAL_TESTS"
    echo -e "Passed: ${GREEN}$TESTS_PASSED${NC}"
    echo -e "Failed: ${RED}$TESTS_FAILED${NC}"
    echo "================================================"
    
    if [ $TESTS_FAILED -eq 0 ]; then
        echo -e "${GREEN}All tests passed! ✅${NC}"
        return 0
    else
        echo -e "${RED}Some tests failed! ❌${NC}"
        return 1
    fi
}

# Main execution
main() {
    echo "================================================"
    echo "    Kindle AI Chat - Curl Test Suite"
    echo "================================================"
    echo
    
    # Trap to ensure server cleanup on exit
    trap stop_server EXIT
    
    # Check if we're in the right directory
    if [ ! -f "Cargo.toml" ]; then
        log_error "Please run this script from the project root directory"
        exit 1
    fi
    
    # Check if project has been built recently
    if [ ! -d "target" ] || [ ! -f "target/debug/aichat" ]; then
        log_info "Project not built yet - first compilation may take longer"
    fi
    
    # Start the server
    start_server
    
    # Run all tests
    log_info "Starting test execution..."
    test_basic_get
    log_info "test_basic_get completed"
    
    test_chat_endpoint
    log_info "test_chat_endpoint completed"
    
    test_session_persistence
    log_info "test_session_persistence completed"
    
    test_kindle_user_agent
    log_info "test_kindle_user_agent completed"
    
    test_session_files
    log_info "test_session_files completed"
    
    # Print summary and exit with appropriate code
    if print_summary; then
        exit 0
    else
        exit 1
    fi
}

# Run main function
main "$@" 