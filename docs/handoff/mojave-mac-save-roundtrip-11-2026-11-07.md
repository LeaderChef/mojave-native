# Mac candidate 11: bounded save and reload

[observed] The exact candidate-11 ARM64 binary was run as two separate native
Metal processes on M4 Pro. Base New Vegas plus all nine official add-ons loaded.
It wrote an isolated new save, exited, then loaded it in a new process.

- Binary SHA-256: `6ff3dfc9b91eacb3b17268182b72329264098412ae17191d154edea24f858c0b`.
- Stimpak count 5 -> 8 before save -> 8 after load.
- Player Luck 9 and a disabled reference persisted.
- Both simulation body and engine state restored; the save bytes were unchanged
  by load. Save SHA-256: `46e09e48849b7f940f0720f3d758527d9bbe457176ccdd8a9b8d16d7ba45e511`.
- Both processes exited normally; no GPU validation errors or app activation.
- Private saves and original game data were not modified.

[scope] This is one scene through native windowless console-driven save/load,
not menu interaction, physical audio, presented FPS, long-session stability,
full TTW/Nuclear Sunset or multiplayer acceptance. Later binaries need their
own evidence. The earlier candidate-10 receipt remains separate.
