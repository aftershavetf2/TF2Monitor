# Embedded UI Fonts

Copied from the local Messy project. These files are embedded at build time;
no system font installation or runtime download is required.

- `inter/Inter-Regular.ttf`: proportional UI text.
- `inter/Inter-SemiBold.ttf`: named heading family.
- `jetbrains-mono/JetBrainsMono-Medium.ttf`: named shortcut family.

Each font directory contains its upstream SIL Open Font License in `LICENSE.txt`.
The default egui font families remain in the fallback stacks.

`tf2-classfont/tf2-classfont.ttf` provides TF2 class symbols in a dedicated font
family. Source: https://github.com/Qixalite/tf2-classfont (MIT; license bundled
in `tf2-classfont/LICENSE`). It is embedded and needs no runtime download.
