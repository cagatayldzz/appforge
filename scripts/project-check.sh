#!/bin/sh
# Reads a bounded project summary from stdin and reports recognizable privacy signals.
set -eu

summary="$(/bin/cat)"
if printf '%s\n' "$summary" | /usr/bin/grep -Eiq 'GoogleMobileAds|AdMob|AppLovin|UnityAds|IronSource'; then
  printf '%s\n' advertising
fi
if printf '%s\n' "$summary" | /usr/bin/grep -Eiq 'AppTrackingTransparency|NSUserTrackingUsageDescription|ATTrackingManager'; then
  printf '%s\n' tracking
fi
if printf '%s\n' "$summary" | /usr/bin/grep -Fiq 'PrivacyInfo.xcprivacy'; then
  printf '%s\n' privacy-manifest
fi
