#!/bin/bash

# Clean root target
echo "Cleaning root target..."
cargo clean

# Clean Android example
echo "Cleaning Android wrapper target..."
if [ -d "examples/android/spottedcat_android_wrapper" ]; then
    (cd examples/android/spottedcat_android_wrapper && cargo clean)
fi

# Clean iOS example
echo "Cleaning iOS wrapper target and temporary files..."
if [ -d "examples/ios/spottedcat_ios_wrapper" ]; then
    (cd examples/ios/spottedcat_ios_wrapper && cargo clean)
fi
rm -rf examples/ios/.tmp

# Clean WASM example
echo "Cleaning WASM demo target..."
if [ -d "examples/wasm/wasm_demo" ]; then
    (cd examples/wasm/wasm_demo && cargo clean)
fi

echo "Cleanup complete!"
