# Native large-address query

## Original command and native contract

[documented: JIP's GetIsLAA description on the GECK wiki]
<https://geckwiki.com/index.php?title=GetIsLAA> describes two original checks:
the executable's LAA flag and an allocation at a large address. Its original
0/1/2 results distinguish an absent flag, a failed allocation, and success.

[documented: Apple virtual-memory overview]
<https://developer.apple.com/library/archive/documentation/Performance/Conceptual/ManagingMemory/Articles/AboutMemory.html>
distinguishes native 64-bit process address space from the 32-bit space. This
does not establish a particular amount of free physical RAM.

[documented: Microsoft 64-bit virtual-address-space documentation]
<https://learn.microsoft.com/en-us/windows/win32/winprog64/virtual-address-space>
also allows a restricted 64-bit executable. Pointer width alone therefore does
not prove that an actual high-address allocation will work.

[design: owner-authorized native compatibility policy, 2026-11-06] NVForever
does not run the original 32-bit Windows executable. Its native query adapts
the capability test instead of claiming to inspect or patch that executable:

| Result | Native meaning |
| --- | --- |
| 0 | Target is not 64-bit; previous unsupported-target result retained. |
| 1 | Native 64-bit target, but the small high-address probe did not pass. |
| 2 | Native 64-bit target and a real allocation's first and last byte addresses are both above `u32::MAX`; those bytes were written and read. |

This is explicitly **not** literal parity with the Windows PE flag test. A
return of 1 does not assert that a PE flag is present. No game executable is
opened, patched or loaded, and no extender plugin is registered by this query.

## Bounded observation

[implemented] One fallible 4096-byte allocation is initialized and touched,
then released. A `OnceLock` caches only the numeric result for this process;
there is no per-frame allocation and no multi-gigabyte reservation. Allocation
failure or an address inside the 32-bit range yields 1. The probe does not
establish memory availability, performance, the original patch's presence or
general TTW compatibility. TTW's own message and predicate remain unchanged.

[test] The existing synthetic parser-to-Game/VM regression calls the actual
query, validates its native result domain, and calls it again for consistency.
It does not reproduce the allocator algorithm or modify game data.
