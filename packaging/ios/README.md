# Horizon on iOS (unofficial port)

An unofficial fork of [Horizon](https://github.com/horizon-rust/horizon) that builds the game
for iPhone. Not made or supported by the Horizon project, Mojang or Microsoft. The IPA holds no
Minecraft files: you copy the resources Horizon prepared from your own copy of the game.

## Build

`.github/workflows/ios.yml` builds `Horizon.ipa` on a macOS runner on every push to `ios-port`
and uploads it as the `Horizon-ipa` artifact. By hand, on a Mac:

```sh
rustup target add aarch64-apple-ios
cargo build --profile fast --target aarch64-apple-ios -p horizon-client --bin horizon-client \
  --no-default-features --features audio
bash packaging/ios/package.sh target/aarch64-apple-ios/fast/horizon-client
```

The IPA is signed ad hoc: install it with LiveContainer, TrollStore, or a sideloading tool that
signs it for you.

## Resources

1. Start Horizon once on the phone and close it. It creates its game folder, `Documents/horizon`
   (under LiveContainer: Files > On My iPhone > LiveContainer > Data > Application > the newest
   folder > Documents > horizon).
2. On the computer where the Horizon launcher runs, zip the contents of
   `.local/horizon-client/26.3` of the installed version (on Windows:
   `%APPDATA%\horizon-launcher\versions\<version>\.local\horizon-client\26.3`). The resources
   must come from the same Horizon commit this fork is based on.
3. Copy the zip to the phone and unpack it as `Documents/horizon/resources`, so that
   `Documents/horizon/resources/horizon-resources.json` exists.

Without them the game closes at once and says so in `Documents/horizon/logs/latest.log`.

## Controls

Behind a screen a finger is the mouse. While playing: the lower left is the move stick (push it
far forward to sprint), a finger anywhere else turns the view, a tap uses or places (or attacks
the mob under the crosshair), a finger held still mines. Jump, Use (hold to eat, block or draw
a bow) and Sneak are on the right, `...` opens the inventory, `||` pauses, and a tap on the
hotbar selects a slot.

## What differs from the desktop game

- No controllers and no code mods (`--no-default-features --features audio`).
- The system clipboard is not used; copy and paste stay inside the game.
- No text entry yet: there is no keyboard to type a world name, a chat line or a sign.
- The first start writes small options (render distance 4, two chunk workers) for a phone's
  memory; change them in the options.
