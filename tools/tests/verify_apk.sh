#!/usr/bin/env bash
set -euo pipefail
apk=${1:?usage: verify_apk.sh APK}
abi=${2:-arm64-v8a}
: "${ANDROID_HOME:?ANDROID_HOME must point at the Android SDK}"
build_tools="$ANDROID_HOME/build-tools/35.0.0"
"$build_tools/apksigner" verify --verbose "$apk"
"$build_tools/zipalign" -c -P 16 4 "$apk"
"$build_tools/aapt" dump badging "$apk" | grep -F "package: name='com.jude.tetrisnesrust'"
"$build_tools/aapt" dump badging "$apk" | grep -F "launchable-activity: name='com.jude.tetrisnesrust.MainActivity'"
python3 - "$apk" "$abi" <<'PY'
import sys, zipfile
apk, abi = sys.argv[1:]
with zipfile.ZipFile(apk) as z:
    files = set(z.namelist())
    dex = [f for f in files if f.startswith('classes') and f.endswith('.dex')]
    assert dex and any(b'Lcom/jude/tetrisnesrust/MainActivity;' in z.read(f) for f in dex), 'missing activity in DEX'
    assert f'lib/{abi}/libtetris_jni.so' in files, 'missing JNI shared library'
    assert 'AndroidManifest.xml' in files
print('PASS: DEX, activity, manifest and JNI library')
PY
so=$(mktemp)
trap 'rm -f "$so"' EXIT
unzip -p "$apk" "lib/$abi/libtetris_jni.so" > "$so"
readelf -Ws "$so" > "$so.symbols"
trap 'rm -f "$so" "$so.symbols"' EXIT
for method in nativeCreate nativeDestroy nativeTick nativePause nativeRender nativeBest nativeSetBest nativeRecord nativeSetRecord; do
  grep -q "Java_com_jude_tetrisnesrust_MainActivity_$method" "$so.symbols" || { echo "Missing JNI symbol $method" >&2; exit 1; }
done
readelf -l "$so" | grep -q '0x4000' || { echo 'JNI library is not aligned for 16 KB pages' >&2; exit 1; }
echo 'PASS: JNI exports and 16 KB ELF alignment'
