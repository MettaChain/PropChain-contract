#!/bin/sh
find "$(dirname "$0")" -name "*.rs" -type f | while read f; do
  firstline=$(head -n 1 "$f")
  if [ "$firstline" != "// SPDX-License-Identifier: MIT" ]; then
    sed -i '1i // SPDX-License-Identifier: MIT' "$f"
  fi
done
