#!/usr/bin/env bash

set -euo pipefail

if [[ $# -lt 1 ]]; then
    echo "Usage: $0 <watch-file>" >&2
    exit 1
fi

WATCH_FILE="$1"
FUZZ_CMD=(cargo +nightly fuzz run --release fuzz-native --)

get_mtime() {
    stat -c '%Y' "$1" 2>/dev/null || echo "0"
}

run_fuzzer() {
    "${FUZZ_CMD[@]}" &
    FUZZ_PID=$!
    echo "Started fuzzer (PID $FUZZ_PID)"
}

cleanup() {
    if [[ -n "${FUZZ_PID:-}" ]] && kill -0 "$FUZZ_PID" 2>/dev/null; then
        echo "Killing fuzzer (PID $FUZZ_PID)"
        kill "$FUZZ_PID"
        wait "$FUZZ_PID" 2>/dev/null || true
    fi
    exit 0
}

trap cleanup SIGINT SIGTERM

LAST_MTIME=$(get_mtime "$WATCH_FILE")
run_fuzzer

while true; do
    sleep 1

    if ! kill -0 "$FUZZ_PID" 2>/dev/null; then
        echo "Fuzzer exited unexpectedly, restarting..."
        run_fuzzer
        continue
    fi

    CURRENT_MTIME=$(get_mtime "$WATCH_FILE")
    if [[ "$CURRENT_MTIME" != "$LAST_MTIME" ]]; then
        echo "$WATCH_FILE changed, restarting fuzzer..."
        kill "$FUZZ_PID"
        wait "$FUZZ_PID" 2>/dev/null || true
        LAST_MTIME="$CURRENT_MTIME"
        run_fuzzer
    fi
done
