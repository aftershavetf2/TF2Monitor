# Embedded UI Fonts

These files are embedded at build time;
no system font installation or runtime download is required.

- `inter/Inter-Regular.ttf`: proportional UI text.
- `inter/Inter-SemiBold.ttf`: named heading family.
- `jetbrains-mono/JetBrainsMono-Medium.ttf`: named shortcut family.
- `noto-sans-symbols-2/NotoSansSymbols2-Regular.ttf`: braille and symbol fallback.
- `noto-emoji/NotoEmoji[wght].ttf`: monochrome emoji fallback (variable font,
  rendered at its default weight).

The Noto fonts are unmodified downloads from Google Fonts at commit
`cc39c4c2c304bc368c9b7c311ea2bcc6013299fe`:
- https://github.com/google/fonts/tree/cc39c4c2c304bc368c9b7c311ea2bcc6013299fe/ofl/notosanssymbols2
- https://github.com/google/fonts/tree/cc39c4c2c304bc368c9b7c311ea2bcc6013299fe/ofl/notoemoji

Inter remains the primary proportional font. The Noto fonts precede egui's
original symbol/emoji fallbacks in the default families and are inherited by
the named heading and shortcut families. Emoji are rendered in the text color;
this does not add color emoji or emoji-sequence shaping support.

Each font directory contains its upstream SIL Open Font License in `LICENSE.txt`.
The default egui font families remain in the fallback stacks.
