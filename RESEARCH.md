# Reuse decision — 7 November 2026

The fastest defensible route for the requested Rust engine is a focused Mac
integration of NVForever, with suitable changes offered upstream. The work has
already produced a native ARM64 binary; the next deliverable is a playable,
reproducible preview with explicit evidence. Completing a large modpack's engine
patches is a separate workload.

## Existing work and what it buys us

| Work | Reusable value | Boundary |
| --- | --- | --- |
| [NVForever](https://nv-forever.com/) / [source](https://github.com/LootJunkieAzza/NVForever) | Existing Rust engine, game-data readers, renderer and game systems; the current local base is its v0.6-era code | Official v0.6.0 is still a prerelease; extender integration is unfinished. Windows/Linux release availability is not Mac runtime acceptance. |
| [FNV decompilation](https://fnvdecomp.com/) / [project overview](https://github.com/momusdigital/fnv_decomp) | The authors report a completed reconstruction matching the original PC program; valuable evidence that a reconstruction exists | Matching the old Windows program does not itself supply an ARM64/Metal port. Its implementation must remain separate from NVForever's clean-room source. We reviewed public status, not imported its game implementation. |
| [Fallout-NV-MacOS](https://github.com/thetheoryofR/Fallout-NV-MacOS) | A different Apple Silicon static-recompilation project worth assessing as prior art | Its README reports a recorded hour and performance results, not reproduced here. It also lists missing audio, Bink, gamepad and NVSE. Its 4 GB guest-address model is distinct from rebuilding game systems as native typed 64-bit Rust. No implementation has been imported. |
| [OpenMW extended modding](https://openmw.readthedocs.io/en/latest/reference/modding/extended.html) | A replacement engine can expose shared documented mod interfaces | An extended API needs real native consumers; a function name or successful file load is insufficient. |
| [OpenGothic modifications](https://github.com/Try/OpenGothic#modifications) | Existing replacement-engine and platform-release practices | Original-engine native extensions have separate compatibility limits. Do not copy another engine's game-specific memory model. |
| [Slang](https://shader-slang.org/docs/compilation-api/) | Shader-language conversion and reflection | A converted shader still needs the correct native renderer inputs and pass placement. |

## Applied improvements, rather than a speculative converter

The selected plugin/script/config workfront was made independent of bulk art
preparation. All 253 selected plugins and their master dependencies could then
be analysed while the rest of the 154,969-directive data graph continued.

Two shared compiler fixes now let three previously failing unchanged authored
ScriptRunner inputs compile. On one complete 337-input pass, 319 compiled and
18 reported errors, with no new failing inputs. These are compilation results,
not a mod-compatibility percentage.

An original synthetic HLSL shader was converted to SPIR-V and WGSL and rendered
on the M4 Pro's Metal adapter, with pixel results checked against its declared
reference. A uniform-only variation reused the conversion cache. This is an
applied toolchain proof, not an imported Nuclear Sunset shader or a measured
gameplay speed increase.

## What is not a shortcut

Mechanically executing a Windows plugin's CPU instructions does not supply the
original engine objects, graphics hooks, event ordering or save lifecycles that
the plugin expects. A new runtime or language cannot remove those contracts.
Use the selected mods' actual demands to choose shared fixes instead.

Public videos are useful demonstrations and leads. They do not establish that
a different build, mod order or Mac has the same behavior. The first public
Mac preview should show its own short, reproducible gameplay route.

## Concrete upstream reuse selected by the new audit

Two changes merged into NVForever on 6 November are absent in our integrated
source and are suitable for narrow review:

- [PR1724](https://github.com/LootJunkieAzza/NVForever/pull/1724), commit
  `71411f2f32fe150af42c9f8278df6e090868c34a`: finer mouse-sensitivity control,
  including a lower minimum and a logarithmic adjustment scale.
- [PR1699](https://github.com/LootJunkieAzza/NVForever/pull/1699), commit
  `0f89a8bdd00d07a224b2aa489e6dd5a73f9ef95a`: bounded retention of unused audio
  file bytes while retaining data still held by live users.

Both are upstream-maintainer changes. Private adaptation candidates are being
prepared, preserving the Mac audio/output and memory-pressure work. They are
not yet applied, built or accepted here; no Mac performance gain is claimed.
The saved-game PR1701 contains 61 stacked commits and overlaps local restore
protections, so it must not be merged wholesale to obtain a small fix.

## Community structure

Use a clearly attributed Mac integration fork and small upstreamable patches.
Keep a separate compatibility tracker for later TTW/Nuclear Sunset work.
Contributors should receive independently testable tasks, not an open-ended
request to finish an entire engine or a promise that multiplayer is ready.

The working name is **Mojave Native**, with the descriptive line **NVForever's Rust
engine rebuild, native on 64-bit Mac**. It is a proposal, not a cleared trademark,
registered domain or claim of official affiliation.

## New Vegas Drivables — future vehicles and traffic

The [author's public documentation](https://into-the-rough.github.io/new-vegas-drivables/)
and [FAQ](https://into-the-rough.github.io/new-vegas-drivables/faq.html), checked
7 November 2026, describe drivable vehicles and NPC road traffic, with a planned
December 2026 release. Authorship: Into-The-Rough / intotherough, publishing as
Oliver Swanick Productions. A future GPLv3 source/tool release is announced.
The [public repository](https://github.com/Into-The-Rough/new-vegas-drivables)
currently hosts its website and documentation, not the vehicle mod's engine source.

Treat the release as a future input. After publication, review its actual licence,
asset permissions and xNVSE-dependent behavior, then adapt shared contracts to
NVForever. One vehicle, its persistence and a bounded nearby NPC traffic route
come before large traffic populations or multiplayer convoys. No implementation
has been imported and no native compatibility is claimed.
