# Promotional artwork

Original project posters were generated with OpenAI image generation and directed/selected for Mojave Native. They are promotional illustrations, not current-engine captures or performance evidence.

- `mojave-native-landscape.png`: landscape recruitment poster.
- `mojave-native-roadmap.png`: roadmap poster with radiation trefoil.
- `mojave-native-neon-v3.gif`: current casino banner; 180 frames, an infinite 14.4-second loop. Only Native flickers, retaining visible dim amber tubes rather than blacking out the letters; Lucky 38 lettering and Mojave are steady. Chasing perimeter bulbs, canopy bulbs and a travelling tower red-strip glow use separate emitter masks.
- `mojave-native-neon-v2.gif`: earlier full-sign relighting sequence, retained as an earlier asset.
- `mojave-native-neon.gif`: earlier subtle animation, retained as an earlier asset.
- `mojave-native-wordmark-v2.svg`: code-authored amber/grey title with the radiation symbol centered between Mojave and Native.

The finished poster uses a radiation trefoil instead of the decorative star and omits the bottom concept-art caption at the user's request. Provenance is recorded here instead. Prompts/editing notes are in `prompts.json`.

Fallout names and fictional locations remain with their respective owners.

The GIF uses the original generated key art and a local ImageIO animation timeline. Only the Native lettering has an irregular worn-neon timeline. The Lucky sign perimeter bulbs chase along their actual contour; only the entrance canopy bulb cores pulse; a glow travels on the tower's red strip. All remaining pixels, including Lucky lettering, Mojave lettering, the building and tower windows, are copied unchanged. The SVG title places the radiation symbol between the two words. No video/audio or game assets are used. `animate-neon.swift` supplies the rendering/encoding source.
