#!/usr/bin/env bash
# Copyright 2026 Google LLC
#
# Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
# https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
# <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
# option. This file may not be copied, modified, or distributed
# except according to those terms.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DTS_DIR="${SCRIPT_DIR}/dts"
DTB_DIR="${SCRIPT_DIR}/dtb"

for dts in "${DTS_DIR}"/*.dts; do
    dtb="${DTB_DIR}/$(basename "${dts}" .dts).dtb"
    echo "Compiling $(basename "${dts}") -> $(basename "${dtb}")"
    dtc -@ -I dts -O dtb -o "${dtb}" "${dts}"
done
