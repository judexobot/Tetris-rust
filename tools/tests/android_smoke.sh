#!/usr/bin/env bash
# Run only against a disposable emulator: removes any previous installation.
set -euo pipefail
apk=${1:?usage: android_smoke.sh APK OUTPUT_DIR}
out=${2:?usage: android_smoke.sh APK OUTPUT_DIR}
mkdir -p "$out"
pkg=com.jude.tetrisnesrust
activity="$pkg/.MainActivity"
cleanup() {
  status=$?
  adb logcat -d -v threadtime > "$out/logcat.txt" 2>&1 || true
  adb shell dumpsys activity activities > "$out/activities.txt" 2>&1 || true
  if [ "$status" -ne 0 ]; then echo "Android smoke test failed; see $out/logcat.txt" >&2; fi
}
trap cleanup EXIT
adb wait-for-device
# The runner waits for boot, but verify it here as well for local use.
for _ in $(seq 1 180); do
  if [ "$(adb shell getprop sys.boot_completed | tr -d '\r')" = 1 ]; then break; fi
  sleep 2
done
[ "$(adb shell getprop sys.boot_completed | tr -d '\r')" = 1 ]
adb shell input keyevent 82 || true
adb uninstall "$pkg" >/dev/null 2>&1 || true
adb install "$apk"
adb logcat -c
adb shell am start -W -n "$activity" | tee "$out/launch.txt"
if ! grep -q 'Status: ok' "$out/launch.txt"; then exit 1; fi
sleep 5
pid=$(adb shell pidof "$pkg" | tr -d '\r')
[ -n "$pid" ]
capture() { adb exec-out screencap -p > "$out/$1.png"; test -s "$out/$1.png"; }
assert_alive() {
  [ "$(adb shell pidof "$pkg" | tr -d '\r')" = "$pid" ]
  adb shell dumpsys activity activities | grep -q "$activity"
}
capture title
# Enter game type, level selection, and a short gameplay session using keyboard
# mappings, then pause. The same binary is exercised through JNI at each step.
for screen in game_type level playing; do
  adb shell input keyevent 66
  sleep 3
  assert_alive
  capture "$screen"
done
sleep 10
assert_alive
adb shell input keyevent 66
sleep 3
assert_alive
capture paused
if adb logcat -d | grep -E 'FATAL EXCEPTION.*|Fatal signal' | grep -q "$pkg"; then
  echo 'Fatal Android exception detected' >&2
  exit 1
fi
echo "PASS: $activity remained alive through title, menus, gameplay and pause (pid $pid)"
