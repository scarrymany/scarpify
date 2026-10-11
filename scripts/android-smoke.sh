#!/usr/bin/env bash
# Offline native launch smoke. Playback/lifecycle remain separate device QA.
set -euo pipefail
mkdir -p smoke-results
adb shell cmd connectivity airplane-mode enable
adb shell svc wifi disable
adb shell svc data disable
adb install -r src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk
adb logcat -c
adb shell am start -W -n com.scarpify.app/.MainActivity > smoke-results/launch.txt
sleep 8
adb shell pidof com.scarpify.app > smoke-results/pid.txt
adb shell dumpsys activity activities > smoke-results/activities.txt
adb exec-out screencap -p > smoke-results/launch.png
adb logcat -d > smoke-results/logcat.txt
if rg -q 'FATAL EXCEPTION|Fatal signal|thread .* panicked' smoke-results/logcat.txt; then
  cat smoke-results/logcat.txt
  exit 1
fi
