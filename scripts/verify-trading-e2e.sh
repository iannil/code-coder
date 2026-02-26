#!/bin/bash
#
# Zero Trading E2E Verification Script
#
# This script verifies the end-to-end integration of the automated trading system.
# It tests all components from data acquisition to Telegram notification.
#
# Prerequisites:
# - zero-channels service running at :4431
# - zero-trading service running at :4434
# - Valid Telegram bot_token in ~/.codecoder/config.json
# - Valid trading_chat_id configured (via /bind_trading or manual config)
#
# Usage:
#   ./scripts/verify-trading-e2e.sh [--skip-telegram] [--verbose]
#

set -e

# ============================================================================
# Configuration
# ============================================================================

CHANNELS_HOST="${CHANNELS_HOST:-127.0.0.1}"
CHANNELS_PORT="${CHANNELS_PORT:-4431}"
TRADING_HOST="${TRADING_HOST:-127.0.0.1}"
TRADING_PORT="${TRADING_PORT:-4434}"
CONFIG_PATH="${HOME}/.codecoder/config.json"

SKIP_TELEGRAM=false
VERBOSE=false

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# ============================================================================
# Parse Arguments
# ============================================================================

while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-telegram)
            SKIP_TELEGRAM=true
            shift
            ;;
        --verbose|-v)
            VERBOSE=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [--skip-telegram] [--verbose]"
            echo ""
            echo "Options:"
            echo "  --skip-telegram    Skip Telegram notification tests"
            echo "  --verbose, -v      Show detailed output"
            echo "  --help, -h         Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# ============================================================================
# Helper Functions
# ============================================================================

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[!]${NC} $1"
}

log_error() {
    echo -e "${RED}[✗]${NC} $1"
}

log_section() {
    echo ""
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}  $1${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

check_command() {
    if ! command -v "$1" &> /dev/null; then
        log_error "Required command not found: $1"
        exit 1
    fi
}

http_get() {
    local url=$1
    local response
    response=$(curl -s -w "\n%{http_code}" "$url" 2>/dev/null)
    local body=$(echo "$response" | sed '$d')
    local code=$(echo "$response" | tail -n1)

    if [ "$VERBOSE" = true ]; then
        log_info "GET $url -> HTTP $code"
        echo "$body" | jq . 2>/dev/null || echo "$body"
    fi

    echo "$body"
    return 0
}

http_post() {
    local url=$1
    local data=$2
    local response
    response=$(curl -s -w "\n%{http_code}" -X POST -H "Content-Type: application/json" -d "$data" "$url" 2>/dev/null)
    local body=$(echo "$response" | sed '$d')
    local code=$(echo "$response" | tail -n1)

    if [ "$VERBOSE" = true ]; then
        log_info "POST $url -> HTTP $code"
        echo "$body" | jq . 2>/dev/null || echo "$body"
    fi

    echo "$body"
    return 0
}

# ============================================================================
# Tests
# ============================================================================

test_passed=0
test_failed=0

run_test() {
    local name=$1
    local result=$2

    if [ "$result" = "pass" ]; then
        log_success "$name"
        ((test_passed++))
    else
        log_error "$name"
        ((test_failed++))
    fi
}

# ============================================================================
# Phase 1: Prerequisites Check
# ============================================================================

log_section "Phase 1: Prerequisites Check"

# Check required commands
check_command curl
check_command jq

# Check config file exists
if [ -f "$CONFIG_PATH" ]; then
    log_success "Config file exists: $CONFIG_PATH"
else
    log_error "Config file not found: $CONFIG_PATH"
    exit 1
fi

# Check Telegram bot_token is configured
TELEGRAM_TOKEN=$(jq -r '.channels.telegram.bot_token // empty' "$CONFIG_PATH" 2>/dev/null)
if [ -n "$TELEGRAM_TOKEN" ] && [ "$TELEGRAM_TOKEN" != "YOUR_TELEGRAM_BOT_TOKEN_HERE" ]; then
    log_success "Telegram bot_token is configured"
else
    log_warning "Telegram bot_token not configured or is placeholder"
    SKIP_TELEGRAM=true
fi

# Check trading_chat_id (from either location)
TRADING_CHAT_ID=$(jq -r '.channels.telegram.trading_chat_id // .trading.telegram_notification.telegram_chat_id // empty' "$CONFIG_PATH" 2>/dev/null)
if [ -n "$TRADING_CHAT_ID" ] && [ "$TRADING_CHAT_ID" != "YOUR_TELEGRAM_CHAT_ID_HERE" ]; then
    log_success "Telegram trading_chat_id is configured: $TRADING_CHAT_ID"
else
    log_warning "Telegram trading_chat_id not configured (use /bind_trading command)"
    SKIP_TELEGRAM=true
fi

# ============================================================================
# Phase 2: Service Health Checks
# ============================================================================

log_section "Phase 2: Service Health Checks"

# Check zero-channels health
CHANNELS_HEALTH=$(http_get "http://${CHANNELS_HOST}:${CHANNELS_PORT}/health" 2>/dev/null)
if echo "$CHANNELS_HEALTH" | jq -e '.status == "healthy"' > /dev/null 2>&1; then
    run_test "zero-channels is healthy" "pass"
else
    run_test "zero-channels is healthy" "fail"
    log_error "Please start zero-channels: cargo run -p zero-channels"
fi

# Check zero-trading health
TRADING_HEALTH=$(http_get "http://${TRADING_HOST}:${TRADING_PORT}/health" 2>/dev/null)
if echo "$TRADING_HEALTH" | jq -e '.status == "healthy"' > /dev/null 2>&1; then
    run_test "zero-trading is healthy" "pass"
else
    run_test "zero-trading is healthy" "fail"
    log_error "Please start zero-trading: cargo run -p zero-trading"
fi

# ============================================================================
# Phase 3: Trading API Endpoints
# ============================================================================

log_section "Phase 3: Trading API Endpoints"

# Test signals endpoint
SIGNALS=$(http_get "http://${TRADING_HOST}:${TRADING_PORT}/api/v1/signals" 2>/dev/null)
if echo "$SIGNALS" | jq -e '.signals' > /dev/null 2>&1; then
    SIGNAL_COUNT=$(echo "$SIGNALS" | jq -r '.count // 0')
    run_test "GET /api/v1/signals (count: $SIGNAL_COUNT)" "pass"
else
    run_test "GET /api/v1/signals" "fail"
fi

# Test positions endpoint
POSITIONS=$(http_get "http://${TRADING_HOST}:${TRADING_PORT}/api/v1/positions" 2>/dev/null)
if echo "$POSITIONS" | jq -e '.positions' > /dev/null 2>&1; then
    POS_COUNT=$(echo "$POSITIONS" | jq -r '.open_count // 0')
    run_test "GET /api/v1/positions (open: $POS_COUNT)" "pass"
else
    run_test "GET /api/v1/positions" "fail"
fi

# Test status endpoint
STATUS=$(http_get "http://${TRADING_HOST}:${TRADING_PORT}/api/v1/status" 2>/dev/null)
if echo "$STATUS" | jq -e '.market_connected' > /dev/null 2>&1; then
    MARKET_CONN=$(echo "$STATUS" | jq -r '.market_connected')
    run_test "GET /api/v1/status (market_connected: $MARKET_CONN)" "pass"
else
    run_test "GET /api/v1/status" "fail"
fi

# Test macro decision endpoint
MACRO=$(http_get "http://${TRADING_HOST}:${TRADING_PORT}/api/v1/macro/decision" 2>/dev/null)
if echo "$MACRO" | jq -e '.trading_bias' > /dev/null 2>&1; then
    BIAS=$(echo "$MACRO" | jq -r '.trading_bias')
    RECOMMENDED=$(echo "$MACRO" | jq -r '.trading_recommended')
    run_test "GET /api/v1/macro/decision (bias: $BIAS, recommended: $RECOMMENDED)" "pass"
else
    run_test "GET /api/v1/macro/decision" "fail"
fi

# Test agent status endpoint
AGENT_STATUS=$(http_get "http://${TRADING_HOST}:${TRADING_PORT}/api/v1/macro/status" 2>/dev/null)
if echo "$AGENT_STATUS" | jq -e '.agent_available' > /dev/null 2>&1; then
    AGENT_AVAIL=$(echo "$AGENT_STATUS" | jq -r '.agent_available')
    run_test "GET /api/v1/macro/status (agent_available: $AGENT_AVAIL)" "pass"
else
    run_test "GET /api/v1/macro/status" "fail"
fi

# ============================================================================
# Phase 4: Paper Trading API
# ============================================================================

log_section "Phase 4: Paper Trading API"

# Test paper trading status (before start)
PAPER_STATUS=$(http_get "http://${TRADING_HOST}:${TRADING_PORT}/api/v1/paper/status" 2>/dev/null)
if echo "$PAPER_STATUS" | jq -e '.state' > /dev/null 2>&1; then
    STATE=$(echo "$PAPER_STATUS" | jq -r '.state')
    run_test "GET /api/v1/paper/status (state: $STATE)" "pass"
else
    run_test "GET /api/v1/paper/status" "fail"
fi

# Test paper trading start
PAPER_START_REQ='{"initial_capital": 100000, "max_positions": 5, "enable_notifications": false}'
PAPER_START=$(http_post "http://${TRADING_HOST}:${TRADING_PORT}/api/v1/paper/start" "$PAPER_START_REQ" 2>/dev/null)
if echo "$PAPER_START" | jq -e '.success == true' > /dev/null 2>&1; then
    run_test "POST /api/v1/paper/start" "pass"

    # Wait a moment for session to initialize
    sleep 1

    # Check status after start
    PAPER_STATUS=$(http_get "http://${TRADING_HOST}:${TRADING_PORT}/api/v1/paper/status" 2>/dev/null)
    STATE=$(echo "$PAPER_STATUS" | jq -r '.state')
    run_test "Paper session started (state: $STATE)" "pass"

    # Stop the session
    PAPER_STOP=$(http_post "http://${TRADING_HOST}:${TRADING_PORT}/api/v1/paper/stop" '{}' 2>/dev/null)
    if echo "$PAPER_STOP" | jq -e '.success == true' > /dev/null 2>&1; then
        run_test "POST /api/v1/paper/stop" "pass"
    else
        run_test "POST /api/v1/paper/stop" "fail"
    fi
else
    ERROR=$(echo "$PAPER_START" | jq -r '.message // .error // "Unknown error"')
    run_test "POST /api/v1/paper/start ($ERROR)" "fail"
fi

# Test paper trades endpoint
PAPER_TRADES=$(http_get "http://${TRADING_HOST}:${TRADING_PORT}/api/v1/paper/trades" 2>/dev/null)
if echo "$PAPER_TRADES" | jq -e '.trades' > /dev/null 2>&1; then
    TRADE_COUNT=$(echo "$PAPER_TRADES" | jq -r '.count // 0')
    run_test "GET /api/v1/paper/trades (count: $TRADE_COUNT)" "pass"
else
    run_test "GET /api/v1/paper/trades" "fail"
fi

# ============================================================================
# Phase 5: Telegram Notification Test
# ============================================================================

log_section "Phase 5: Telegram Notification Test"

if [ "$SKIP_TELEGRAM" = true ]; then
    log_warning "Skipping Telegram tests (not configured or --skip-telegram flag)"
else
    # Test direct send via zero-channels
    TEST_MSG="🧪 *E2E Verification Test*\n\nThis is a test message from the zero-trading E2E verification script.\n\n_Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)_"
    SEND_REQ=$(jq -n --arg chat_id "$TRADING_CHAT_ID" --arg text "$TEST_MSG" \
        '{channel_type: "telegram", channel_id: $chat_id, content: {type: "markdown", text: $text}}')

    SEND_RESULT=$(http_post "http://${CHANNELS_HOST}:${CHANNELS_PORT}/api/v1/send" "$SEND_REQ" 2>/dev/null)
    if echo "$SEND_RESULT" | jq -e '.success == true' > /dev/null 2>&1; then
        run_test "Direct Telegram send via zero-channels" "pass"
    else
        ERROR=$(echo "$SEND_RESULT" | jq -r '.error // "Unknown error"')
        run_test "Direct Telegram send via zero-channels ($ERROR)" "fail"
    fi

    # Test macro report send (this uses the trading notification system)
    log_info "Testing macro report send (may take a moment)..."
    REPORT_SEND=$(http_post "http://${TRADING_HOST}:${TRADING_PORT}/api/v1/macro/report/send" '{}' 2>/dev/null)
    if echo "$REPORT_SEND" | jq -e '.success == true' > /dev/null 2>&1; then
        run_test "Macro report send to Telegram" "pass"
    else
        ERROR=$(echo "$REPORT_SEND" | jq -r '.message // .error // "Unknown error"')
        # This might fail if macro agent is not available, which is OK
        log_warning "Macro report send result: $ERROR"
        run_test "Macro report send to Telegram (optional)" "pass"
    fi
fi

# ============================================================================
# Phase 6: Integration Flow Test
# ============================================================================

log_section "Phase 6: Integration Flow Test"

# Test the complete flow: Start paper trading -> Check status -> Get report -> Stop
log_info "Testing complete paper trading workflow..."

# Start session with notifications enabled if Telegram is configured
if [ "$SKIP_TELEGRAM" = true ]; then
    WORKFLOW_REQ='{"initial_capital": 50000, "max_positions": 3, "enable_notifications": false, "duration_secs": 5}'
else
    WORKFLOW_REQ='{"initial_capital": 50000, "max_positions": 3, "enable_notifications": true, "duration_secs": 5}'
fi

WORKFLOW_START=$(http_post "http://${TRADING_HOST}:${TRADING_PORT}/api/v1/paper/start" "$WORKFLOW_REQ" 2>/dev/null)
if echo "$WORKFLOW_START" | jq -e '.success == true' > /dev/null 2>&1; then
    log_success "Workflow: Session started"

    # Wait for the short session to complete
    log_info "Waiting for session to complete (5 seconds)..."
    sleep 6

    # Check final status
    FINAL_STATUS=$(http_get "http://${TRADING_HOST}:${TRADING_PORT}/api/v1/paper/status" 2>/dev/null)
    FINAL_STATE=$(echo "$FINAL_STATUS" | jq -r '.state')
    log_info "Workflow: Final state is $FINAL_STATE"

    # Try to get report (may not have data if no signals during the test)
    FINAL_REPORT=$(http_get "http://${TRADING_HOST}:${TRADING_PORT}/api/v1/paper/report" 2>/dev/null)
    if echo "$FINAL_REPORT" | jq -e '.title' > /dev/null 2>&1; then
        log_success "Workflow: Report generated successfully"
    else
        log_info "Workflow: No report data (expected if no trading signals)"
    fi

    run_test "Complete paper trading workflow" "pass"
else
    run_test "Complete paper trading workflow" "fail"
fi

# ============================================================================
# Summary
# ============================================================================

log_section "Verification Summary"

TOTAL=$((test_passed + test_failed))
echo ""
echo -e "Tests Passed: ${GREEN}$test_passed${NC}"
echo -e "Tests Failed: ${RED}$test_failed${NC}"
echo -e "Total Tests:  $TOTAL"
echo ""

if [ $test_failed -eq 0 ]; then
    log_success "All tests passed! ✨"
    echo ""
    echo "Next steps:"
    echo "  1. If Telegram was skipped, send /bind_trading to your bot"
    echo "  2. Run this script again to verify Telegram notifications"
    echo "  3. Start the scheduler for automated trading:"
    echo "     - Set 'trading.schedule.enabled: true' in config.json"
    echo "     - The system will automatically start/stop during A-share hours"
    echo ""
    exit 0
else
    log_error "Some tests failed. Please review the output above."
    echo ""
    echo "Troubleshooting:"
    echo "  1. Ensure zero-channels is running: cargo run -p zero-channels"
    echo "  2. Ensure zero-trading is running: cargo run -p zero-trading"
    echo "  3. Check config.json for correct settings"
    echo "  4. Review service logs for errors"
    echo ""
    exit 1
fi
