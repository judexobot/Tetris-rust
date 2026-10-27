#!/usr/bin/env bash
# Run inside the already-booted CI emulator. The old release is diagnostic;
# the current APK and its persistence test are mandatory.
set -euo pipefail
if tools/tests/android_smoke.sh target/baseline/*.apk target/baseline-smoke; then
  echo 'v0.2.2 smoke: PASS' | tee target/baseline-result.txt
else
  echo 'v0.2.2 smoke: FAIL (see captured Logcat)' | tee target/baseline-result.txt
fi
tools/tests/android_smoke.sh android/app/build/outputs/apk/release/app-release.apk target/android-smoke
tools/tests/android_persistence.sh android/app/build/outputs/apk/androidTest/release/app-release-androidTest.apk target/android-persistence
