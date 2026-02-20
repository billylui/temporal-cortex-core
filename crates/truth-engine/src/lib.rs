//! # truth-engine
//!
//! Deterministic calendar computation for AI agents.
//!
//! The Truth Engine provides mathematically correct recurrence rule expansion,
//! conflict detection, free/busy computation, multi-calendar availability
//! merging, and temporal computation that LLMs cannot reliably perform via
//! inference.
//!
//! ## Modules
//!
//! - [`expander`] — RRULE string → list of concrete datetime instances
//! - [`dst`] — DST transition policies (skip, shift, etc.)
//! - [`conflict`] — Detect overlapping events in expanded schedules
//! - [`freebusy`] — Compute free time slots from event lists
//! - [`availability`] — Merge N event streams into unified busy/free with privacy control
//! - [`temporal`] — Timezone conversion, duration computation, timestamp adjustment, relative datetime resolution
//! - [`error`] — Error types

pub mod availability;
pub mod conflict;
pub mod dst;
pub mod error;
pub mod expander;
pub mod freebusy;
pub mod temporal;

pub use availability::{
    find_first_free_across, merge_availability, BusyBlock, EventStream, PrivacyLevel,
    UnifiedAvailability,
};
pub use conflict::find_conflicts;
pub use error::TruthError;
pub use expander::{expand_rrule, expand_rrule_with_exdates, ExpandedEvent};
pub use freebusy::{find_free_slots, FreeSlot};
pub use temporal::{
    adjust_timestamp, compute_duration, convert_timezone, resolve_relative,
    resolve_relative_with_options, AdjustedTimestamp, ConvertedDatetime, DurationInfo,
    ResolveOptions, ResolvedDatetime, WeekStartDay,
};
