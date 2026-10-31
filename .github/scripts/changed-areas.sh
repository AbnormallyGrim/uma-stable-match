#!/usr/bin/env bash
# Prints `<area>=true` or `<area>=false` for each `<area>=<regex>` argument: true when a changed
# path, read from stdin one per line, matches the extended regex. With `--all`, every area is true.
set -euo pipefail

all=false
if [ "${1:-}" = --all ]; then
  all=true
  shift
fi

changed=""
if [ "$all" = false ]; then
  changed=$(cat)
fi

for arg in "$@"; do
  area=${arg%%=*}
  pattern=${arg#*=}
  if [ "$all" = true ] || grep -qE -- "$pattern" <<<"$changed"; then
    echo "$area=true"
  else
    echo "$area=false"
  fi
done
