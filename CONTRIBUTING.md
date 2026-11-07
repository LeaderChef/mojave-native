# Contributing to Mojave Native

This experimental source export is being prepared for public review. Start by reproducing the source build in BUILD-MAC.md; it has not yet been demonstrated from this fresh export. Keep evidence and known limits explicit.

## Work streams

These are acceptance tasks, not assertions that every listed behavior is broken.

| Stream | Small first task | Done means |
| --- | --- | --- |
| Mac application | Reproduce the ARM64 build on a second machine | Pinned source/toolchain, reproducible command and exact resulting app identity |
| Display and input | Check MacBook/external-display selection and focus | One recorded scenario; the app stays on the selected display without stealing focus |
| Audio | Check low-volume playback and output-device behavior | Actual audible playback and configuration persistence, not just successful audio-library compilation |
| Persistence | Run one short save/exit/reopen/load route | Character/world state compared before and after, with owned saves kept private |
| Metal renderer | Profile one bounded world route | Settings, frame-time distribution and memory observations; no claimed FPS from a static picture |
| Packaging | Create a bundle from already built sources | Correct resources, licence/source correspondence, no game payload or machine-specific paths |
| Test coordination | Reproduce an existing report | Exact revision, hardware and steps; one concise result instead of repeated screenshots |

## Changes and reviews

Read upstream [CONTRIBUTING.md](https://github.com/LootJunkieAzza/NVForever/blob/main/CONTRIBUTING.md)
before preparing an upstream change. Follow its current task-claiming, issue,
review and sign-off rules. A local draft is not permission to post as another
person, merge someone else's work or claim an issue reserved for a human.

For this integration, each task should have a short problem statement, one
owner, a bounded proposed change, its relevant evidence and a rollback point.
Clearly label compile-only, synthetic, headless-render and actual gameplay
results. Provide the focused check that can reject the old behavior; avoid
large repeated check runs that add no new evidence.

## Content boundaries

Use original project code, public documentation and independently observed
behavior under the upstream clean-room rules. Do not copy game decompilation,
leaked implementation or prohibited extender implementation into this engine.
Research into another project's public status does not authorize mixing its
implementation into NVForever.

Do not attach game executables, ESM/BSA files, meshes, textures, music, mod
archives, original-game captures used as private test data, private saves,
Steam/Nexus/GOG credentials, OAuth tokens or purchase/account records. Supply
synthetic fixtures or instructions for the tester's own purchased copy instead.

The repository retains GPL-3.0-or-later and upstream attribution.
New third-party dependencies and assets require a recorded licence and origin.
