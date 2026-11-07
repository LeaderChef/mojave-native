# Native save/reload check — 7 November 2026

[observed] Candidate 10's ARM64 client and bundled catalogue ran two independent,
silent M4 Pro/Metal processes using the owned base game and all nine official
add-ons. Both used a new disposable engine home; existing saves and game folders
were untouched. The write process started in Doc Mitchell's house. A console
sequence added three Stimpaks, set Luck to nine, disabled a reference and requested
a manual save. The fresh load process received that file through `--load`.

[observed] The native save contains 54 changed references, 359 quests and 229 script
instances. Both Game and renderer/engine state restoration are reported before
the fresh process's queries. Inventory is five before the addition, eight before
saving and eight after loading; Luck is nine and the reference remains disabled.
The 334,403-byte save is unchanged by loading. The save includes an automatically
produced thumbnail; no desktop screenshot or visible window was requested.

[observed] Both processes exit normally without foreground activation or GPU
validation errors. They take 27.45 and 17.12 seconds respectively. This is a real
client save/load path, rather than a serialization-only unit test. The executable
is unchanged from candidate 09 and the new catalogue is the movement-alias version.
See the [machine-readable record](mojave-mac-save-roundtrip-2026-11-07.json).

[observed] This establishes a bounded base/DLC roundtrip in one scene. It does not
establish full Nuclear Sunset/TTW saves, the manual save/load menu interaction,
long-session persistence, AI progress, human input, hardware audio, presented FPS,
an entire playthrough or multiplayer. Saved game bodies and private logs are not
included in the public source project.

NVForever is by LootJunkieAzza and contributors, GPL-3.0-or-later.
