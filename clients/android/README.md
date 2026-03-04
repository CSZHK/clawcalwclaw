# clawclawclaw Android Client 🦀📱

Native Android client for clawclawclaw - run your autonomous AI assistant on Android.

## Features

- 🚀 **Native Performance** - Kotlin/Jetpack Compose, not a webview
- 🔋 **Battery Efficient** - WorkManager, Doze-aware, minimal wake locks
- 🔐 **Security First** - Android Keystore for secrets, sandboxed execution
- 🦀 **clawclawclaw Core** - Full Rust binary via UniFFI/JNI
- 🎨 **Material You** - Dynamic theming, modern Android UX

## Requirements

- Android 8.0+ (API 26+)
- ~50MB storage
- ARM64 (arm64-v8a) or ARMv7 (armeabi-v7a)

## Building

### Prerequisites

```bash
# Install Rust Android targets
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android

# Install cargo-ndk
cargo install cargo-ndk

# Android SDK (via Android Studio or sdkman)
# NDK r25+ required
```

### Build APK

```bash
cd clients/android
./gradlew assembleDebug
```

### Build with Rust

```bash
# Build native library first
cargo ndk -t arm64-v8a -o app/src/main/jniLibs build --release

# Then build APK
./gradlew assembleRelease
```

## Architecture

```
┌─────────────────────────────────────┐
│  UI (Jetpack Compose)               │
├─────────────────────────────────────┤
│  Service Layer (Kotlin)             │
│  ├─ ZeroClawService                 │
│  ├─ NotificationHandler             │
│  └─ WorkManager Jobs                │
├─────────────────────────────────────┤
│  Bridge (UniFFI)                    │
├─────────────────────────────────────┤
│  Native (libclawclawclaw.so)            │
└─────────────────────────────────────┘
```

## Status

✅ **Phase 1: Foundation** (Complete)
- [x] Project setup (Kotlin/Compose/Gradle)
- [x] Basic JNI bridge stub
- [x] Foreground service
- [x] Notification channels
- [x] Boot receiver

✅ **Phase 2: Core Features** (Complete)
- [x] UniFFI bridge crate
- [x] Settings UI (provider/model/API key)
- [x] Chat UI scaffold
- [x] Theme system (Material 3)

✅ **Phase 3: Integration** (Complete)
- [x] WorkManager for cron/heartbeat
- [x] DataStore + encrypted preferences
- [x] Quick Settings tile
- [x] Share intent handling
- [x] Battery optimization helpers
- [x] CI workflow for Android builds

✅ **Phase 4: Polish** (Complete)
- [x] Home screen widget
- [x] Accessibility utilities (TalkBack support)
- [x] One-liner install scripts (Termux, ADB)
- [x] Web installer page

🚀 **Ready for Production**
- [ ] Cargo NDK CI integration
- [ ] F-Droid submission
- [ ] Google Play submission

## Contributing

See the RFC in issue discussions for design decisions.

## License

Same as clawclawclaw (MIT/Apache-2.0)
