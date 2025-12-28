#!/bin/bash

set -e

# --- Setup: Create sample zip files ---
iz0="./sample.d/hw0.zip"
iz1="./sample.d/hw1.zip"
iz2="./sample.d/hw2.zip"
dhost=./sample.d
dguest=/guest.d

genizips() {
    echo "---"
    echo "Creating sample input files..."
    rm -rf ./sample.d
    mkdir -p ./sample.d

    echo "hw00" >./sample.d/hw00.txt
    echo "hw10" >./sample.d/hw10.txt
    echo "hw11" >./sample.d/hw11.txt
    echo "hw20" >./sample.d/hw20.txt

    # -0 means no compression
    # Create zips with different sizes
    zip -0 -j "${iz0}" ./sample.d/hw00.txt >/dev/null
    zip -0 -j "${iz1}" ./sample.d/hw10.txt ./sample.d/hw11.txt >/dev/null
    zip -0 -j "${iz2}" ./sample.d/hw20.txt >/dev/null
    echo "Sample files created."
    ls -l ./sample.d/*.zip # Show file sizes
}

# Ensure zips exist
test -f "${iz0}" || genizips
test -f "${iz1}" || genizips
test -f "${iz2}" || genizips

# --- Helper function to run the wasm module ---
run_wasm() {
    local input_paths="$1"
    shift # The rest of the arguments are for the wasm module

    # Use a subshell to avoid issues with `echo` and pipes
    (echo "${input_paths}") |
        wazero run \
            -mount="${dhost}:${dguest}:ro" \
            ./target/wasm32-wasip1/release-wasi/rawzips2blobs2jsons.wasm "$@" |
        jq -c .
}

# --- Example 1: Happy path, process all files ---
ex1() {
    echo
    echo "--- Example 1: Standard run, process all zips and items ---"
    input_paths=$(find ./sample.d -type f -name '*.zip' | cut -d/ -f3- | sed -n -e 's,^,/guest.d/,' -e p | sort)
    run_wasm "${input_paths}" \
        --zip-size-max 1048576 \
        --item-size-max 131072 \
        --item-content-type text/plain \
        --item-content-encoding identical \
        --verbose
}

# --- Example 2: Demonstrate skipping a zip file ---
ex2() {
    echo
    echo "--- Example 2: Demonstrate zip skipping with --zip-size-max=150 ---"
    # We expect a warning on stderr about hw1.zip being skipped (it's > 250 bytes)
    input_paths=$(find ./sample.d -type f -name 'hw1.zip' | cut -d/ -f3- | sed -n -e 's,^,/guest.d/,' -e p)
    run_wasm "${input_paths}" \
        --zip-size-max 150 \
        --item-size-max 131072 \
        --item-content-type text/plain \
        --item-content-encoding identical \
        --verbose
}

# --- Example 3: Demonstrate skipping an item within a zip ---
ex3() {
    echo
    echo "--- Example 3: Demonstrate item skipping with --item-size-max=4 ---"
    # The text files are 5 bytes each ("hw00\n"), so a limit of 4 will cause them to be skipped.
    input_paths=$(find ./sample.d -type f -name 'hw0.zip' | cut -d/ -f3- | sed -n -e 's,^,/guest.d/,' -e p)
    run_wasm "${input_paths}" \
        --zip-size-max 1048576 \
        --item-size-max 4 \
        --item-content-type text/plain \
        --item-content-encoding identical \
        --verbose
}

# --- Example 4: Demonstrate resilience by skipping one oversized zip in a list ---
ex4() {
    echo
    echo "--- Example 4: Process a list where one zip is oversized ---"
    # hw0.zip and hw2.zip should be processed, hw1.zip should be skipped.
    input_paths=$(find ./sample.d -type f -name 'hw*.zip' | cut -d/ -f3- | sed -n -e 's,^,/guest.d/,' -e p | sort)
    run_wasm "${input_paths}" \
        --zip-size-max 250 \
        --item-size-max 131072 \
        --item-content-type text/plain \
        --item-content-encoding identical \
        --verbose
}


# --- Run all examples ---
# Force regeneration of zips to ensure correct sizes for ex4
ex1
ex2
ex3
ex4
