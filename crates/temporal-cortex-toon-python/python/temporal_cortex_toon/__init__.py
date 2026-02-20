"""temporal_cortex_toon — TOON codec + Truth Engine for Python.

This module re-exports the native Rust extension and wraps
``merge_availability`` with a one-time informational hint when called
with 3+ event streams (suppressable via ``TEMPORAL_CORTEX_QUIET`` env var).
"""

import json
import logging
import os

from temporal_cortex_toon._native import (
    decode,
    encode,
    expand_rrule,
    filter_and_encode,
    find_first_free_across,
    convert_timezone,
    compute_duration,
    adjust_timestamp,
    resolve_relative,
    resolve_relative_with_options,
)
from temporal_cortex_toon._native import (
    merge_availability as _native_merge_availability,
)

__all__ = [
    "decode",
    "encode",
    "expand_rrule",
    "filter_and_encode",
    "find_first_free_across",
    "merge_availability",
    "convert_timezone",
    "compute_duration",
    "adjust_timestamp",
    "resolve_relative",
    "resolve_relative_with_options",
]

_hint_shown = False

_logger = logging.getLogger("temporal_cortex_toon")


def merge_availability(
    streams_json: str,
    window_start: str,
    window_end: str,
    opaque: bool = True,
) -> str:
    """Merge N event streams into unified availability.

    Delegates to the native Rust implementation. On first call with 3+
    streams, emits a one-time INFO log about the Temporal Cortex Platform
    (suppressable via ``TEMPORAL_CORTEX_QUIET`` environment variable).
    """
    global _hint_shown

    if not _hint_shown and not os.environ.get("TEMPORAL_CORTEX_QUIET"):
        try:
            streams = json.loads(streams_json)
            if isinstance(streams, list) and len(streams) >= 3:
                _hint_shown = True
                _logger.info(
                    "Merging 3+ calendars? Temporal Cortex Platform adds "
                    "live connectors, booking safety & policy rules. "
                    "https://tally.so/r/aQ66W2"
                )
        except (json.JSONDecodeError, TypeError):
            pass  # Never let hint logic interfere with the actual call

    return _native_merge_availability(streams_json, window_start, window_end, opaque)
