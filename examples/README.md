# Cross-Platform Examples

This repository keeps source-first example projects for the platforms that `spottedcat` declares support for.

Included in version control:

- WASM example source in `examples/wasm/wasm_demo` and browser host files in `examples/wasm/web`
- iOS wrapper and Xcode sample app in `examples/ios/spottedcat_ios_wrapper` and `examples/ios/SpottedcatIosSimulatorExample`
- Android Rust wrapper and Gradle sample app in `examples/android/spottedcat_android_wrapper` and `examples/android/GameActivityExample`

Excluded from version control:

- Rust `target/`
- Gradle `.gradle/`, `build/`, `.cxx/`
- WASM `pkg/`
- iOS `.xcframework/`, `DerivedData/`, `.tmp/`
- Android `jniLibs/` prebuilt `.so` outputs
- IDE-local files such as `.idea/`, `xcuserdata/`, and `local.properties`

## WASM

Key files:

- `examples/wasm/wasm_demo/Cargo.toml`
- `examples/wasm/wasm_demo/src/lib.rs`
- `examples/wasm/web/index.html`
- `examples/wasm/web/main.js`

Typical flow:

1. Build with `wasm-pack` from `examples/wasm/wasm_demo`
2. Serve `examples/wasm/web`
3. Load the generated package output locally without committing it

## iOS

Key files:

- `examples/ios/spottedcat_ios_wrapper/Cargo.toml`
- `examples/ios/spottedcat_ios_wrapper/src/lib.rs`
- `examples/ios/SpottedcatIosSimulatorExample/SpottedcatIosSimulatorExample.xcodeproj/project.pbxproj`
- `examples/ios/build_spottedcat_xcframework.sh`

Typical flow:

1. Build the Rust wrapper static library for iOS targets
2. Assemble the local `.xcframework`
3. Open the Xcode sample app and link against the locally built artifact

Notes:

- The iOS wrapper shows today's steps plus the last 7 days of pedometer history, both queried from Rust.
- Historical pedometer data is expected to be unavailable in the iOS Simulator.

## Android

Key files:

- `examples/android/spottedcat_android_wrapper/Cargo.toml`
- `examples/android/spottedcat_android_wrapper/src/lib.rs`
- `examples/android/GameActivityExample/app/build.gradle.kts`
- `examples/android/GameActivityExample/app/src/main/AndroidManifest.xml`
- `examples/android/build_spottedcat_android_libs.sh`

Typical flow:

1. Build the Rust Android shared libraries locally
2. Copy outputs into the Android app's `jniLibs/` directory locally
3. Open the Gradle project and run it from Android Studio or `gradlew`

Notes:

- The sample shows sensor-driven "today's steps", not a historical or lifetime total.
- On Android 10 and above, the sample requests `ACTIVITY_RECOGNITION` at runtime before step data becomes available.
- Recent step history is requested from Rust via JNI after Health Connect permission is granted by the Android host app.
