# Native IsGreetingPlayer condition

Issue: LeaderChef/mojave-native#27.

## Public contract and observed gap

- [documented] The xEdit FNV CTDA table assigns condition index **123** to
  `IsGreetingPlayer`, with no parameters:
  <https://tes5edit.github.io/fopdoc/FalloutNV/Records/Subrecords/CTDA.html>.
- [documented] The FNV command table lists `ref.IsGreetingPlayer` at decimal
  **4219**, or **0x107B**, with no arguments:
  <https://mod.gib.me/fonv/functions.html>.
- [observed] An owned TTW birthday continuation repeatedly reached this condition
  from an idle record. The native catalogue/dispatcher lacked it, so its value was
  the unhandled fallback zero. No game record, script or animation is distributed
  with this change.

## Native interpretation

- [hypothesis] The query is true only while the calling actor is actually saying
  a greeting to the player: a native HELLO bark, a Goodbye GREETING said outside a
  conversation, or an active GREETING response inside player dialogue. It is false
  between responses and after the greeting ends, and for ordinary speech or a
  conversation directed at another NPC. An actor in combat is not greeting.
- [observed] NVForever already stores each of those lifecycles in `Speech` or
  `Conversation::line`; the query should read them, without a new saved flag or
  treating every `IsTalking` result as a greeting. Existing `greets_player` holds
  Goodbye speakers still for AI; its narrower meaning must remain unchanged.
- [hypothesis] Exact original-engine HELLO/GREETING boundaries have not been
  recorded in a parity run. These are explicit native semantics, not a claim of
  original-engine parity or proof that a visible gesture played.

## Acceptance

- Authored fixtures exercise compiled reference calls and parsed CTDA dispatch,
  HELLO expiry/cancellation, conversation greeting/choice/later-topic boundaries,
  Goodbye greetings, unrelated actors and speech, and save/restore.
- Root tests, root clippy and engine clippy pass; an ARM64 release builds.
- A bounded, isolated Metal TTW continuation retains the previous quest/inventory
  checkpoint and no longer reports `0x107B` as unhandled. No foreground control,
  user project changes or packaged game data are needed.
- GetUsedItemLevel (`0x1102`) remains a separate investigation. It must not be
  replaced with a fabricated item-stat level or an unconditional success.
