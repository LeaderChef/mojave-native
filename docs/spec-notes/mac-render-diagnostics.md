# macOS render diagnostics device policy

## Evidence and scope

[observed] On 2026-11-06, the native Apple M4 Pro/Metal build `91d82f6d137f`
lost its device during a 1920x1080 Goodsprings play test with
`--log-frame-times 5`. Metal reported `Cannot allocate sample buffer` while
creating a counter sample buffer; the following screenshot readback panicked.
The otherwise matched run without this flag completed 50 gameplay seconds and
produced a world image. Private evidence: `mac-graphics-gs-vanilla-01` and `-02`.
These observations isolate a diagnostics-related failure; they do not prove
that every Metal device or timestamp use is broken.

[observed: dependency source] Bevy 0.20.1 `diagnostic/internal.rs` allocates
`MAX_TIMESTAMP_QUERIES` (256) when the device exposes `TIMESTAMP_QUERY`, even
when inside-encoder/pass timestamps are unavailable. wgpu-hal 30.0.1
`metal/device.rs::create_query_set` requests an `MTLCounterSampleBuffer` and
returns `DeviceError::Unexpected` if this allocation fails. `--log-frame-times`
enables `RenderDiagnosticsPlugin` independently of the frame-telemetry GPU aid.

[documented] Bevy's device settings expose `disabled_features`; its renderer
applies this mask before explicit `features`. The public diagnostics API keeps
CPU render-span times when GPU queries are unavailable:
<https://docs.rs/bevy_render/0.20.1/bevy_render/diagnostic/struct.RenderDiagnosticsPlugin.html>.
An earlier, separately fixed Metal diagnostics failure is recorded at
<https://github.com/bevyengine/bevy/issues/21167>; it is not evidence that this
allocation failure has the same cause.

## Policy

[design] Before creating the macOS device, exclude the timestamp-query feature
family from automatic and explicitly required features. Preserve existing
disabled features and all unrelated renderer settings. Use this device setup
for ordinary play and background tests alike. Other target platforms keep their
original device feature requests. No dependency/vendor source is modified.

[design] `RenderDiagnosticsPlugin` remains registered for render CPU timings;
main-world frame telemetry remains available. GPU elapsed time with no samples
is explicitly unavailable, never a measured zero. A GPU-enabled telemetry
request records its existing unsupported-query sentinel for this restricted
device; that describes this run's configured device, not a hardware capability
assessment. No image-quality or simulation option is disabled by this policy.

## Acceptance

[design] Keep the failing native run. Repeat the original frame-time-enabled
case on the new binary, requiring a full-duration native exit, real world PNG,
frame-time output, explicit GPU timing unavailability and no counter/device-loss
error. Separately request GPU frame telemetry and require null timing plus an
unsupported-query reason. Matched vanilla/enhanced pictures and main-loop
percentiles are offscreen observations, not visible presentation FPS, GPU time
or input-latency proof.
