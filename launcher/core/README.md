# Native launcher core

Shared launch planning, game-data inspection, executable validation and signed
update-manifest checks. GPL-3.0-or-later; the engine is NVForever by LootJunkieAzza
and contributors. These files contain no game data or promotional artwork.

On an Apple Silicon Mac with Xcode command-line tools, from this directory:

```sh
xcrun swiftc -swift-version 5 -O -target arm64-apple-macos13.0 -framework CryptoKit LauncherCore.swift SelfTests.swift main.swift -o /tmp/mojave-launcher-checks
/tmp/mojave-launcher-checks
```

The authored checks use disposable fixtures and do not construct NSApplication,
open a window, launch the game, use the network or write player preferences.
They cover the actual launch plan, including the bundled script catalogue,
rejecting its absence and preserving its correct path after relocation.
The themed launcher UI/resources and a public packaged application are separate
work; this is a reusable core, not a complete application installer.
