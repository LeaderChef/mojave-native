# Persistent funded work orders [design]

This is an opt-in J6 extension, not an assertion about the original game's crafting
rules. Normal instant crafting stays available. At audited WorkBench and
CampfireCrafting01 activators a player may queue a concrete-item recipe for 15
accepted game minutes per batch. Its skill, full CTDA conditions, category,
station identity, access, distance and bounded batch are checked at admission.

A disabled dynamic MetalBox01Empty container owned by PLAYER_BASE holds the
actual funded items, including extra-data groups. The ledger's inventory seal is
read-only evidence. It is never an alternative spendable stock balance. Ordinary
inventory APIs and deletion refuse the active reservation. Trusted direct mutable
World access is not a security sandbox; a seal mismatch fails closed.

Working -> Ready uses the existing atomic inventory exchange in that storage.
Ready -> Collected transfers all actual outputs atomically to the player. Working
-> Cancelled transfers the actual remaining inputs, retaining their metadata.
No consumed inputs are regenerated after completion. Capacity refusal leaves both
holders and the order unchanged. Closed commands are retained and replaying one
cannot fund, finish, refund or collect a second time. One active order per station,
64 active orders and 4096 retained commands are conservative first-version limits.
There is no automatic deletion of financial history.

## Two clocks, not two notions of a game day

* Normal recipes use World's monotonic accepted-game-hours counter. Waiting,
  travelling and normal simulation credit the accepted interval once. Menus,
  pause, direct calendar writes and failed calendar commits do not grant work.
* Long construction/upgrade timing uses a separate HostUtc deadline: started UTC
  seconds + checked duration seconds. 48 hours is exactly 172800 seconds. It is
  persisted even when the client is offline. A host restart after the deadline
  resolves completion once from saved stock. It does not run 172800 per-second
  simulation steps or load every settlement in the world.

The Game layer takes an explicit trusted host time sample and never reads client
wall time itself. The native executable is currently a local single-player host:
its service samples SystemTime at most once a second, including menus/pause,
only while host-timed work exists. Backwards samples clamp to the last accepted
host sample. A deliberately fast-forwarded or incorrect host system clock is not
prevented by this local implementation. A future multiplayer host must accept,
fund and commit requests server-side and supply its own UTC samples. Clients
must receive status; their own clocks and completion claims are not authority.
Commands at the boundary must process a fresh host sample before cancellation.

This release provides the HostUtc scheduling primitive and tests. It does not yet
provide a network settlement server, construction blueprints, automatic NPC
workers, visible building upgrades or a verified 48-hour live construction soak.
A short real-clock cold restart probe is distinct from the 48-hour deterministic
Game/World test. No such test is a claim that settlement multiplayer is playable.

## Save boundary and conservative recovery

Body/game schema 2 makes the ledger mandatory and prevents an older executable
from silently stripping funded work. Schema 1 migrates only a pristine ledger.
Shape/format validation runs before generic JSON FormID remapping. Non-pristine
history currently requires the saved plugin order and sizes as an unchanged
prefix; appended plugins retain existing ids, but changed recipe content is still
rejected. No recipe permission is inferred merely from a base FormID.

Restore validates the book against the actual restored World, including orphan
reservations, exact stock, storage ownership/type, station identity and full
recipe descriptor. Invalid state refuses load without discarding the current
runtime/session or starting a new world. Closed receipts survive even after their
empty dynamic container is omitted from snapshots. A due runtime failure is held
without repeat work/logging every frame. Restore after correcting the underlying
binding can retry; malformed stock is not reseeded or silently repaired.

The two cached next deadlines and held messages are transient. They are rebuilt
at restore. The hot idle path compares cached scalar deadlines rather than
cloning the World or walking every inventory each frame.
