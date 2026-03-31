# iOS Simulator Example

This folder contains a minimal Xcode iOS app example that links the Rust `spottedcat` library as an `xcframework` and runs it in the iOS Simulator.

The **core** `spottedcat` crate is an engine library and does **not** define any iOS app entrypoint.

This example uses a small wrapper crate at `examples/ios/spottedcat_ios_wrapper` that depends on `spottedcat` and exports `spottedcat_ios_start()` for demo purposes.

This example is **winit-driven**: the iOS app entrypoint is `main.m`, which calls `spottedcat_ios_start()`.

## Build the xcframework

From repo root:

```bash
bash -c 'chmod +x examples/ios/build_spottedcat_xcframework.sh'
bash examples/ios/build_spottedcat_xcframework.sh
```

This produces:

- `examples/ios/Spottedcat.xcframework`

## Run in Simulator

1. Open `examples/ios/SpottedcatIosSimulatorExample/SpottedcatIosSimulatorExample.xcodeproj`
2. Select an iOS Simulator device
3. Run

The app calls `spottedcat_ios_start()` on launch.

Note: on iOS, `winit`'s `EventLoop::run_app` calls `UIApplicationMain`, so you cannot start it from a SwiftUI `@main` app (that would call `UIApplicationMain` twice).

Note: the iOS Simulator may print messages like `Failed to send CA Event for app launch measurements...` / `Invalidating cache...`. These are system-level diagnostics and can be ignored if the app runs and renders correctly.
