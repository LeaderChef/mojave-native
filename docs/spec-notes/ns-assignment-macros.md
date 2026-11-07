# Native assignment macro compatibility

2026-11-07, selected Nuclear Sunset script workfront.

[observed] The private exact winning ScriptRunner directive16702 uses an
unprefixed division assignment on a float local. Its unchanged source SHA256 is
358c2ece827ffbf1e8d53600d0f22d0980793672bdb241276d63dd1b4edd4339.
The pinned older native tool reports `not_a_call` on this line. No extender
implementation was read and no mod source is altered by this change.

[documented] `ext-syntax.md` section2 records arithmetic/bitwise/remainder
assignment operators in the extender expression language. Section4 records the
four examples named on the Let Macro page; acceptance of the other unprefixed
operators in the original GECK remains an unmeasured point. Public reference:
https://geckwiki.com/index.php?title=Let_Macro

[native extension] Recognize all assignment operators already implemented by
the native extended-expression parser when they appear after a valid macro
target, including indexed array targets. Classification then delegates to the
same expression compiler and VM as explicit `let`. This supplies the selected
script's intended division through an existing real arithmetic consumer, with
no string rewriting, version spoofing, numeric stub or special mod branch.

The classifier does not accept comparisons as assignments. Invalid targets and
malformed expressions still produce compiler errors. Assignment associativity,
numeric conversion, division-by-zero behavior and event/save lifecycles are not
changed here. Synthetic compile/decode/VM checks compare the macro with explicit
Let and verify final scalar/array values. Original-game syntax parity and the
complete real mod lifecycle remain separate acceptance work.
