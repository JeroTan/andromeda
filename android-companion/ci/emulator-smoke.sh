#!/usr/bin/env bash
set -euo pipefail

# Set ANDROID_SERIAL when more than one device is attached. CI has exactly one emulator.
initial_font_scale="$(adb shell settings get system font_scale | tr -d '\r')"
initial_night_mode="$(adb shell cmd uimode night | tr -d '\r')"
initial_services="$(adb shell settings get secure enabled_accessibility_services | tr -d '\r')"
initial_accessibility="$(adb shell settings get secure accessibility_enabled | tr -d '\r')"
restore() {
  if [ -n "$initial_font_scale" ] && [ "$initial_font_scale" != "null" ]; then adb shell settings put system font_scale "$initial_font_scale" >/dev/null 2>&1 || true; fi
  case "$initial_night_mode" in
    *yes*) adb shell cmd uimode night yes >/dev/null 2>&1 || true ;;
    *no*) adb shell cmd uimode night no >/dev/null 2>&1 || true ;;
  esac
  if [ -z "$initial_services" ] || [ "$initial_services" = "null" ]; then adb shell settings delete secure enabled_accessibility_services >/dev/null 2>&1 || true; else adb shell settings put secure enabled_accessibility_services "$initial_services" >/dev/null 2>&1 || true; fi
  if [ -z "$initial_accessibility" ] || [ "$initial_accessibility" = "null" ]; then adb shell settings delete secure accessibility_enabled >/dev/null 2>&1 || true; else adb shell settings put secure accessibility_enabled "$initial_accessibility" >/dev/null 2>&1 || true; fi
}
trap restore EXIT

adb shell settings put system font_scale 1.0
adb shell cmd uimode night no >/dev/null
adb install -r android-companion/app/build/outputs/apk/debug/app-debug.apk
adb install -r android-companion/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk
adb shell pm clear io.andromeda.companion >/dev/null
adb shell am start -S -W -a android.intent.action.MAIN -c android.intent.category.LAUNCHER -n io.andromeda.companion/.MainActivity >/dev/null

if adb shell pm list packages | grep -q '^package:com.google.android.marvin.talkback$'; then
  # API 33+ shows a notification-permission dialog when TalkBack starts. It covers the app
  # under test on a fresh emulator, so grant it before the accessibility probe runs.
  adb shell pm grant com.google.android.marvin.talkback android.permission.POST_NOTIFICATIONS >/dev/null 2>&1 || true
  adb shell settings put secure enabled_accessibility_services com.google.android.marvin.talkback/com.google.android.marvin.talkback.TalkBackService
  adb shell settings put secure accessibility_enabled 1
  result="$(adb shell 'am instrument --user 0 -w -e click "Enable accessibility" io.andromeda.companion.test/io.andromeda.companion.CompanionUiProbe')"
  if ! printf '%s' "$result" | grep -q 'INSTRUMENTATION_RESULT: settings_opened=true' || ! printf '%s' "$result" | grep -q 'INSTRUMENTATION_RESULT: status=passed'; then
    printf '%s\n' "$result" | sed '/INSTRUMENTATION_RESULT: nodes=/d'
    exit 1
  fi
  printf '%s\n' 'Phone permission button is accessible and opens Android settings.'
else
  printf '%s\n' 'TalkBack is absent from this emulator image; APK launch verified, screen-reader task check remains open.'
fi

result="$(adb shell 'am instrument --user 0 -w -e scenario session-settings io.andromeda.companion.test/io.andromeda.companion.CompanionUiProbe')"
if ! printf '%s' "$result" | grep -q 'INSTRUMENTATION_RESULT: session_settings=true' || ! printf '%s' "$result" | grep -q 'INSTRUMENTATION_RESULT: status=passed'; then
  printf '%s\n' "$result" | sed '/INSTRUMENTATION_RESULT: nodes=/d'
  exit 1
fi
printf '%s\n' 'Session timer defaults to Off; timer choices and settings navigation passed.'

adb shell am force-stop io.andromeda.companion
result="$(adb shell 'am instrument --user 0 -w -e scenario timer-persistence io.andromeda.companion.test/io.andromeda.companion.CompanionUiProbe')"
if ! printf '%s' "$result" | grep -q 'INSTRUMENTATION_RESULT: timer_persistence=true' || ! printf '%s' "$result" | grep -q 'INSTRUMENTATION_RESULT: status=passed'; then
  printf '%s\n' "$result" | sed '/INSTRUMENTATION_RESULT: nodes=/d'
  exit 1
fi
printf '%s\n' 'Session timer survives a new app process; Off restored.'

adb shell settings put system font_scale 2.0
adb shell cmd uimode night yes >/dev/null
adb shell am start -S -W -a android.intent.action.MAIN -c android.intent.category.LAUNCHER -n io.andromeda.companion/.MainActivity >/dev/null
adb shell pidof io.andromeda.companion >/dev/null
printf '%s\n' 'Companion stayed running with large text and dark mode.'

python3 android-companion/ci/lifecycle-probe.py --device "$(adb get-serialno)"
