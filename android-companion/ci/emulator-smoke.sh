#!/usr/bin/env bash
set -euo pipefail

adb install -r android-companion/app/build/outputs/apk/debug/app-debug.apk
adb install -r android-companion/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk
adb shell am start -W -n io.andromeda.companion/.MainActivity >/dev/null

if adb shell pm list packages | rg -q '^package:com.google.android.marvin.talkback$'; then
  adb shell settings put secure enabled_accessibility_services com.google.android.marvin.talkback/com.google.android.marvin.talkback.TalkBackService
  adb shell settings put secure accessibility_enabled 1
  result="$(adb shell 'am instrument --user 0 -w -e click "Enable accessibility" io.andromeda.companion.test/io.andromeda.companion.CompanionUiProbe')"
  if ! printf '%s' "$result" | rg -q 'INSTRUMENTATION_RESULT: settings_opened=true' || ! printf '%s' "$result" | rg -q 'INSTRUMENTATION_RESULT: status=passed'; then
    printf '%s\n' "$result" | sed '/INSTRUMENTATION_RESULT: nodes=/d'
    exit 1
  fi
  printf '%s\n' 'Phone permission button is accessible and opens Android settings.'
else
  printf '%s\n' 'TalkBack is absent from this emulator image; APK launch verified, screen-reader task check remains open.'
fi

adb shell settings put system font_scale 2.0
adb shell cmd uimode night yes >/dev/null
adb shell am start -W -n io.andromeda.companion/.MainActivity >/dev/null
adb shell pidof io.andromeda.companion >/dev/null
printf '%s\n' 'Companion stayed running with large text and dark mode.'
