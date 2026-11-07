//! macOS capacity and native pressure are different measurements.
//!
//! `hw.memsize` supplies physical capacity for the base cache limits. Dispatch's
//! public MEMORYPRESSURE source supplies the runtime policy; neither free pages
//! nor sysinfo's platform-specific `available_memory` is treated as pressure.
//! Apple SDK dispatch/source.h:135–146,232–257 (macOS 10.9+); os/proc.h:85 marks
//! `os_proc_available_memory` unavailable on macOS. Callback work is one atomic
//! publication; the existing two-second Bevy sweep applies the cache policy.

use bevy::prelude::Resource;
use core::ffi::c_void;
use core::ptr::NonNull;
use dispatch2::{
    DispatchObject, DispatchQoS, DispatchQueue, DispatchRetained, DispatchSource,
    GlobalQueueIdentifier, _dispatch_source_type_memorypressure, dispatch_source_type_t,
};
use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

// Public Dispatch flag values, not the different numeric XNU pressure levels.
const NORMAL: usize = 0x01;
const WARNING: usize = 0x02;
const CRITICAL: usize = 0x04;
const EVENTS: usize = NORMAL | WARNING | CRITICAL;
const CLOSED: u8 = 254;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(super) enum Pressure {
    Unknown = 0,
    Normal = 1,
    Warning = 2,
    Critical = 4,
    Unavailable = 255,
}

impl Pressure {
    fn from_flags(flags: usize) -> Self {
        // Coalesced elevated flags take priority. Unknown bits must never turn
        // a future or malformed event into a false all-clear.
        if flags & CRITICAL != 0 {
            Self::Critical
        } else if flags & WARNING != 0 {
            Self::Warning
        } else if flags == NORMAL {
            Self::Normal
        } else {
            Self::Unknown
        }
    }

    fn from_atomic(value: u8) -> Self {
        match value {
            1 => Self::Normal,
            2 => Self::Warning,
            4 => Self::Critical,
            CLOSED | 255 => Self::Unavailable,
            _ => Self::Unknown,
        }
    }

    pub(super) fn cache_pressed(self, was: bool) -> bool {
        match self {
            Self::Unknown | Self::Unavailable => was,
            Self::Normal => false,
            Self::Warning | Self::Critical => true,
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Normal => "normal",
            Self::Warning => "warning",
            Self::Critical => "critical",
            Self::Unavailable => "unavailable",
        }
    }

    pub(super) fn description(self) -> &'static str {
        match self {
            Self::Unknown => "unknown (observer active; awaiting a native event; previous cache policy retained)",
            Self::Unavailable => "unavailable (native observer not active; previous cache policy retained)",
            pressure => pressure.label(),
        }
    }
}

// The checked binding is deliberate: dispatch2 0.3.1's `DispatchSource::new`
// panics on a native NULL. An unavailable sensor is a reported state instead.
unsafe extern "C" {
    fn dispatch_source_create(
        source_type: dispatch_source_type_t,
        handle: usize,
        mask: usize,
        queue: Option<&DispatchQueue>,
    ) -> *mut DispatchSource;
}

/// Only this owner retains the native source. A native callback never touches ECS.
#[derive(Resource)]
pub(super) struct Observer {
    source: Option<DispatchRetained<DispatchSource>>,
    state: Arc<AtomicU8>,
}

struct Context {
    state: Arc<AtomicU8>,
    // Non-owning: retaining the source here would form source -> context -> source.
    // Dispatch keeps the source valid during its event/cancellation callbacks.
    source: NonNull<DispatchSource>,
}

impl Observer {
    pub(super) fn new() -> Self {
        let state = Arc::new(AtomicU8::new(Pressure::Unknown as u8));
        let queue = DispatchQueue::global_queue(GlobalQueueIdentifier::QualityOfService(DispatchQoS::Utility));
        // SAFETY: the public static source type has process lifetime, memory
        // pressure uses handle 0 and the documented 1/2/4 mask. The target queue
        // is live. This checked call returns a new +1 native ownership or NULL.
        let raw = unsafe {
            dispatch_source_create(
                core::ptr::addr_of!(_dispatch_source_type_memorypressure).cast_mut(),
                0,
                EVENTS,
                Some(&queue),
            )
        };
        let Some(raw) = NonNull::new(raw) else {
            state.store(Pressure::Unavailable as u8, Ordering::Release);
            return Self { source: None, state };
        };
        // SAFETY: a non-null successful dispatch_source_create returned +1.
        let source = unsafe { DispatchRetained::from_raw(raw) };
        let context = Box::into_raw(Box::new(Context { state: Arc::clone(&state), source: raw }));
        // SAFETY: context is a stable Box transferred to the native cancellation
        // handler, which runs only after the event handler has returned. Set it
        // before registering either handler, and never replace it afterwards.
        unsafe { source.set_context(context.cast()) };
        source.set_event_handler_f(on_event);
        source.set_cancel_handler_f(on_cancel);
        // Created sources must be activated before disposal. Everything after
        // successful creation above is non-fallible setup; no source is suspended.
        source.activate();
        Self { source: Some(source), state }
    }

    pub(super) fn current(&self) -> Pressure {
        Pressure::from_atomic(self.state.load(Ordering::Acquire))
    }

    pub(super) fn description(&self) -> &'static str {
        self.current().description()
    }
}

impl Drop for Observer {
    fn drop(&mut self) {
        // A callback already in flight must not publish a new state after close.
        self.state.store(CLOSED, Ordering::Release);
        if let Some(source) = self.source.take() {
            // Cancellation is asynchronous. Do not free Context here or wait
            // on a queue from Bevy's thread; on_cancel owns the eventual free.
            source.cancel();
        }
    }
}

fn publish_event(state: &AtomicU8, pressure: Pressure) {
    if pressure == Pressure::Unknown {
        return;
    }
    let _ = state.try_update(Ordering::AcqRel, Ordering::Acquire, |old| {
        (old != CLOSED && old != Pressure::Unavailable as u8).then_some(pressure as u8)
    });
}

extern "C" fn on_event(raw_context: *mut c_void) {
    let Some(context) = NonNull::new(raw_context.cast::<Context>()) else { return };
    // SAFETY: this is our unchanged source context. Dispatch serializes this
    // source's event handlers and postpones on_cancel until this callback ends.
    let context = unsafe { context.as_ref() };
    // SAFETY: the non-owning source remains native-owned for the callback's
    // duration; this only queries the coalesced event flags.
    let flags = unsafe { context.source.as_ref() }.data();
    publish_event(&context.state, Pressure::from_flags(flags));
}

extern "C" fn on_cancel(raw_context: *mut c_void) {
    let Some(context) = NonNull::new(raw_context.cast::<Context>()) else { return };
    // SAFETY: this callback is installed exactly once, only one owner cancels,
    // and no event handler can still access the context when it runs. It alone
    // reclaims the Box transferred during setup; there is no separate finalizer.
    drop(unsafe { Box::from_raw(context.as_ptr()) });
}

fn checked_total_bytes(result: libc::c_int, width: usize, total: u64) -> Option<u64> {
    (result == 0 && width == core::mem::size_of::<u64>() && total > 0).then_some(total)
}

pub(super) fn total_mb() -> Option<u64> {
    let mut total = 0_u64;
    let mut width = core::mem::size_of::<u64>();
    // SAFETY: hw.memsize returns a uint64 value into our writable buffer with
    // its exact byte width. newp is NULL/newlen 0, so this cannot modify sysctl.
    let result = unsafe {
        libc::sysctlbyname(c"hw.memsize".as_ptr(), (&mut total as *mut u64).cast(), &mut width, core::ptr::null_mut(), 0)
    };
    checked_total_bytes(result, width, total).map(|bytes| bytes / super::MB).filter(|mb| *mb > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_flags_are_not_kernel_level_numbers() {
        assert_eq!(Pressure::from_flags(0), Pressure::Unknown);
        assert_eq!(Pressure::from_flags(NORMAL), Pressure::Normal);
        assert_eq!(Pressure::from_flags(WARNING), Pressure::Warning);
        assert_eq!(Pressure::from_flags(CRITICAL), Pressure::Critical);
        assert_eq!(Pressure::from_flags(EVENTS), Pressure::Critical);
        assert_eq!(Pressure::from_flags(NORMAL | WARNING), Pressure::Warning);
        assert_eq!(Pressure::from_flags(8), Pressure::Unknown);
        assert_eq!(Pressure::from_flags(NORMAL | 8), Pressure::Unknown);
    }

    #[test]
    fn unknown_preserves_policy_and_only_normal_releases_it() {
        for was in [false, true] {
            assert_eq!(Pressure::Unknown.cache_pressed(was), was);
            assert_eq!(Pressure::Unavailable.cache_pressed(was), was);
            assert!(!Pressure::Normal.cache_pressed(was));
            assert!(Pressure::Warning.cache_pressed(was));
            assert!(Pressure::Critical.cache_pressed(was));
        }
    }

    #[test]
    fn absent_or_unknown_event_does_not_forge_an_all_clear() {
        let state = AtomicU8::new(Pressure::Unknown as u8);
        publish_event(&state, Pressure::Unknown);
        assert_eq!(Pressure::from_atomic(state.load(Ordering::Acquire)), Pressure::Unknown);
        publish_event(&state, Pressure::Warning);
        publish_event(&state, Pressure::Unknown);
        assert_eq!(Pressure::from_atomic(state.load(Ordering::Acquire)), Pressure::Warning);
        publish_event(&state, Pressure::Normal);
        assert_eq!(Pressure::from_atomic(state.load(Ordering::Acquire)), Pressure::Normal);
    }

    #[test]
    fn callback_cannot_reopen_a_closed_or_unavailable_observer() {
        for terminal in [CLOSED, Pressure::Unavailable as u8] {
            let state = AtomicU8::new(terminal);
            for pressure in [Pressure::Normal, Pressure::Warning, Pressure::Critical] {
                publish_event(&state, pressure);
                assert_eq!(state.load(Ordering::Acquire), terminal);
            }
        }
    }

    #[test]
    fn physical_capacity_checks_sysctl_width_and_success() {
        let capacity = 24 * 1024 * 1024 * 1024_u64;
        assert_eq!(checked_total_bytes(0, 8, capacity), Some(capacity));
        assert_eq!(checked_total_bytes(-1, 8, capacity), None);
        assert_eq!(checked_total_bytes(0, 4, capacity), None);
        assert_eq!(checked_total_bytes(0, 8, 0), None);
    }

    #[test]
    fn observer_is_a_send_sync_bevy_resource_without_raw_context_fields() {
        fn assert_resource<T: Resource + Send + Sync>() {}
        assert_resource::<Observer>();
    }
}
