# Mac input and audio cache integration — 7 November 2026

## Change and attribution

[documented] This adaptation reuses merged NVForever maintainer work by
**LootJunkieAzza** and preserves the existing Mac integration:

- [PR 1699](https://github.com/LootJunkieAzza/NVForever/pull/1699), reviewed
  head `0f89a8bdd00d07a224b2aa489e6dd5a73f9ef95a`: release unused sound data.
- [PR 1724](https://github.com/LootJunkieAzza/NVForever/pull/1724), reviewed
  head `71411f2f32fe150af42c9f8278df6e090868c34a`: finer mouse sensitivity.

[observed] Applied changes cover `nvf-play` bindings, pause settings, ambient
loops, music, radio, decoded effects and the shared file-byte cache. Mac audio
device handling and the asset-release policy were preserved byte for byte.
The existing TTW child-world radio range handling was retained.

## User-facing behavior

[documented] Mouse sensitivity now reaches 5 percent, with finer steps at the low
end. Existing common settings remain representable. INI output retains enough
precision for the new settings to survive a save/load cycle.

[documented] Radio, music, ambient and effect caches can release unused data
under the engine's existing age/count policy. A playing decoder retains shared
bytes; playing asset handles and sample references protect active effects. An
evicted file is read again when needed. This is not a hard byte limit for all
engine memory, and it does not by itself establish a measured RAM or FPS gain.

## Evidence available before the client build

[observed] 36 focused tests passed: three sensitivity checks, 31 effect checks,
one shared-file lifetime check and one TTW radio range check. The additional
Mac regression test exercises eviction with zero idle-cache capacity while
holding both a live asset handle and sample references, then releases those
holders and verifies subsequent eviction.

[observed] Engine all-target release Clippy passed with warnings treated as
errors. Two existing client test-style warnings were corrected without changing
runtime code or relaxing checks. Smoke scenario integrity verification passed.

## Bounded runtime result

[observed] The release build completed in 4 minutes 22 seconds and identifies as
Mach-O 64-bit ARM64, SHA256
`9083779db3c649038ca187bcb17239e3ef514f27f8f4209d9375303dbb31fac6`.
On Apple M4 Pro/Metal, the owned base game plus all nine official add-ons ran
without a native window, activation or graphics validation errors. Both the
initial run and the fresh-process reload exited normally.

[observed] The private 48 kHz stereo float capture contains 53.075 seconds of
mixed audio, with nonzero samples, peak 0.755001, no values beyond +/-1 and no
nonfinite values. The radio reported a song and seven DJ lines with no missing
sound files. No hardware output device was opened. Startup stalls caused the
bounded capture consumer to skip 257819 frames; this run does not establish
continuous hardware playback quality or latency.

[observed] A new manual save was written, loaded in a fresh process and saved
again. Player identity, inventory, actor-value pools, camera mode, magazine and
position roundtripped in the checked fields (position error zero). Both saves
contain a decodable, nonblank 512x288 thumbnail. The game was not activated and
no standalone scene screenshot was requested.

## Runtime acceptance boundary

[documented] The check uses the player's owned base game and all official
DLC in an isolated, read-only profile. A bounded windowless run captures the
game's own mixer to a private WAV instead of opening a speaker; it also creates
a private engine save for a fresh load check. No unrelated application audio is
recorded, and no screen activation or repeated scene screenshots are needed.

[hypothesis] The new cache should reduce retained sound data over long sessions;
the unit checks establish lifetime correctness, not a measured long-session
memory reduction. Silent mixer output is not hardware audio acceptance.
Windowless execution is not a visible-window FPS or human input measurement.
Full TTW/Nuclear Sunset/Long 15 compatibility and multiplayer remain separate,
unfinished acceptance work.
