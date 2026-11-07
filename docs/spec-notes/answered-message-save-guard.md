# Saving after an answered message

[observed, native source] `Game::close_menu` and the engine's message-answer path close the visible box but retain `MessageBox::pressed` for the owning script's read-once `GetButtonPressed` call. The save guard in `engine/nvf-play/src/save.rs` currently treats any retained `message_box` as open, so an acknowledged informational message can continue to refuse saves.

[documented] The GECK options-menu example separates button polling from the menu transition: https://geckwiki.com/index.php/Adding_an_Options_Menu . Native button ownership and read-once behavior remain unchanged by this work; no external implementation is used.

[design] The save guard blocks a live script menu, queued boxes, or an unanswered message. A closed, answered message retained only for its script result does not block saving. Keep the answer intact, including owner restrictions; do not consume or clear it merely to permit saving. Retain dialogue, loading, active engine menu and pause/console-state guards.

[validation plan] Authored engine tests invoke a real script message, answer/close it, call the actual save guard, then read the chosen button exactly once. Open/unanswered and queued message controls remain blocked. Required engine checks and a bounded windowless ARM64/Metal save plus fresh-process load must pass. Do not suppress dependency warnings or claim full TTW/Nuclear Sunset compatibility.
