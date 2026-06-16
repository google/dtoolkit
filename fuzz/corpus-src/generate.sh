#!/bin/sh

# Copyright 2026 Arm Limited and/or its affiliates <open-source-office@arm.com>
#
# Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
# https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
# <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
# option. This file may not be copied, modified, or distributed
# except according to those terms.

# Generate DTB corpus files from DTS files.

if [ $# -ne 2 ]; then
    echo "error: input and output directories are required" >&2
    echo "usage: $0 <input dir> <output dir>" >&2
    exit 1
fi

input_dir=$1
output_dir=$2

for dts in "$input_dir"/*.dts; do
    dtb="${dts%.dts}.dtb"
    dtc -Idts -Odtb -o "$output_dir/$dtb" "$dts"
done
