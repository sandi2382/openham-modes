#!/usr/bin/env bash
#
# Comprehensive test runner for OpenHam digital modes.
#
# Runs the full Rust test suite (cargo test --workspace) and reports its
# pass/fail count, then exercises the actual `openham` CLI end-to-end: signal
# generation, round-trips for the working modes and text encodings, live
# frame acquisition (a transmission that starts mid-stream), and error handling.
#
# Portable across macOS and Linux. No arguments.

set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

OPENHAM_BIN="./target/release/openham"

# Modes that round-trip end-to-end (file-aligned).
WORK_MODES="bpsk fsk afsk psk4 ofdm64"
# Modes that also acquire a frame starting at an arbitrary offset (live audio).
LIVE_MODES="bpsk fsk afsk ofdm64"

MSG="CQ DE S56SPZ TEST 123 K QRZ 73"

# --- output helpers -----------------------------------------------------------
if [ -t 1 ]; then
    GREEN='\033[0;32m'; RED='\033[0;31m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'; NC='\033[0m'
else
    GREEN=''; RED=''; YELLOW=''; BLUE=''; NC=''
fi
PASS=0; FAIL=0
ok()      { PASS=$((PASS + 1)); printf "${GREEN}[PASS]${NC} %s\n" "$1"; }
bad()     { FAIL=$((FAIL + 1)); printf "${RED}[FAIL]${NC} %s\n" "$1"; }
info()    { printf "${BLUE}[INFO]${NC} %s\n" "$1"; }
section() { printf "\n${YELLOW}=== %s ===${NC}\n" "$1"; }

filesize() { wc -c < "$1" | tr -d ' '; }   # portable file size (BSD + GNU)

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

HAVE_PY=0
command -v python3 >/dev/null 2>&1 && HAVE_PY=1

# Round-trip a message through tx/rx for a mode+encoding; echoes the decoded text.
roundtrip() {
    local mode="$1" enc="$2" msg="$3"
    if ! $OPENHAM_BIN tx -o "$TMP/t.wav" -t "$msg" -c S56SPZ -m "$mode" --encoding "$enc" >/dev/null 2>&1; then
        echo "__TX_FAILED__"; return
    fi
    $OPENHAM_BIN rx -i "$TMP/t.wav" -m "$mode" --encoding "$enc" 2>/dev/null \
        | sed -nE 's/^  (.*)/\1/p' | tail -1
}

# =============================================================================
section "Build (release)"
if cargo build --release --quiet 2>"$TMP/build.err"; then
    ok "cargo build --release"
else
    bad "cargo build --release"
    cat "$TMP/build.err"
    info "Build failed; aborting."
    exit 1
fi

# =============================================================================
section "Rust test suite (cargo test --workspace)"
cargo test --workspace 2>&1 | tee "$TMP/cargo.out" \
    | grep -E "Running|test result:|FAILED|error\[" || true
RUST_PASS=$(grep -E "test result:" "$TMP/cargo.out" | awk '{s += $4} END {print s + 0}')
RUST_FAIL=$(grep -E "test result:" "$TMP/cargo.out" | awk '{s += $6} END {print s + 0}')
RUST_IGN=$(grep -E "test result:"  "$TMP/cargo.out" | awk '{s += $8} END {print s + 0}')
info "Rust tests: ${RUST_PASS} passed, ${RUST_FAIL} failed, ${RUST_IGN} ignored"
if [ "$RUST_FAIL" -eq 0 ]; then
    ok "cargo test --workspace (${RUST_PASS} passed, ${RUST_IGN} ignored)"
else
    bad "cargo test --workspace (${RUST_FAIL} failed)"
fi

# =============================================================================
section "Basic functionality"
$OPENHAM_BIN info    >/dev/null 2>&1 && ok "openham info"   || bad "openham info"
$OPENHAM_BIN --help  >/dev/null 2>&1 && ok "openham --help" || bad "openham --help"

# =============================================================================
section "Signal generation"
for sig in sine noise sweep two-tone; do
    if $OPENHAM_BIN generate -o "$TMP/g.wav" -s "$sig" --frequency 1000 -d 1 >/dev/null 2>&1 \
        && [ "$(filesize "$TMP/g.wav")" -gt 1000 ]; then
        ok "generate $sig"
    else
        bad "generate $sig"
    fi
done

# =============================================================================
section "End-to-end round-trip (working modes x encodings)"
for mode in $WORK_MODES; do
    for enc in huffman ascii; do
        got="$(roundtrip "$mode" "$enc" "$MSG")"
        if [ "$got" = "$MSG" ]; then
            ok "$mode / $enc round-trip"
        else
            bad "$mode / $enc round-trip (got: '$got')"
        fi
    done
done

# =============================================================================
section "Encoding fidelity"
UMSG="Pozdrav SCCZ de S56SPZ 73"
got="$(roundtrip bpsk utf8 "$UMSG")"
[ "$got" = "$UMSG" ] && ok "utf8 round-trip" || bad "utf8 round-trip (got: '$got')"

# =============================================================================
section "Live acquisition (transmission starts mid-stream)"
if [ "$HAVE_PY" -eq 1 ]; then
    AMSG="CQ DE S56SPZ LIVE K"
    for mode in $LIVE_MODES; do
        $OPENHAM_BIN tx -o "$TMP/tx.wav" -t "$AMSG" -c S56SPZ -m "$mode" --encoding huffman >/dev/null 2>&1
        # Prepend ~3000 samples of lead-in noise to simulate a live capture.
        python3 - "$TMP/tx.wav" "$TMP/off.wav" <<'PY'
import sys, wave, struct, random
src, dst = sys.argv[1], sys.argv[2]
w = wave.open(src, 'rb'); p = w.getparams(); fr = w.readframes(w.getnframes()); w.close()
lead = b''.join(struct.pack('<h', random.randint(-250, 250)) for _ in range(3000))
o = wave.open(dst, 'wb'); o.setparams(p); o.writeframes(lead + fr); o.close()
PY
        got="$($OPENHAM_BIN rx -i "$TMP/off.wav" -m "$mode" 2>/dev/null | sed -nE 's/^  (.*)/\1/p' | tail -1)"
        [ "$got" = "$AMSG" ] && ok "$mode acquires at arbitrary offset" \
            || bad "$mode arbitrary-offset acquisition (got: '$got')"
    done
else
    info "python3 not found; skipping live-acquisition tests"
fi

# =============================================================================
section "Error handling"
$OPENHAM_BIN rx -i "$TMP/nope.wav" -m bpsk >/dev/null 2>&1 \
    && bad "missing input rejected" || ok "missing input rejected"
$OPENHAM_BIN tx -o "$TMP/x.wav" -t "x" -c S56SPZ -m notamode >/dev/null 2>&1 \
    && bad "invalid modulation rejected" || ok "invalid modulation rejected"

# =============================================================================
section "Summary"
info "CLI / build checks: ${PASS} passed, ${FAIL} failed"
info "Rust tests:         ${RUST_PASS} passed, ${RUST_FAIL} failed, ${RUST_IGN} ignored"
if [ "$FAIL" -eq 0 ] && [ "$RUST_FAIL" -eq 0 ]; then
    printf "${GREEN}ALL GREEN — %d Rust tests + %d CLI checks passed${NC}\n" "$RUST_PASS" "$PASS"
    exit 0
else
    printf "${RED}FAILURES — %d CLI, %d Rust${NC}\n" "$FAIL" "$RUST_FAIL"
    exit 1
fi
