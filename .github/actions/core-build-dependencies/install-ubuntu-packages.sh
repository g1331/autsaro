#!/usr/bin/env bash
set -euo pipefail

work_dir=$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/apt.XXXXXX")
trap 'rm -rf -- "$work_dir"' EXIT
sources="$work_dir/ubuntu.sources"
update_log="$work_dir/update.log"

# Azure package downloads stalled Linux desktop preparation (g1331/autsaro#374).
# Use the runner's official HTTPS fallback and keep its suites and signing key.
sed -e 's|mirror+file:/etc/apt/apt-mirrors.txt|https://archive.ubuntu.com/ubuntu/|g' \
    -e 's|http://azure\.archive\.ubuntu\.com/ubuntu/|https://archive.ubuntu.com/ubuntu/|g' \
    /etc/apt/sources.list.d/ubuntu.sources > "$sources"
cat "$sources"

apt_options=(
  -o "Dir::Etc::sourcelist=$sources"
  -o 'Dir::Etc::sourceparts=-'
  -o 'Acquire::Retries=5'
  -o 'Acquire::http::Timeout=15'
  -o 'Acquire::https::Timeout=15'
)

max_attempts=3
for ((attempt = 1; attempt <= max_attempts; attempt++)); do
  status=100
  if sudo env LC_ALL=C apt-get "${apt_options[@]}" update --error-on=any 2>&1 | tee "$update_log"; then
    if ! grep -Eq '^W: (Failed to fetch|Some index files failed)' "$update_log"; then
      sudo apt-get "${apt_options[@]}" install -y "$@"
      exit 0
    fi
  else
    status=$?
  fi

  if ((attempt == max_attempts)); then
    echo "apt-get update failed after $max_attempts attempts" >&2
    exit "$status"
  fi
  delay=$((5 * 2 ** (attempt - 1)))
  echo "apt-get update attempt $attempt/$max_attempts failed. Retrying in ${delay}s." >&2
  sleep "$delay"
done
