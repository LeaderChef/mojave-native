<p align="center">
  <img src="assets/mojave-native-neon.gif" alt="Mojave Native — neon over the Mojave" width="100%">
</p>

<h1 align="center">The Mojave. Native.</h1>
<p align="center"><b>Bringing New Vegas to Apple Silicon.</b><br>Rust · ARM64 · Metal · Open source</p>
<p align="center">
  <a href="BUILD-MAC.md"><b>Build the source</b></a> &nbsp; / &nbsp;
  <a href="CONTRIBUTING.md"><b>Help build it</b></a> &nbsp; / &nbsp;
  <a href="ROADMAP.md"><b>Roadmap</b></a> &nbsp; / &nbsp;
  <a href="STATUS.md"><b>Development status</b></a>
</p>
<p align="center"><sub>Early development · Experimental source snapshot · No playable release yet</sub></p>

---

**Mojave Native** is a community effort to bring Fallout: New Vegas to native **64-bit ARM64 macOS**, using **Metal** and the player's own game files. Our first goal is a Mac version that is straightforward to build, set up and play, with responsive first- and third-person controls, sound and reliable save/load.

**Built on [NVForever](https://github.com/LootJunkieAzza/NVForever) by LootJunkieAzza and the NVForever contributors.** Their Rust engine is the foundation. Mojave Native's Mac adaptations are separate work, and suitable improvements should go upstream.

## One road. Three destinations.

| **01 · Native Mac** | **02 · A richer Mojave** | **03 · A living world** |
| :--- | :--- | :--- |
| A playable Apple Silicon preview, measured performance, sound and save/load. | **Nuclear Sunset Hardercore**, TTW and **Long 15 - NCR Expansion**, with native feature compatibility. | Self-hosted **5–10-player Mac/Windows** survival, settlements, territory, production and trade. |
| **Current focus** | **Integration target** | **Long-term ambition** |

The later phases are goals, not released features. Strategy and resource management should matter, while you explore and act through a first- or third-person character.

## Help shape the next build

We need **Rust / Bevy**, **Metal / wgpu**, **macOS packaging** and **Apple Silicon testing** contributors. Pick one small, verifiable task: reproduce the exported source build, improve display/input behavior, check audio and save/reopen/load, or measure one gameplay route.

**[Start contributing →](CONTRIBUTING.md)** &nbsp; **[Open tasks →](https://github.com/LeaderChef/mojave-native/issues)**

<details>
<summary><b>What has actually been verified?</b></summary>

Updated 7 November 2026:

- A local **Mach-O ARM64 release executable** built successfully.
- Earlier sessions exercised parts of world rendering and movement. Focused player acceptance for the newest candidate is still incomplete.
- Two shared compiler fixes improved a fixed **337-input** ScriptRunner corpus from **316 to 319** successful compilations, without new failures. Compilation is not a mod-compatibility percentage.
- This new **source-only export needs fresh build reproduction**. It is not a verified public playable release.
- Full playthrough, full TTW/Nuclear Sunset compatibility, Long 15 gameplay and Mac/Windows shared sessions have not been demonstrated here.

**[Full status](STATUS.md)** · **[Build instructions and limits](BUILD-MAC.md)** · **[Technical background](RESEARCH.md)**

</details>

## The road ahead

<p align="center">
  <a href="ROADMAP.md"><img src="assets/mojave-native-roadmap.png" alt="Mojave Native roadmap: native Mac first, Nuclear Sunset and Long 15 next, a living multiplayer world long term" width="440"></a>
</p>

## Built on a community's work

| Project | Original creators |
| :--- | :--- |
| **[NVForever](https://github.com/LootJunkieAzza/NVForever)** — our engine foundation | **LootJunkieAzza and contributors** |
| **[Nuclear Sunset](https://nuclearsunset.com/docs/main/credits/)** — Hardercore integration target | **DarkSolarLegend, OrgasmicSneeze, DoktorAkcel, Lime, FinalCatalyst, TheBooher**, original mod authors and community |
| **[Long 15 - NCR Expansion](https://www.nexusmods.com/newvegas/mods/94306)** — expansion target | **Long 15 Team / ToxicWhiteout** |
| **[Tale of Two Wastelands](https://mod.pub/ttw/133-tale-of-two-wastelands)** | **TTW team and contributors** |

**[Full credits and original project links →](CREDITS.md)**

<details>
<summary><b>Your game. Original creators' rights.</b></summary>

Bring your own lawfully acquired New Vegas and DLC data. TTW/Nuclear Sunset also require the appropriate owned Fallout 3 data. Game files, mod archives, converted assets, credentials and private saves are not bundled. Credit does not grant permission to redistribute another creator's work.

NVForever code remains **GPL-3.0-or-later**; retain its notices and provide corresponding source for distributed covered binaries. Clean-room development rules apply. See [LICENSE](LICENSE), [UPSTREAM-LEGAL.md](UPSTREAM-LEGAL.md), [source origin](SOURCE-ORIGIN.json) and [artwork provenance](assets/README.md).

Fallout: New Vegas was developed by Obsidian Entertainment; Fallout names/content remain with Bethesda and their respective rights holders. Independent fan effort, with no endorsement by NVForever, Nuclear Sunset, Long 15 Team, Bethesda, Obsidian or Apple claimed.

</details>
