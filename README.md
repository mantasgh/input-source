# input-source

A macOS command-line tool that registers and enables an installed keyboard layout or input method. It does not install files or switch the active input source.

```sh
cargo run -- "/Library/Keyboard Layouts/us-lithuanian.keylayout" "org.unknown.keylayout.US-Lithuanian"
```

Pass the installed file or bundle path and its input-source ID. Run `cargo run -- --help` for usage details.

macOS may display a confirmation dialog. Click **Allow** to finish enabling the input source.
