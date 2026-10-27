#!/usr/bin/env bash
set -euo pipefail
test_apk=${1:?usage: android_persistence.sh TEST_APK OUTPUT_DIR}
out=${2:?usage: android_persistence.sh TEST_APK OUTPUT_DIR}
mkdir -p "$out"
cleanup() { adb logcat -d -v threadtime > "$out/logcat.txt" 2>&1 || true; }
trap cleanup EXIT
adb install -r "$test_apk"
adb logcat -c
adb shell am instrument -w -r com.jude.tetrisnesrust.test/androidx.test.runner.AndroidJUnitRunner | tee "$out/instrumentation.txt"
grep -Eq '^OK \([0-9]+ tests?\)' "$out/instrumentation.txt"
