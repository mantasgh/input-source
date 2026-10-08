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

## Why not `defaults write`?

The following command appends the Lithuanian layout to the saved input-source list:

```sh
defaults write com.apple.HIToolbox AppleEnabledInputSources -array-add \
'<dict>
  <key>InputSourceKind</key><string>Keyboard Layout</string>
  <key>KeyboardLayout ID</key><integer>-4016</integer>
  <key>KeyboardLayout Name</key><string>U.S. - Lithuanian</string>
</dict>'
```

The preference change may require logout or restart before the layout appears in the input menu. Repeating `-array-add` can create duplicates.

`input-source` instead requests enabling through macOS's input-source APIs in the current session.
