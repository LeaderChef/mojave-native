# MoveToMarker source spelling

[documented] GECK lists `MoveToMarker` in its function-alias category:
https://geckwiki.com/index.php?title=Category%3AFunction_Alias . The canonical movement operation and parameters are documented at https://geckwiki.com/index.php?title=MoveTo .

[observed] In the owned TTW selected load order, one unchanged quest source uses this spelling for five companion relocations. Its stored compiled statements decode as the existing `MoveTo` operation, opcode `0x109E`, with the same marker references. No opcode or movement rule is inferred from another engine implementation.

The catalogue therefore maps the alias to the existing function. It does not create a second handler, change the movement implementation, change the plugin version claims or alter authored mod files. Both ordinary and compiler-override grammar use the same catalogue entry; authored fixtures require identical emitted bytes/reference lists for both spellings and preserve optional offsets and caller references.

Private selected-mod compilation and runtime evidence is recorded separately. This source spelling fix alone does not establish TTW quest completion or full Nuclear Sunset compatibility.
