# Rex application icon

These PNGs are the Rex Beta application icon (`App Icon Beta`, bundle version 0.1.0), taken
unchanged from the embedded `ic11` (32px), `ic07` (128px) and `ic13` (256px) representations of
the official macOS application's `.icns` file. Colours, transparency, padding, shadows and the
BETA badge are preserved.

**This is Rex/Superlogical artwork.** It is not original Rex Rune artwork and is not covered by
this repository's MIT license or the dependencies' open-source licenses. It is included only so
the client can identify itself as a client for Rex, and is used in the window icon, the identity
chip, empty states and the desktop launcher. If you redistribute or package Rex Rune, check that
doing so is acceptable to the icon's owner, or replace these files.

The app embeds the 256px PNG once and reuses it for its identity and empty-state marks.
`scripts/install-desktop.sh` copies the native-size files into the user's `hicolor` icon theme
under the application ID `dev.shuv.rex-rune`.

Source ICNS SHA-256: `d28fc64c3e6f6919d0c69e99056b7234b549ad6d88bd457404d9402833e0e1ed`.

| PNG | SHA-256 |
| --- | --- |
| 32px | `6dcc0b20afba99ae52e8ffd16e85a9a33783324b9554fef65c312b41c717d8f0` |
| 128px | `b03afd23aa65bdcefc462704524e387c88430159a5d6dfa63d05fd3554d2a2c0` |
| 256px | `e65a53577e38070a03f163bf5c84aaab6e7d6d9006dd7902e7bf306930e4c24c` |
