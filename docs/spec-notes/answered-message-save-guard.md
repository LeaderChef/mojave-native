# Saving after an answered message

[observed, native source] `Game::close_menu` and the engine's message-answer path close the visible box but retain `MessageBox::pressed` for the owning script's read-once `GetButtonPressed` call. The old save guard in `engine/nvf-play/src/save.rs` treated any retained `message_box` as open, so an acknowledged informational message continued to refuse saves.

[documented] The GECK options-menu example separates button polling from the menu transition: https://geckwiki.com/index.php/Adding_an_Options_Menu . Native button ownership and read-once behavior remain unchanged by this work; no external implementation is used.

[design] The save guard blocks a live script menu, queued boxes, or an unanswered message. A closed, answered message retained only for its script result does not block saving. Keep the answer intact, including owner restrictions; do not consume or clear it merely to permit saving. Retain dialogue, loading, active engine menu and pause/console-state guards.

[validated, 7 November 2026] Both new engine regressions failed on the old guard and pass with this change. They invoke a real script message, answer/close it, call the actual save guard, then read the chosen button exactly once. Open/unanswered, queued and unrelated-menu controls remain blocked. The full engine save-test filter passes (16 tests), required release engine Clippy passes, and the native ARM64 release build passes. The smoke Python suite passes (74 tests, 4 skipped).

[native runtime] An isolated, windowless M4 Pro/Metal run acknowledged the authored informational message and actual owned TTW dependency warnings, then the real F5 input wrote a native QuickSave. A fresh process loaded that exact file and restored positive Hardcore need values, which continued increasing. Both processes exited successfully with no GPU validation errors. The initial integration attempt caught a semicolon in the authored fixture's PrintC string; that fixture was corrected and the full save/load probe passed without changing the engine or weakening assertions. No user session, dependency warnings, original game data or existing saves were modified.

[limits] This validates the bounded native save-guard repair, not a complete TTW/Nuclear Sunset playthrough, visible controls, audio, sustained FPS or multiplayer. The public tree includes the new authored smoke scenario/fixture; the companion owned-data run and payload remain private. Root-crate tests were not repeated because this follow-up changes only the engine guard and authored fixtures.
