# Mojave Native roadmap

Updated 7 November 2026. Priorities and acceptance conditions, without release-date promises for unresolved engine work.

## 1 — Playable native Mac preview

| Work | Done means |
| --- | --- |
| Reproducible ARM64 source/build | Pinned NVForever base, identified Mac changes, retained licences, successful build by another contributor |
| Setup/application bundle | Player selects owned data; missing-file message; separate settings and saves |
| Playable route | Menu, controllable character, first/third person, interaction, combat and actual sound |
| Persistence/display | Save, normal exit, reopen/load; selected display respected; volume setting retained |
| Performance | Fixed route/settings, frame-time distribution and memory observations; cold vs warm loading separated |
| Preview release | App, corresponding source, checksums, install instructions, known issues and real capture |

Consistent 60 FPS is a target to measure, not an established result for every scene or Mac.

## 2 — Nuclear Sunset Hardercore + Long 15

- TTW opening/content routes, required dependencies and persistence.
- Nuclear Sunset graphics/gameplay and optional **Hardercore** balance, tracked by actual native behavior.
- **Long 15 - NCR Expansion:** selected-version compatibility, entry/exit, quests, navigation, encounters and save/load.
- Preserve original creator credits and permissions; link original downloads rather than mirror payloads.
- No fake extender version, empty function or hidden warning substitutes for support.

## 2a — Vehicles and NPC traffic

**New Vegas Drivables**, by **Into-The-Rough / Oliver Swanick Productions**, is a future integration target for drivable vehicles and NPC road traffic. The author's [FAQ](https://into-the-rough.github.io/new-vegas-drivables/faq.html) currently targets **December 2026** and announces a future GPLv3 source release and public vehicle tools. These are the author's plans, not our delivery dates or a released dependency.

- After release, pin the original version and review its actual source, licence, assets and xNVSE dependencies before native adaptation. Its public website is documentation, not the mod's engine source.
- Preserve the original creator's credit. Native vehicle physics, controls/cameras, NPC traffic behavior and parked/ridden vehicle persistence need their own implementation and validation in NVForever.
- Start with one drivable vehicle and one bounded traffic route. Use capped nearby traffic and cheaper distant simulation so roads feel alive without overwhelming the Mac.
- Later connect vehicles to trade, convoy escorts, fuel/repairs and server-authoritative persistence. Multiplayer synchronization remains a separate milestone.

This target is planned: no vehicle mod payload has been included and native Drivables compatibility has not been demonstrated.

## 3 — Persistent survival and strategy world

Future targets, not released capabilities:

- Settlements, bases, construction, storage, farming, production and upgrades; server-owned timers can advance while a player is offline.
- Factions/reputation with distinct identities, relationships, territory, recruitment, wages and consequences scaled to player actions.
- Economy/logistics: supply, demand, trade, resources, convoys, escorts, ambushes and meaningful production chains.
- Living world: varied regional encounters, persistent groups/settlements and cheaper simulation of distant activity.
- Strategic territory, army and siege decisions through first/third-person characters; resource management matters beyond PvP.

## 4 — Self-hostable community server

Initial target: **5–10 Mac/Windows players**, common protocol and authoritative server state. Different CPU architectures use the same world rules and separate native client builds.

Before release: concurrent sessions, reconnects, durable world saves, inventory/transaction validation, server-owned timing and bounded resource use. This is substantial later development.

## Progress reporting

Publish meaningful results: change, revision, verification, remaining faults and next bounded task. Keep compile-only, synthetic, headless rendering, single-player and multiplayer evidence separate. Download counts and screenshots are not compatibility percentages.

Check upstream work before assigning tasks; keep Mac changes small and offer suitable fixes upstream. Keep unfinished world systems separate from the first platform preview.
