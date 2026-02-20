"""Tests for the temporal_cortex_toon Python bindings.

TDD RED phase: These tests define the expected behavior of the PyO3 bindings.
They will fail until the native extension is built and installed.
"""

import json
import logging
import os

import pytest

from temporal_cortex_toon import (
    decode, encode, expand_rrule, filter_and_encode,
    convert_timezone, compute_duration, adjust_timestamp, resolve_relative,
)
import temporal_cortex_toon


# ---------------------------------------------------------------------------
# encode
# ---------------------------------------------------------------------------


class TestEncode:
    """Tests for JSON -> TOON encoding."""

    def test_encode_simple_object(self):
        result = encode('{"name":"Alice","age":30}')
        assert "name: Alice" in result
        assert "age: 30" in result

    def test_encode_nested_object(self):
        result = encode('{"user":{"name":"Bob","active":true}}')
        assert "user" in result
        assert "name: Bob" in result

    def test_encode_array(self):
        result = encode('{"scores":[95,87,92]}')
        assert "scores" in result
        assert "95" in result
        assert "87" in result
        assert "92" in result

    def test_encode_returns_string(self):
        result = encode('{"x":1}')
        assert isinstance(result, str)

    def test_encode_invalid_json_raises(self):
        with pytest.raises(ValueError):
            encode("not json")

    def test_encode_empty_object(self):
        result = encode("{}")
        # Empty object should still produce valid TOON (possibly empty string)
        assert isinstance(result, str)

    def test_encode_null_value(self):
        result = encode('{"key":null}')
        assert "null" in result

    def test_encode_boolean_values(self):
        result = encode('{"yes":true,"no":false}')
        assert "true" in result
        assert "false" in result

    def test_encode_string_with_spaces(self):
        result = encode('{"greeting":"hello world"}')
        assert "hello world" in result


# ---------------------------------------------------------------------------
# decode
# ---------------------------------------------------------------------------


class TestDecode:
    """Tests for TOON -> JSON decoding."""

    def test_decode_simple(self):
        toon = "name: Alice\nage: 30"
        result = decode(toon)
        data = json.loads(result)
        assert data["name"] == "Alice"
        assert data["age"] == 30

    def test_decode_returns_valid_json(self):
        toon = "x: 1"
        result = decode(toon)
        data = json.loads(result)
        assert data["x"] == 1

    def test_decode_nested(self):
        toon = "user:\n  name: Bob\n  active: true"
        result = decode(toon)
        data = json.loads(result)
        assert data["user"]["name"] == "Bob"
        assert data["user"]["active"] is True


# ---------------------------------------------------------------------------
# roundtrip
# ---------------------------------------------------------------------------


class TestRoundtrip:
    """Tests verifying encode -> decode roundtrip fidelity."""

    def test_roundtrip_simple(self):
        original = '{"name":"Alice","scores":[95,87,92]}'
        roundtripped = decode(encode(original))
        assert json.loads(roundtripped) == json.loads(original)

    def test_roundtrip_nested(self):
        original = '{"user":{"name":"Bob","age":25}}'
        roundtripped = decode(encode(original))
        assert json.loads(roundtripped) == json.loads(original)

    def test_roundtrip_preserves_types(self):
        original = '{"s":"hello","n":42,"f":3.14,"b":true,"nil":null}'
        roundtripped = decode(encode(original))
        data = json.loads(roundtripped)
        assert isinstance(data["s"], str)
        assert isinstance(data["n"], int)
        assert isinstance(data["f"], float)
        assert isinstance(data["b"], bool)
        assert data["nil"] is None


# ---------------------------------------------------------------------------
# filter_and_encode
# ---------------------------------------------------------------------------


class TestFilterAndEncode:
    """Tests for semantic filtering + TOON encoding."""

    def test_filter_removes_specified_fields(self):
        json_str = '{"name":"Alice","etag":"abc","kind":"event"}'
        result = filter_and_encode(json_str, ["etag", "kind"])
        assert "name: Alice" in result
        assert "etag" not in result
        assert "kind" not in result

    def test_filter_empty_patterns_preserves_all(self):
        json_str = '{"name":"Alice","etag":"abc"}'
        result = filter_and_encode(json_str, [])
        assert "name" in result
        assert "etag" in result

    def test_filter_wildcard_pattern(self):
        json_str = '{"items":[{"name":"Event","etag":"x"}]}'
        result = filter_and_encode(json_str, ["*.etag"])
        assert "name" in result
        assert "etag" not in result

    def test_filter_invalid_json_raises(self):
        with pytest.raises(ValueError):
            filter_and_encode("bad json", ["field"])


# ---------------------------------------------------------------------------
# expand_rrule
# ---------------------------------------------------------------------------


class TestExpandRrule:
    """Tests for RRULE expansion via the truth-engine."""

    def test_expand_daily_count(self):
        result = expand_rrule(
            "FREQ=DAILY;COUNT=3",
            "2026-02-17T14:00:00",
            60,
            "America/Los_Angeles",
            None,
            None,
        )
        events = json.loads(result)
        assert len(events) == 3

    def test_expand_returns_start_and_end(self):
        result = expand_rrule(
            "FREQ=DAILY;COUNT=1",
            "2026-02-17T14:00:00",
            60,
            "America/Los_Angeles",
            None,
            None,
        )
        events = json.loads(result)
        assert len(events) == 1
        assert "start" in events[0]
        assert "end" in events[0]

    def test_expand_with_until(self):
        result = expand_rrule(
            "FREQ=DAILY;COUNT=3",
            "2026-02-17T14:00:00",
            60,
            "UTC",
            "2026-12-31T23:59:59",
            None,
        )
        events = json.loads(result)
        assert len(events) == 3

    def test_expand_with_max_count(self):
        result = expand_rrule(
            "FREQ=DAILY",
            "2026-02-17T14:00:00",
            30,
            "UTC",
            None,
            5,
        )
        events = json.loads(result)
        assert len(events) == 5

    def test_expand_invalid_rrule_raises(self):
        with pytest.raises(ValueError):
            expand_rrule("", "2026-02-17T14:00:00", 60, "UTC", None, None)

    def test_expand_invalid_timezone_raises(self):
        with pytest.raises(ValueError):
            expand_rrule(
                "FREQ=DAILY;COUNT=1",
                "2026-02-17T14:00:00",
                60,
                "Not/A/Timezone",
                None,
                None,
            )

    def test_expand_weekly(self):
        result = expand_rrule(
            "FREQ=WEEKLY;COUNT=4;BYDAY=MO",
            "2026-02-16T09:00:00",
            45,
            "America/New_York",
            None,
            None,
        )
        events = json.loads(result)
        assert len(events) == 4
        # Verify duration: end - start should be 45 minutes
        from datetime import datetime
        start = datetime.fromisoformat(events[0]["start"].replace("+00:00", "+00:00"))
        end = datetime.fromisoformat(events[0]["end"].replace("+00:00", "+00:00"))
        delta = (end - start).total_seconds()
        assert delta == 45 * 60


# ---------------------------------------------------------------------------
# merge_availability hint
# ---------------------------------------------------------------------------


class TestMergeAvailabilityHint:
    """Tests for the 3+ stream contextual hint in merge_availability."""

    @staticmethod
    def _make_streams(n: int) -> str:
        """Build N empty event streams as a JSON string."""
        streams = [{"stream_id": f"cal-{i}", "events": []} for i in range(n)]
        return json.dumps(streams)

    def test_hint_fires_on_3_streams(self, caplog):
        temporal_cortex_toon._hint_shown = False
        os.environ.pop("TEMPORAL_CORTEX_QUIET", None)

        with caplog.at_level(logging.INFO, logger="temporal_cortex_toon"):
            temporal_cortex_toon.merge_availability(
                self._make_streams(3),
                "2026-03-17T08:00:00+00:00",
                "2026-03-18T00:00:00+00:00",
                True,
            )
        assert "tally.so/r/aQ66W2" in caplog.text

    def test_hint_does_not_fire_on_2_streams(self, caplog):
        temporal_cortex_toon._hint_shown = False
        os.environ.pop("TEMPORAL_CORTEX_QUIET", None)

        with caplog.at_level(logging.INFO, logger="temporal_cortex_toon"):
            temporal_cortex_toon.merge_availability(
                self._make_streams(2),
                "2026-03-17T08:00:00+00:00",
                "2026-03-18T00:00:00+00:00",
                True,
            )
        assert "tally.so" not in caplog.text

    def test_hint_fires_only_once(self, caplog):
        temporal_cortex_toon._hint_shown = False
        os.environ.pop("TEMPORAL_CORTEX_QUIET", None)

        with caplog.at_level(logging.INFO, logger="temporal_cortex_toon"):
            temporal_cortex_toon.merge_availability(
                self._make_streams(4),
                "2026-03-17T08:00:00+00:00",
                "2026-03-18T00:00:00+00:00",
                True,
            )
            first_count = caplog.text.count("tally.so/r/aQ66W2")
            temporal_cortex_toon.merge_availability(
                self._make_streams(5),
                "2026-03-17T08:00:00+00:00",
                "2026-03-18T00:00:00+00:00",
                True,
            )
        assert caplog.text.count("tally.so/r/aQ66W2") == first_count

    def test_hint_suppressed_by_env_var(self, caplog):
        temporal_cortex_toon._hint_shown = False
        os.environ["TEMPORAL_CORTEX_QUIET"] = "1"

        try:
            with caplog.at_level(logging.INFO, logger="temporal_cortex_toon"):
                temporal_cortex_toon.merge_availability(
                    self._make_streams(3),
                    "2026-03-17T08:00:00+00:00",
                    "2026-03-18T00:00:00+00:00",
                    True,
                )
            assert "tally.so" not in caplog.text
        finally:
            os.environ.pop("TEMPORAL_CORTEX_QUIET", None)


# ---------------------------------------------------------------------------
# convert_timezone
# ---------------------------------------------------------------------------


class TestConvertTimezone:
    """Tests for timezone conversion."""

    def test_convert_utc_to_eastern(self):
        result = json.loads(convert_timezone("2026-03-15T14:00:00Z", "America/New_York"))
        assert result["timezone"] == "America/New_York"
        assert "10:00:00" in result["local"]
        assert result["dst_active"] is True  # March = EDT

    def test_convert_invalid_timezone_raises(self):
        with pytest.raises(ValueError):
            convert_timezone("2026-03-15T14:00:00Z", "Invalid/Zone")


# ---------------------------------------------------------------------------
# compute_duration
# ---------------------------------------------------------------------------


class TestComputeDuration:
    """Tests for duration computation."""

    def test_duration_8_hours(self):
        result = json.loads(compute_duration("2026-03-16T09:00:00Z", "2026-03-16T17:00:00Z"))
        assert result["total_seconds"] == 28800
        assert result["hours"] == 8
        assert result["days"] == 0

    def test_duration_invalid_raises(self):
        with pytest.raises(ValueError):
            compute_duration("not-a-date", "2026-03-16T17:00:00Z")


# ---------------------------------------------------------------------------
# adjust_timestamp
# ---------------------------------------------------------------------------


class TestAdjustTimestamp:
    """Tests for timestamp adjustment."""

    def test_adjust_add_hours(self):
        result = json.loads(adjust_timestamp("2026-03-16T10:00:00Z", "+2h", "UTC"))
        assert "12:00:00" in result["adjusted_utc"]
        assert result["adjustment_applied"] == "+2h"

    def test_adjust_invalid_format_raises(self):
        with pytest.raises(ValueError):
            adjust_timestamp("2026-03-16T10:00:00Z", "2h", "UTC")


# ---------------------------------------------------------------------------
# resolve_relative
# ---------------------------------------------------------------------------


class TestResolveRelative:
    """Tests for relative time expression resolution."""

    def test_resolve_tomorrow(self):
        # Anchor: Wed Feb 18, 2026 14:30 UTC
        result = json.loads(resolve_relative("2026-02-18T14:30:00+00:00", "tomorrow", "UTC"))
        assert "2026-02-19" in result["resolved_utc"]
        assert "00:00:00" in result["resolved_utc"]

    def test_resolve_next_tuesday_at_2pm(self):
        result = json.loads(resolve_relative("2026-02-18T14:30:00+00:00", "next Tuesday at 2pm", "UTC"))
        assert "2026-02-24" in result["resolved_utc"]
        assert "14:00:00" in result["resolved_utc"]

    def test_resolve_unparseable_raises(self):
        with pytest.raises(ValueError):
            resolve_relative("2026-02-18T14:30:00+00:00", "gobbledygook", "UTC")
