#!/usr/bin/env bash
set -euo pipefail

artifact_dir="${1:?usage: decompile-apks.sh ARTIFACT_DIR}"
jadx_command="${JADX_BIN:-jadx}"
apktool_command="${APKTOOL_BIN:-apktool}"
apk_dir="$artifact_dir/apks"
output_dir="$artifact_dir/decompiled"

mapfile -t apks < <(find "$apk_dir" -maxdepth 1 -type f -name '*.apk' -print | sort)
if [[ ${#apks[@]} -eq 0 ]]; then
  echo "No APK files found in $apk_dir" >&2
  exit 1
fi

mkdir -p "$output_dir/jadx" "$output_dir/apktool" "$artifact_dir/analysis"

if ! "$jadx_command" --output-dir "$output_dir/jadx" "${apks[@]}" \
  >"$artifact_dir/analysis/jadx.log" 2>&1; then
  echo "JADX reported partial decompilation; inspect analysis/jadx.log" >&2
fi

for apk in "${apks[@]}"; do
  name="$(basename "$apk" .apk)"
  "$apktool_command" decode --force --output "$output_dir/apktool/$name" "$apk" \
    >"$artifact_dir/analysis/apktool-$name.log" 2>&1
done

printf 'Decompiler output written to %s\n' "$output_dir"
