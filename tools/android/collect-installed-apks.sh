#!/usr/bin/env bash
set -euo pipefail

package_name="${1:-com.kakao.talk}"
output_root="${2:-.research/kakaotalk}"

adb_command=(adb)
if [[ -n "${ANDROID_SERIAL:-}" ]]; then
  adb_command+=(-s "$ANDROID_SERIAL")
fi

"${adb_command[@]}" get-state >/dev/null

package_dump="$("${adb_command[@]}" shell dumpsys package "$package_name" | tr -d '\r')"
version_name="$(sed -n 's/^[[:space:]]*versionName=//p' <<<"$package_dump" | head -n 1)"
version_code="$(sed -n 's/^[[:space:]]*versionCode=\([0-9]*\).*/\1/p' <<<"$package_dump" | head -n 1)"

if [[ -z "$version_name" || -z "$version_code" ]]; then
  echo "Could not read version metadata for $package_name" >&2
  exit 1
fi

artifact_dir="$output_root/$version_name-$version_code"
apk_dir="$artifact_dir/apks"
mkdir -p "$apk_dir" "$artifact_dir/analysis"

mapfile -t remote_apks < <(
  "${adb_command[@]}" shell pm path "$package_name" |
    tr -d '\r' |
    sed -n 's/^package://p'
)

if [[ ${#remote_apks[@]} -eq 0 ]]; then
  echo "No installed APK paths found for $package_name" >&2
  exit 1
fi

for remote_apk in "${remote_apks[@]}"; do
  "${adb_command[@]}" pull "$remote_apk" "$apk_dir/$(basename "$remote_apk")"
done

printf '%s\n' "$package_dump" >"$artifact_dir/analysis/package.txt"
{
  printf 'model=%s\n' "$("${adb_command[@]}" shell getprop ro.product.model | tr -d '\r')"
  printf 'android_release=%s\n' "$("${adb_command[@]}" shell getprop ro.build.version.release | tr -d '\r')"
  printf 'sdk=%s\n' "$("${adb_command[@]}" shell getprop ro.build.version.sdk | tr -d '\r')"
} >"$artifact_dir/analysis/device.txt"

(
  cd "$artifact_dir"
  sha256sum apks/*.apk >SHA256SUMS
)

printf 'Collected %s %s (%s) in %s\n' \
  "$package_name" "$version_name" "$version_code" "$artifact_dir"
