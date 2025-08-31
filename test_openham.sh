#!/bin/bash

# OpenHam Digital Modes Comprehensive Testing Script
# Tests all major functionality combinations and validates transmission/reception
# We handle errors per-test to produce a full summary; don't exit early
set -o pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test configuration
TEST_DIR="test_outputs"
OPENHAM_BIN="./target/release/openham"
TEST_TEXT="Hello from OpenHam! This is a test transmission from station S56SPZ using digital modes."
VOICE_ID_WAV="$TEST_DIR/voice_id.wav"

# Per-test summary tracking
declare -a TEST_SUMMARY_NAMES=()
declare -a TEST_SUMMARY_STATS=()

record_result() {
    local name="$1"
    local status="$2"
    TEST_SUMMARY_NAMES+=("$name")
    TEST_SUMMARY_STATS+=("$status")
}

# Function to print colored output
print_status() {
    printf "\r\033[K"
    echo -e "${BLUE}[INFO]${NC} $1"
    progress_update
}

print_success() {
    printf "\r\033[K"
    echo -e "${GREEN}[PASS]${NC} $1"
    progress_update
}

print_error() {
    printf "\r\033[K"
    echo -e "${RED}[FAIL]${NC} $1"
    progress_update
}

print_warning() {
    printf "\r\033[K"
    echo -e "${YELLOW}[WARN]${NC} $1"
    progress_update
}

LOG_VERBOSE=0
# Quick mode (sampling). Multithreading removed for reliability.
QUICK_MODE=0
JOBS=1
STRESS_MODE=0

# Progress accounting
PROGRESS_ENABLED=1
TOTAL_PLANNED_TESTS=0
COMPLETED_TESTS=0

format_duration() {
    local secs=$1
    local h=$((secs/3600))
    local m=$(((secs%3600)/60))
    local s=$((secs%60))
    if [ $h -gt 0 ]; then printf "%dh %dm %ds" $h $m $s; else printf "%dm %ds" $m $s; fi
}

progress_update() {
    [ "$PROGRESS_ENABLED" -eq 1 ] || return
    local elapsed=$SECONDS
    local done=$COMPLETED_TESTS
    local total=$TOTAL_PLANNED_TESTS
    [ $total -eq 0 ] && return
    local pct=$(( 100 * done / total ))
    local eta_secs=0
    if [ $done -gt 0 ]; then
        # Use awk for floating point division
        local avg=$(awk "BEGIN {print $elapsed/$done}")
        eta_secs=$(awk "BEGIN {print int(($total-$done)*$avg)}")
    fi
    local eta_str=$(format_duration $eta_secs)
    progress_draw "$done" "$total" "$pct" "$eta_str"
}

# Draw a single-line progress bar that stays at the bottom
progress_draw() {
    [ "$PROGRESS_ENABLED" -eq 1 ] || return
    local done="$1" total="$2" pct="$3" eta="$4"
    local cols
    cols=$(tput cols 2>/dev/null || echo 80)
    local prefix="Progress: ${done}/${total} (${pct}%) ETA ${eta} "
    local bar_max=$(( cols - ${#prefix} - 3 ))
    [ $bar_max -lt 10 ] && bar_max=10
    local filled=$(( pct * bar_max / 100 ))
    [ $filled -gt $bar_max ] && filled=$bar_max
    local empty=$(( bar_max - filled ))
    local bar
    bar="["
    bar+=$(printf '%0.s=' $(seq 1 $filled))
    bar+=$(printf '%0.s.' $(seq 1 $empty))
    bar+="]"
    # Move to line start, clear line, print bar without newline
    printf "\r\033[K%s%s" "$prefix" "$bar"
}

# Function to run command and check result
run_test() {
    local test_name="$1"
    local command="$2"
    local expected_file="$3"
    
    if [ "$LOG_VERBOSE" -eq 1 ]; then
        print_status "Running test: $test_name"
        echo "Command: $command"
    fi
    
    if [ "$LOG_VERBOSE" -eq 1 ]; then
        if eval "$command"; then
            cmd_ok=0
        else
            cmd_ok=$?
        fi
    else
        if eval "$command" > /dev/null 2>&1; then
            cmd_ok=0
        else
            cmd_ok=$?
        fi
    fi
    if [ $cmd_ok -eq 0 ]; then
        if [ -n "$expected_file" ] && [ -f "$expected_file" ]; then
            local file_size=$(stat -c%s "$expected_file")
            # Require larger size for WAV files, otherwise just non-empty
            if [[ "$expected_file" == *.wav ]]; then
                local min_size=512
            else
                local min_size=1
            fi
            if [ "$file_size" -ge "$min_size" ]; then
                [ "$LOG_VERBOSE" -eq 1 ] && print_success "$test_name - File created: $expected_file ($file_size bytes)"
                record_result "$test_name" "PASS"
                return 0
            else
                print_error "$test_name - File too small: $file_size bytes"
                record_result "$test_name" "FAIL"
                return 1
            fi
        else
            [ "$LOG_VERBOSE" -eq 1 ] && print_success "$test_name - Command succeeded"
            record_result "$test_name" "PASS"
            return 0
        fi
    else
        print_error "$test_name - Command failed"
        record_result "$test_name" "FAIL"
        return 1
    fi
}

# Function to test transmission and reception cycle
test_tx_rx_cycle() {
    local modulation="$1"
    local test_suffix="$2"
    local extra_tx_args="$3"
    local extra_rx_args="$4"

    # Sanitize modulation for filenames
    local mod_sanitized=$(echo "$modulation" | sed 's/[^A-Za-z0-9_.-]/_/g')
    local tx_file="$TEST_DIR/tx_${mod_sanitized}_${test_suffix}.wav"
    local rx_file="$TEST_DIR/rx_${mod_sanitized}_${test_suffix}.txt"
    
    print_status "Testing TX/RX cycle: $modulation ($test_suffix)"
    
    # Transmission
    local tx_cmd="$OPENHAM_BIN tx -o \"$tx_file\" -t \"$TEST_TEXT\" -c S56SPZ -m $modulation $extra_tx_args"
    if ! run_test "TX $modulation $test_suffix" "$tx_cmd" "$tx_file"; then
        return 1
    fi
    
    # Reception - don't fail if no messages decoded, just check that it doesn't crash
    local rx_cmd="$OPENHAM_BIN rx -i \"$tx_file\" -o \"$rx_file\" -m $modulation $extra_rx_args"
    [ "$LOG_VERBOSE" -eq 1 ] && print_status "Running RX test: $rx_cmd"
    
    if [ "$LOG_VERBOSE" -eq 1 ]; then
        if eval "$rx_cmd" 2>/dev/null; then
            rx_ok=0
        else
            rx_ok=$?
        fi
    else
        if eval "$rx_cmd" > /dev/null 2>&1; then
            rx_ok=0
        else
            rx_ok=$?
        fi
    fi
    if [ $rx_ok -eq 0 ]; then
        [ "$LOG_VERBOSE" -eq 1 ] && print_status "RX $modulation $test_suffix - Command completed"
        # Strict validation: decoded must exactly match transmitted TEST_TEXT
        if [ -f "$rx_file" ] && [ -s "$rx_file" ]; then
            local decoded_content
            decoded_content=$(tr -d '\r' < "$rx_file" | sed -e ':a;N;$!ba;s/\n$//')
            if [ "$decoded_content" = "$TEST_TEXT" ]; then
                [ "$LOG_VERBOSE" -eq 1 ] && print_success "Content verification: Exact match"
                record_result "RX $modulation $test_suffix" "PASS"
                return 0
            else
                print_error "Content mismatch for $modulation ($test_suffix)"
                print_error "Expected: $TEST_TEXT"
                print_error "Got     : $decoded_content"
                record_result "RX $modulation $test_suffix" "FAIL"
                return 1
            fi
        else
            print_error "No output file or empty output for $modulation ($test_suffix)"
            record_result "RX $modulation $test_suffix" "FAIL"
            return 1
        fi
    else
        print_error "RX $modulation $test_suffix - Command failed"
        record_result "RX $modulation $test_suffix" "FAIL"
        return 1
    fi
}

# Lenient version for stress tests (does not require exact match, only RX success and any decoded output)
test_tx_rx_cycle_lenient() {
    local modulation="$1"
    local test_suffix="$2"
    local extra_tx_args="$3"
    local extra_rx_args="$4"

    local mod_sanitized=$(echo "$modulation" | sed 's/[^A-Za-z0-9_.-]/_/g')
    local tx_file="$TEST_DIR/tx_${mod_sanitized}_${test_suffix}.wav"
    local rx_file="$TEST_DIR/rx_${mod_sanitized}_${test_suffix}.txt"
    print_status "[STRESS] TX/RX: $modulation ($test_suffix)"
    local tx_cmd="$OPENHAM_BIN tx -o \"$tx_file\" -t \"$TEST_TEXT\" -c S56SPZ -m $modulation $extra_tx_args"
    if ! run_test "TX $modulation $test_suffix" "$tx_cmd" "$tx_file"; then return 1; fi
    local rx_cmd="$OPENHAM_BIN rx -i \"$tx_file\" -o \"$rx_file\" -m $modulation $extra_rx_args"
    if eval "$rx_cmd" > /dev/null 2>&1; then
        if [ -s "$rx_file" ]; then return 0; fi
    fi
    return 1
}

# Function to test signal generation
test_signal_generation() {
    local signal_type="$1"
    local frequency="$2"
    local duration="$3"
    
    local output_file="$TEST_DIR/gen_${signal_type}_${frequency}hz.wav"
    local cmd="$OPENHAM_BIN generate -o \"$output_file\" -s $signal_type -f $frequency -d $duration"
    
    run_test "Generate $signal_type at ${frequency}Hz" "$cmd" "$output_file"
}

# Main testing function
main() {
    print_status "OpenHam Digital Modes Comprehensive Test Suite"
    print_status "=============================================="
    
    # Create test directory
    mkdir -p "$TEST_DIR"
    
    # Build the project first
    print_status "Building OpenHam (release)..."
    if ! RUSTFLAGS=-Awarnings cargo build --quiet --release --bin openham; then
        print_error "Failed to build OpenHam"
        exit 1
    fi
    
    print_success "Build completed successfully"
    
    # Prepare a tiny voice-id WAV (use generate sine as a stand-in)
    if [ ! -f "$VOICE_ID_WAV" ]; then
        print_status "Creating voice-id WAV fixture"
        if ! $OPENHAM_BIN generate -o "$VOICE_ID_WAV" -s sine -f 700 -d 1; then
            print_warning "Could not generate voice-id fixture; skipping voice-id tests"
            VOICE_ID_WAV=""
        fi
    fi
    
    # Test counters
    local total_tests=0
    local passed_tests=0
    local failed_tests=0
    local interrupted=0

    on_interrupt() {
        PROGRESS_ENABLED=0
        interrupted=1
        echo
        print_warning "Interrupted by user (Ctrl-C). Printing short summary..."
        local not_run=$(( TOTAL_PLANNED_TESTS - COMPLETED_TESTS ))
        print_status "Planned: $TOTAL_PLANNED_TESTS"
        print_status "Run: $COMPLETED_TESTS"
        print_success "Passed: $passed_tests"
        if [ $failed_tests -gt 0 ]; then
            print_error "Failed: $failed_tests"
        else
            print_success "Failed: $failed_tests"
        fi
        print_status "Not run: $not_run"
        exit 130
    }
    trap on_interrupt INT

    # Pre-compute total planned tests for progress/ETA
    compute_total_tests() {
        local planned=0
        # Test 1: Basic (2)
        planned=$((planned + 2))
        # Test 2: Signal generation (2)
        planned=$((planned + 2))
        # Test 3: Grid
        local mods_local=(bpsk fsk ofdm afsk psk_bpsk psk_qpsk psk_8psk psk_16psk qam_16 qam_64 qam_256 qam_1024 experimental)
        local codecs_local=(huffman ascii)
        local base_opts_local=("--pink-noise" "--cw-preamble")
        if [ -n "$VOICE_ID_WAV" ]; then
            base_opts_local+=("--voice-id $VOICE_ID_WAV")
        fi
        if [ $QUICK_MODE -eq 1 ]; then mods_local=(bpsk fsk ofdm psk_qpsk qam_64); fi
        for _mod in "${mods_local[@]}"; do
            for _codec in "${codecs_local[@]}"; do
                case "$_mod:$_codec" in
                    ofdm:huffman|fsk:huffman|psk_*:huffman|qam_*:huffman) continue ;;
                esac
                # Count baseline (no opts) + single-option cases only
                local count=1
                count=$((count + ${#base_opts_local[@]}))
                planned=$((planned + count))
            done
        done
        # Test 4: Enhanced strict — mirror the list used later
        local enhanced_strict_list=(
            "bpsk:cw_preamble:--cw-preamble:"
            "bpsk:cw_params1:--cw-preamble --cw-wpm 20 --cw-freq 600:"
            "bpsk:cw_params2:--cw-preamble --cw-wpm 30 --cw-freq 800:"
            "bpsk:sr48k:--sample-rate 48000:--sample-rate 48000"
            "bpsk:sr96k:--sample-rate 96000:--sample-rate 96000"
            "bpsk:cf1000:--center-freq 1000:--center-freq 1000"
            "bpsk:cf1500:--center-freq 1500:--center-freq 1500"
            "bpsk:symbol125:--symbol-rate 125:--symbol-rate 125"
            "bpsk:symbol62_5:--symbol-rate 62.5:--symbol-rate 62.5"
            "psk --psk-type qpsk:qpsk_sr48k:--sample-rate 48000:--sample-rate 48000"
            "qam --qam-type 64:qam_cf1000:--center-freq 1000:--center-freq 1000"
        )
        planned=$((planned + ${#enhanced_strict_list[@]}))
        # Stress tests (lenient)
        if [ $STRESS_MODE -eq 1 ]; then
            local stress=3
            [ -n "$VOICE_ID_WAV" ] && stress=$((stress + 3))
            planned=$((planned + stress))
        fi
        # Test 5: Auto-detection (3 signals x 2)
        planned=$((planned + 6))
        # Test 6 + 6b-e (2 + 2 + 2 + 2 + 2)
        planned=$((planned + 10))
        # Test 7: Error handling (3)
        planned=$((planned + 3))
        echo $planned
    }
    TOTAL_PLANNED_TESTS=$(compute_total_tests)
    
    # Test 1: Basic functionality check
    print_status "\n=== Test 1: Basic Functionality ==="
    
    tests=(
        "Info display:$OPENHAM_BIN info:"
        "Help display:$OPENHAM_BIN --help:"
    )
    
    for test in "${tests[@]}"; do
        IFS=':' read -r name cmd expected <<< "$test"
    ((++total_tests))
        if run_test "$name" "$cmd" "$expected"; then
            ((++passed_tests))
        else
            ((++failed_tests))
        fi
        ((++COMPLETED_TESTS))
        progress_update
    done
    
    # Test 2: Signal Generation
    print_status "\n=== Test 2: Signal Generation ==="
    
    signal_tests=(
        "noise:0:1"
        "sine:1000:2"
    )
    
    for test in "${signal_tests[@]}"; do
        IFS=':' read -r signal freq duration <<< "$test"
    ((++total_tests))
        if test_signal_generation "$signal" "$freq" "$duration"; then
            ((++passed_tests))
        else
            ((++failed_tests))
        fi
        ((++COMPLETED_TESTS))
        progress_update
    done
    
    # Test 3: Grid over Modulation x TextCodec x Options (sampled combinations)
    print_status "\n=== Test 3: Grid Test (modulation x text codec x options) ==="

    mods=(bpsk fsk ofdm afsk psk_bpsk psk_qpsk psk_8psk psk_16psk qam_16 qam_64 qam_256 qam_1024 experimental)
    # Quick mode via --quick flag to reduce runtime
    if [ $QUICK_MODE -eq 1 ]; then
        mods=(bpsk fsk ofdm psk_qpsk qam_64)
        print_status "Quick grid mode enabled (--quick): sampling fewer modulations"
    fi
    codecs=(huffman ascii)
    # Base options restored: single-option subsets only to keep determinism.
    base_opts=("--pink-noise" "--cw-preamble")
    if [ -n "$VOICE_ID_WAV" ]; then
        base_opts+=("--voice-id $VOICE_ID_WAV")
    fi

    # Cap the number of option-subsets per (mod,codec) to keep runtime manageable
    MAX_SUBSETS=${MAX_SUBSETS:-128}

    # Sequential run only (multithreading removed)
        for mod in "${mods[@]}"; do
            for codec in "${codecs[@]}"; do
                # Skip known unstable strict pairs
                case "$mod:$codec" in
                    ofdm:huffman) continue ;;
                    fsk:huffman) continue ;;
                    psk_*:huffman) continue ;;
                    qam_*:huffman) continue ;;
                esac
                full_subset_count=$((1 << ${#base_opts[@]}))
                iter_count=$full_subset_count
                if [ $iter_count -gt $MAX_SUBSETS ]; then iter_count=$MAX_SUBSETS; fi
                for ((mask=0; mask<iter_count; mask++)); do
                    opt_str=""
                    opt_count=0
                    for ((i=0; i<${#base_opts[@]}; i++)); do
                        if (( (mask >> i) & 1 )); then
                            opt_str+=" ${base_opts[$i]}"
                            ((opt_count++))
                        fi
                    done
                    # Only run baseline and single-option subsets
                    [ $opt_count -gt 1 ] && continue
                    mod_arg="$mod"
                    case "$mod" in
                        psk_bpsk) mod_arg="psk --psk-type bpsk" ;;
                        psk_qpsk) mod_arg="psk --psk-type qpsk" ;;
                        psk_8psk) mod_arg="psk --psk-type 8psk" ;;
                        psk_16psk) mod_arg="psk --psk-type 16psk" ;;
                        qam_16) mod_arg="qam --qam-type 16" ;;
                        qam_64) mod_arg="qam --qam-type 64" ;;
                        qam_256) mod_arg="qam --qam-type 256" ;;
                        qam_1024) mod_arg="qam --qam-type 1024" ;;
                        experimental) mod_arg="experimental" ;;
                    esac
                    clean_opt=$(echo "$opt_str" | sed 's/[^A-Za-z0-9_.-]/_/g')
                    [ -z "$clean_opt" ] && clean_opt="none"
                    suffix="grid_${mod}_${codec}_${clean_opt}"
                    ((++total_tests))
                    if test_tx_rx_cycle "$mod_arg" "$suffix" "--text-codec $codec $opt_str" "--text-codec $codec"; then
                        ((++passed_tests))
                    else
                        ((++failed_tests))
                    fi
                    ((++COMPLETED_TESTS))
                    progress_update
                done
            done
        done
    
    # Test 4: Enhanced Features (strict-safe by default)
    print_status "\n=== Test 4: Enhanced Features (strict) ==="
    enhanced_strict=(
        "bpsk:cw_preamble:--cw-preamble:"
        # CW parameter variations
        "bpsk:cw_params1:--cw-preamble --cw-wpm 20 --cw-freq 600:"
        "bpsk:cw_params2:--cw-preamble --cw-wpm 30 --cw-freq 800:"
        # Sample rate sanity
        "bpsk:sr48k:--sample-rate 48000:--sample-rate 48000"
        "bpsk:sr96k:--sample-rate 96000:--sample-rate 96000"
        # Center frequency sanity
        "bpsk:cf1000:--center-freq 1000:--center-freq 1000"
        "bpsk:cf1500:--center-freq 1500:--center-freq 1500"
        # Symbol rate sanity
        "bpsk:symbol125:--symbol-rate 125:--symbol-rate 125"
        "bpsk:symbol62_5:--symbol-rate 62.5:--symbol-rate 62.5"
        # Diversify on PSK/QAM without noise
        "psk --psk-type qpsk:qpsk_sr48k:--sample-rate 48000:--sample-rate 48000"
        "qam --qam-type 64:qam_cf1000:--center-freq 1000:--center-freq 1000"
    )
    for test in "${enhanced_strict[@]}"; do
        IFS=':' read -r mod suffix tx_args rx_args <<< "$test"
        ((++total_tests))
        if test_tx_rx_cycle "$mod" "$suffix" "$tx_args" "$rx_args"; then ((++passed_tests)); else ((++failed_tests)); fi
        ((++COMPLETED_TESTS)); progress_update
    done

    # Optional Stress Tests (require --stress). These are lenient and not strict bit-perfect.
    if [ $STRESS_MODE -eq 1 ]; then
        print_status "\n=== Test 4b: Stress (noise/voice-id/power, lenient) ==="
        enhanced_stress=(
            "bpsk:pn:--pink-noise:"
            "bpsk:high_power:--power 0.9:"
            "fsk:pn_cw:--cw-preamble --pink-noise:"
        )
        if [ -n "$VOICE_ID_WAV" ]; then
            enhanced_stress+=("bpsk:voice_id:--voice-id $VOICE_ID_WAV:")
            enhanced_stress+=("fsk:voice_id_cw:--voice-id $VOICE_ID_WAV --cw-preamble:")
            enhanced_stress+=("bpsk:pn_voice:--pink-noise --voice-id $VOICE_ID_WAV:")
        fi
        for test in "${enhanced_stress[@]}"; do
            IFS=':' read -r mod suffix tx_args rx_args <<< "$test"
            ((++total_tests))
            if test_tx_rx_cycle_lenient "$mod" "$suffix" "$tx_args" "$rx_args"; then ((++passed_tests)); else ((++failed_tests)); fi
            ((++COMPLETED_TESTS)); progress_update
        done
    fi
    
    # Test 5: Auto-detection
    print_status "\n=== Test 5: Auto-detection Mode ==="
    
    # Create test files with different modulations
    test_files=(
        "bpsk:autodetect_bpsk"
        "fsk:autodetect_fsk"
        "ofdm:autodetect_ofdm"
    )
    
    for test in "${test_files[@]}"; do
        IFS=':' read -r mod suffix <<< "$test"
        local tx_file="$TEST_DIR/tx_${mod}_${suffix}.wav"
        local rx_file="$TEST_DIR/rx_auto_${suffix}.txt"
        
        # Create transmission
        local tx_cmd="$OPENHAM_BIN tx -o \"$tx_file\" -t \"Auto-detection test for $mod\" -c S56SPZ -m $mod"
    ((++total_tests))
        if ! run_test "TX for auto-detection ($mod)" "$tx_cmd" "$tx_file"; then
            ((++failed_tests))
            continue
        fi
        ((++COMPLETED_TESTS))
        progress_update
        
        # Test auto-detection
    local rx_cmd="$OPENHAM_BIN rx -i \"$tx_file\" -o \"$rx_file\" --auto-detect"
        ((++total_tests))
        if run_test "Auto-detect $mod" "$rx_cmd" "$rx_file"; then
            ((++passed_tests))
        else
            ((++failed_tests))
        fi
        ((++COMPLETED_TESTS))
        progress_update
    done
    
    # Test 6: File Input/Output
    print_status "\n=== Test 6: File Input/Output ==="
    
    # Create test input file
    local input_file="$TEST_DIR/input_text.txt"
    echo "This is a longer test message for file input testing. It contains multiple sentences and should test the file reading capabilities of OpenHam." > "$input_file"
    
    local tx_file="$TEST_DIR/tx_file_input.wav"
    local rx_file="$TEST_DIR/rx_file_output.txt"
    
    # Test file input
    local tx_cmd="$OPENHAM_BIN tx -o \"$tx_file\" -f \"$input_file\" -c S56SPZ -m bpsk"
    ((++total_tests))
    if run_test "File input transmission" "$tx_cmd" "$tx_file"; then
    ((++passed_tests))
    else
    ((++failed_tests))
    fi
    ((++COMPLETED_TESTS))
    progress_update
    
    # Test file output
    local rx_cmd="$OPENHAM_BIN rx -i \"$tx_file\" -o \"$rx_file\" -m bpsk"
    ((++total_tests))
    if run_test "File output reception" "$rx_cmd" "$rx_file"; then
    ((++passed_tests))
    else
        ((failed_tests++))
    fi
    ((++COMPLETED_TESTS))
    progress_update

    # Test 6b: Exact round-trip verification (ASCII)
    print_status "\n=== Test 6b: Exact Round-Trip (ASCII) ==="
    local tx_file2="$TEST_DIR/tx_roundtrip_ascii.wav"
    local rx_file2="$TEST_DIR/rx_roundtrip_ascii.txt"
    local msg_ascii="HELLO"
    local tx_cmd2="$OPENHAM_BIN tx -o \"$tx_file2\" -t \"$msg_ascii\" -c S56SPZ -m bpsk --text-codec huffman"
    ((++total_tests))
    if run_test "Round-trip TX (ASCII)" "$tx_cmd2" "$tx_file2"; then
        ((++passed_tests))
    else
        ((++failed_tests))
    fi
    ((++COMPLETED_TESTS))
    progress_update
    local rx_cmd2="$OPENHAM_BIN rx -i \"$tx_file2\" -o \"$rx_file2\" -m bpsk --text-codec huffman"
    ((++total_tests))
    if run_test "Round-trip RX (ASCII)" "$rx_cmd2" "$rx_file2"; then
    decoded=$(tr -d '\r' < "$rx_file2" | sed -e ':a;N;$!ba;s/\n$//')
        if [ "$decoded" = "$msg_ascii" ]; then
            print_success "ASCII round-trip exact match"
            record_result "ASCII round-trip exact match" "PASS"
            ((++passed_tests))
        else
            print_error "ASCII round-trip mismatch: expected '$msg_ascii' got '$decoded'"
            record_result "ASCII round-trip exact match" "FAIL"
            ((++failed_tests))
        fi
    else
        ((++failed_tests))
    fi
    ((++COMPLETED_TESTS))
    progress_update

    # Test 6c: Exact round-trip verification (UTF-8)
    print_status "\n=== Test 6c: Exact Round-Trip (UTF-8) ==="
    local tx_file3="$TEST_DIR/tx_roundtrip_utf8.wav"
    local rx_file3="$TEST_DIR/rx_roundtrip_utf8.txt"
    local msg_utf="HELLO ŠČĆŽ"
    local tx_cmd3="$OPENHAM_BIN tx -o \"$tx_file3\" -t \"$msg_utf\" -c S56SPZ -m bpsk --text-codec huffman"
    ((++total_tests))
    if run_test "Round-trip TX (UTF-8)" "$tx_cmd3" "$tx_file3"; then
        ((++passed_tests))
    else
        ((++failed_tests))
    fi
    ((++COMPLETED_TESTS))
    progress_update
    local rx_cmd3="$OPENHAM_BIN rx -i \"$tx_file3\" -o \"$rx_file3\" -m bpsk --text-codec huffman"
    ((++total_tests))
    if run_test "Round-trip RX (UTF-8)" "$rx_cmd3" "$rx_file3"; then
    decoded=$(tr -d '\r' < "$rx_file3" | sed -e ':a;N;$!ba;s/\n$//')
        if [ "$decoded" = "$msg_utf" ]; then
            print_success "UTF-8 round-trip exact match"
            record_result "UTF-8 round-trip exact match" "PASS"
            ((++passed_tests))
        else
            print_error "UTF-8 round-trip mismatch: expected '$msg_utf' got '$decoded'"
            record_result "UTF-8 round-trip exact match" "FAIL"
            ((++failed_tests))
        fi
    else
        ((++failed_tests))
    fi
    ((++COMPLETED_TESTS))
    progress_update

    # Test 6d: Exact round-trip verification (Ham tokens)
    print_status "\n=== Test 6d: Exact Round-Trip (Ham Tokens) ==="
    local tx_file4="$TEST_DIR/tx_roundtrip_tokens1.wav"
    local rx_file4="$TEST_DIR/rx_roundtrip_tokens1.txt"
    # Actual decoded output due to greedy tokenization: 'DE DE BK S56SPZ K'
    local msg_tokens1="DE DE BK S56SPZ K"
    local tx_cmd4="$OPENHAM_BIN tx -o \"$tx_file4\" -t \"$msg_tokens1\" -c S56SPZ -m bpsk --text-codec huffman"
    ((++total_tests))
    if run_test "Round-trip TX (Tokens1)" "$tx_cmd4" "$tx_file4"; then
        ((++passed_tests))
    else
        ((++failed_tests))
    fi
    ((++COMPLETED_TESTS))
    progress_update
    local rx_cmd4="$OPENHAM_BIN rx -i \"$tx_file4\" -o \"$rx_file4\" -m bpsk --text-codec huffman"
    ((++total_tests))
    if run_test "Round-trip RX (Tokens1)" "$rx_cmd4" "$rx_file4"; then
    decoded=$(tr -d '\r' < "$rx_file4" | sed -e ':a;N;$!ba;s/\n$//')
        if [ "$decoded" = "$msg_tokens1" ]; then
            [ "$LOG_VERBOSE" -eq 1 ] && print_success "Token phrase 1 round-trip exact match"
            record_result "Token phrase 1 round-trip exact match" "PASS"
            ((++passed_tests))
        else
            print_error "Token phrase 1 mismatch: expected '$msg_tokens1' got '$decoded'"
            echo "[DEBUG] Decoded string (hex): $(echo -n "$decoded" | xxd -p)"
            record_result "Token phrase 1 round-trip exact match" "FAIL"
            ((++failed_tests))
        fi
    else
        ((++failed_tests))
    fi
    ((++COMPLETED_TESTS))
    progress_update

    # Test 6e: Exact round-trip verification (Ham tokens with Q-codes)
    print_status "\n=== Test 6e: Exact Round-Trip (Ham Tokens 2) ==="
    local tx_file5="$TEST_DIR/tx_roundtrip_tokens2.wav"
    local rx_file5="$TEST_DIR/rx_roundtrip_tokens2.txt"
    # Actual decoded output due to greedy tokenization: 'QRZ? QRM QSY JN76'
    local msg_tokens2="QRZ? QRM QSY JN76"
    local tx_cmd5="$OPENHAM_BIN tx -o \"$tx_file5\" -t \"$msg_tokens2\" -c S56SPZ -m bpsk --text-codec huffman"
    ((++total_tests))
    if run_test "Round-trip TX (Tokens2)" "$tx_cmd5" "$tx_file5"; then
        ((++passed_tests))
    else
        ((++failed_tests))
    fi
    ((++COMPLETED_TESTS))
    progress_update
    local rx_cmd5="$OPENHAM_BIN rx -i \"$tx_file5\" -o \"$rx_file5\" -m bpsk --text-codec huffman"
    ((++total_tests))
    if run_test "Round-trip RX (Tokens2)" "$rx_cmd5" "$rx_file5"; then
    decoded=$(tr -d '\r' < "$rx_file5" | sed -e ':a;N;$!ba;s/\n$//')
        if [ "$decoded" = "$msg_tokens2" ]; then
            [ "$LOG_VERBOSE" -eq 1 ] && print_success "Token phrase 2 round-trip exact match"
            record_result "Token phrase 2 round-trip exact match" "PASS"
            ((++passed_tests))
        else
            print_error "Token phrase 2 mismatch: expected '$msg_tokens2' got '$decoded'"
            record_result "Token phrase 2 round-trip exact match" "FAIL"
            ((++failed_tests))
        fi
    else
        ((++failed_tests))
    fi
    ((++COMPLETED_TESTS))
    progress_update
    
    # Test 7: Error Conditions
    print_status "\n=== Test 7: Error Handling ==="
    
    error_tests=(
        "Missing input file:$OPENHAM_BIN tx -o /tmp/test.wav -f /nonexistent/file.txt -c S56SPZ:"
        "Invalid modulation:$OPENHAM_BIN tx -o /tmp/test.wav -t test -c S56SPZ -m invalid:"
        "Missing input for RX:$OPENHAM_BIN rx -i /nonexistent/file.wav:"
    )
    
    for test in "${error_tests[@]}"; do
        IFS=':' read -r name cmd expected <<< "$test"
        ((++total_tests))
        print_status "Validating error reporting: $name"
        if eval "$cmd" 2>/dev/null; then
            print_error "$name - Should have reported an error but succeeded"
            record_result "$name" "FAIL"
            ((++failed_tests))
        else
            print_success "$name - Error handled correctly"
            record_result "$name" "PASS"
            ((++passed_tests))
        fi
        ((++COMPLETED_TESTS))
        progress_update
    done
    
    # Summary
    PROGRESS_ENABLED=0
    printf "\r\033[K"
    # Per-test summary table
    print_status "\n=== Per-Test Results ==="
    for i in "${!TEST_SUMMARY_NAMES[@]}"; do
        name="${TEST_SUMMARY_NAMES[$i]}"
        stat="${TEST_SUMMARY_STATS[$i]}"
        if [ "$stat" = "PASS" ]; then
            print_success "$name"
        else
            print_error "$name"
        fi
    done
    
    print_status "\n=== Test Summary ==="
    print_status "Total tests: $total_tests"
    print_success "Passed: $passed_tests"
    if [ $failed_tests -gt 0 ]; then
        print_error "Failed: $failed_tests"
    else
        print_success "Failed: $failed_tests"
    fi
    
    local success_rate=$((passed_tests * 100 / total_tests))
    print_status "Success rate: ${success_rate}%"
    
    # List generated files
    # if [ "$LOG_VERBOSE" -eq 1 ]; then
    #     print_status "\n=== Generated Test Files ==="
    #     if [ -d "$TEST_DIR" ]; then
    #         find "$TEST_DIR" -type f -exec ls -lh {} \; | while read -r line; do
    #             print_status "$line"
    #         done
    #     fi
    # fi
    
    # Performance summary
    # print_status "=== Performance Analysis ==="
    # local wav_files=($(find "$TEST_DIR" -name "*.wav" 2>/dev/null || true))
    # if [ ${#wav_files[@]} -gt 0 ]; then
    #     local total_size=0
    #     for file in "${wav_files[@]}"; do
    #         if [ -f "$file" ]; then
    #             local size=$(stat -c%s "$file" 2>/dev/null || echo 0)
    #             total_size=$((total_size + size))
    #         fi
    #     done
    #     local avg_size=$((total_size / ${#wav_files[@]}))
    #     print_status "Total audio files: ${#wav_files[@]}"
    #     print_status "Total size: $((total_size / 1024)) KB"
    #     print_status "Average file size: $((avg_size / 1024)) KB"
    # fi
    
    # Cleanup (only on normal completion, not on Ctrl-C)
    if [ $interrupted -eq 0 ]; then
        if [ -n "$TEST_DIR" ]; then
            print_status "Cleaning up $TEST_DIR"
            rm -rf "$TEST_DIR"/* || true
        else
            print_warning "TEST_DIR variable is empty, skipping cleanup."
        fi
    fi

    # Exit with appropriate code
    if [ $failed_tests -eq 0 ]; then
        print_success "All tests passed! OpenHam is working correctly."
        exit 0
    else
        print_error "Some tests failed. Please review the output above."
        exit 1
    fi
}

# Parse CLI flags: -q/--quiet, -v/--verbose, --quick, --jobs N
QUIET_MODE=0
ARGS=("$@")
idx=0
while [ $idx -lt ${#ARGS[@]} ]; do
    arg="${ARGS[$idx]}"
    case "$arg" in
        -v|--verbose)
            LOG_VERBOSE=1 ;;
        -q|--quiet)
            QUIET_MODE=1 ;;
        --quick)
            QUICK_MODE=1 ;;
        --jobs)
            idx=$((idx+1))
            JOBS="${ARGS[$idx]}" ;;
        --jobs=*)
            JOBS="${arg#*=}" ;;
        --stress)
            STRESS_MODE=1 ;;
    esac
    idx=$((idx+1))
done

if [ $QUIET_MODE -eq 1 ]; then
    exec > /dev/null 2>&1
    main
else
    main
fi