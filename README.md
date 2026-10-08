# input-source

A macOS command-line tool that registers and enables an installed keyboard layout or input method. It does not install files or switch the active input source.

Download `input-source` from [Releases](https://github.com/mantasgh/input-source/releases/latest) for Apple Silicon Macs. Make it executable and run:

```sh
chmod +x input-source
./input-source "/Library/Keyboard Layouts/us-lithuanian.keylayout" "org.unknown.keylayout.US-Lithuanian"
```

Pass the installed file or bundle path and its input-source ID. Run `./input-source --help` for usage details. To build from source, use `cargo build --release`.

macOS may display a confirmation dialog. Click **Allow** to finish enabling the input source.

Release binaries are not Apple-notarized. If macOS blocks execution, follow [Apple's instructions for opening trusted software](https://support.apple.com/en-us/102445).
