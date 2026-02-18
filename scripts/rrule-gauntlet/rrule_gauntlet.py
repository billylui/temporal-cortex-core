#!/usr/bin/env python3
"""The RRULE Gauntlet — 10 challenges that break LLMs on calendar math.

Usage:
    python rrule_gauntlet.py verify [--verbose] [--output FILE]
    python rrule_gauntlet.py prompt [--challenge ID]
    python rrule_gauntlet.py test --model MODEL --provider PROVIDER [--output FILE]

Prerequisites:
    pip install temporal-cortex-toon

Optional (for LLM testing):
    pip install openai      # for --provider openai
    pip install anthropic   # for --provider anthropic
"""

from __future__ import annotations

import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path
from textwrap import dedent

# ---------------------------------------------------------------------------
# ANSI colors
# ---------------------------------------------------------------------------

GREEN = "\033[32m"
RED = "\033[31m"
YELLOW = "\033[33m"
CYAN = "\033[36m"
BOLD = "\033[1m"
DIM = "\033[2m"
RESET = "\033[0m"

DIFFICULTY_COLORS = {"Easy": GREEN, "Medium": YELLOW, "Hard": RED}

# ---------------------------------------------------------------------------
# Challenge loader
# ---------------------------------------------------------------------------


def load_challenges() -> list[dict]:
    """Load challenges.json from the same directory as this script."""
    path = Path(__file__).parent / "challenges.json"
    with open(path) as f:
        return json.load(f)


# ---------------------------------------------------------------------------
# Truth Engine verification
# ---------------------------------------------------------------------------


def verify_challenge(challenge: dict) -> list[str]:
    """Expand an RRULE via Truth Engine and return UTC start time strings.

    For 'hardcoded' challenges (e.g. EXDATE), returns the pre-computed answer.
    """
    if challenge.get("verification_mode") == "hardcoded":
        return challenge["correct_answer"]

    from temporal_cortex_toon import expand_rrule

    result_json = expand_rrule(
        challenge["rrule"],
        challenge["dtstart"],
        challenge["duration_minutes"],
        challenge["timezone"],
        challenge.get("until"),
        challenge.get("max_count"),
    )
    events = json.loads(result_json)
    return [e["start"] for e in events]


def verify_all(challenges: list[dict]) -> dict[str, list[str] | str]:
    """Verify all challenges. Returns {id: [utc_starts]} or {id: "ERROR: ..."}."""
    results = {}
    for ch in challenges:
        try:
            results[ch["id"]] = verify_challenge(ch)
        except Exception as e:
            results[ch["id"]] = f"ERROR: {e}"
    return results


# ---------------------------------------------------------------------------
# Answer comparison
# ---------------------------------------------------------------------------


def normalize_datetime(s: str) -> str:
    """Normalize a datetime string for comparison (Z → +00:00, strip whitespace)."""
    s = s.strip()
    if s.endswith("Z"):
        s = s[:-1] + "+00:00"
    return s


def compare_answers(expected: list[str], actual: list[str]) -> dict:
    """Compare expected vs actual datetime lists (order-sensitive)."""
    norm_expected = [normalize_datetime(e) for e in expected]
    norm_actual = [normalize_datetime(a) for a in actual]

    matching = sum(
        1 for e, a in zip(norm_expected, norm_actual) if e == a
    )
    missing = [e for e in norm_expected if e not in norm_actual]
    extra = [a for a in norm_actual if a not in norm_expected]

    return {
        "correct": norm_expected == norm_actual,
        "expected_count": len(norm_expected),
        "actual_count": len(norm_actual),
        "matching": matching,
        "missing": missing,
        "extra": extra,
    }


# ---------------------------------------------------------------------------
# LLM prompt generation
# ---------------------------------------------------------------------------

SYSTEM_PROMPT = dedent("""\
    You are a calendar computation expert. You will be given recurrence rule
    (RRULE) challenges per RFC 5545. For each challenge, compute the exact
    UTC start times of the specified recurring events.

    Rules:
    - Output ONLY a JSON array of UTC datetime strings in RFC 3339 format.
    - Use the +00:00 suffix (not Z).
    - Account for DST transitions, leap years, and timezone offsets.
    - Double-check your work: count the occurrences carefully.

    Example output format:
    ["2026-03-01T07:00:00+00:00", "2026-03-08T07:00:00+00:00"]
""")


def build_prompt(challenge: dict) -> str:
    """Build the user prompt for a single challenge."""
    parts = [
        f"Challenge: {challenge['name']}",
        "",
        challenge["question"],
        "",
        "Technical details:",
        f"  RRULE: {challenge['rrule']}",
        f"  DTSTART: {challenge['dtstart']} (local time in the specified timezone)",
        f"  Timezone: {challenge['timezone']}",
        f"  Duration: {challenge['duration_minutes']} minutes",
    ]
    if challenge.get("until"):
        parts.append(f"  UNTIL: {challenge['until']} (local time in the specified timezone)")
    if challenge.get("exdates"):
        parts.append(f"  EXDATE: {', '.join(challenge['exdates'])}")
    parts.append("")
    parts.append("Return ONLY the JSON array of UTC start times.")
    return "\n".join(parts)


# ---------------------------------------------------------------------------
# LLM runner
# ---------------------------------------------------------------------------


def run_llm(provider: str, model: str, system: str, user: str) -> str:
    """Call an LLM API and return the raw response text."""
    if provider == "openai":
        from openai import OpenAI

        client = OpenAI()  # uses OPENAI_API_KEY env var
        response = client.chat.completions.create(
            model=model,
            messages=[
                {"role": "system", "content": system},
                {"role": "user", "content": user},
            ],
            temperature=0,
        )
        return response.choices[0].message.content
    elif provider == "anthropic":
        from anthropic import Anthropic

        client = Anthropic()  # uses ANTHROPIC_API_KEY env var
        response = client.messages.create(
            model=model,
            max_tokens=4096,
            system=system,
            messages=[{"role": "user", "content": user}],
        )
        return response.content[0].text
    else:
        raise ValueError(f"Unknown provider: {provider}")


def parse_llm_response(response: str) -> list[str]:
    """Extract a JSON array of datetime strings from an LLM response.

    Handles raw JSON, markdown code fences, and surrounding prose.
    """
    text = response.strip()

    # Strip markdown code fences
    if "```" in text:
        lines = text.split("\n")
        in_block = False
        block_lines: list[str] = []
        for line in lines:
            if line.strip().startswith("```"):
                if in_block:
                    break  # end of first code block
                in_block = True
                continue
            if in_block:
                block_lines.append(line)
        if block_lines:
            text = "\n".join(block_lines).strip()

    # Find the JSON array
    start = text.find("[")
    end = text.rfind("]")
    if start != -1 and end != -1:
        text = text[start : end + 1]

    return json.loads(text)


# ---------------------------------------------------------------------------
# Terminal output
# ---------------------------------------------------------------------------


def print_header() -> None:
    print(f"\n{BOLD}{'=' * 60}{RESET}")
    print(f"{BOLD}  THE RRULE GAUNTLET{RESET}")
    print(f"{DIM}  10 challenges that break LLMs on calendar math{RESET}")
    print(f"{BOLD}{'=' * 60}{RESET}\n")


def print_challenge_result(
    challenge: dict, comparison: dict, verbose: bool = False
) -> None:
    status = f"{GREEN}PASS{RESET}" if comparison["correct"] else f"{RED}FAIL{RESET}"
    diff = challenge["difficulty"]
    diff_color = DIFFICULTY_COLORS.get(diff, RESET)

    print(f"  [{status}] {challenge['name']} {DIM}({diff_color}{diff}{RESET}{DIM}){RESET}")

    if not comparison["correct"] or verbose:
        print(
            f"        Expected {comparison['expected_count']} events, "
            f"got {comparison['actual_count']}"
        )
        print(f"        Matching: {comparison['matching']}/{comparison['expected_count']}")
        if comparison["missing"]:
            print(f"        {RED}Missing: {comparison['missing']}{RESET}")
        if comparison["extra"]:
            print(f"        {YELLOW}Extra:   {comparison['extra']}{RESET}")

    if not comparison["correct"] and "why_llms_fail" in challenge:
        print(f"        {DIM}Why: {challenge['why_llms_fail']}{RESET}")


def print_summary(total: int, passed: int) -> None:
    pct = (passed / total * 100) if total > 0 else 0
    color = GREEN if pct == 100 else YELLOW if pct >= 50 else RED
    print(f"\n{BOLD}  Score: {color}{passed}/{total} ({pct:.0f}%){RESET}")

    if pct == 100:
        print(f"  {GREEN}Perfect score! This model handles calendar math correctly.{RESET}")
    elif pct >= 70:
        print(f"  {YELLOW}Good but not reliable for production calendar operations.{RESET}")
    elif pct >= 40:
        print(f"  {RED}Significant gaps in calendar reasoning.{RESET}")
    else:
        print(f"  {RED}This model should not be trusted with calendar math.{RESET}")
    print()


# ---------------------------------------------------------------------------
# Commands
# ---------------------------------------------------------------------------


def cmd_verify(args: argparse.Namespace) -> None:
    """Verify all challenges against the Truth Engine."""
    challenges = load_challenges()
    print_header()
    print(f"  {CYAN}Verifying challenges against Truth Engine...{RESET}\n")

    results = verify_all(challenges)
    errors = 0

    for ch in challenges:
        answer = results[ch["id"]]
        if isinstance(answer, str) and answer.startswith("ERROR"):
            print(f"  [{RED}ERR {RESET}] {ch['name']}: {answer}")
            errors += 1
        else:
            mode = "hardcoded" if ch.get("verification_mode") == "hardcoded" else "engine"
            print(f"  [{GREEN} OK {RESET}] {ch['name']}: {len(answer)} events ({mode})")
            if args.verbose:
                for dt in answer:
                    print(f"           {dt}")

    if args.output and errors == 0:
        for ch in challenges:
            val = results[ch["id"]]
            if not isinstance(val, str):
                ch["correct_answer"] = val
        with open(args.output, "w") as f:
            json.dump(challenges, f, indent=2)
            f.write("\n")
        print(f"\n  {CYAN}Written to {args.output}{RESET}")
    elif errors > 0:
        print(f"\n  {RED}{errors} challenge(s) failed verification.{RESET}")
        sys.exit(1)

    print()


def cmd_prompt(args: argparse.Namespace) -> None:
    """Print LLM prompts for challenges."""
    challenges = load_challenges()
    target = (
        [ch for ch in challenges if ch["id"] == args.challenge]
        if args.challenge
        else challenges
    )

    if not target:
        print(f"  {RED}Challenge '{args.challenge}' not found.{RESET}")
        print(f"  Available: {', '.join(ch['id'] for ch in challenges)}")
        sys.exit(1)

    for ch in target:
        print(f"\n{'=' * 60}")
        print(f"Challenge: {ch['name']} (difficulty: {ch['difficulty']})")
        print(f"{'=' * 60}")
        print(build_prompt(ch))


def cmd_test(args: argparse.Namespace) -> None:
    """Run an LLM against the gauntlet."""
    challenges = load_challenges()
    target = (
        [ch for ch in challenges if ch["id"] == args.challenge]
        if args.challenge
        else challenges
    )

    if not target:
        print(f"  {RED}Challenge '{args.challenge}' not found.{RESET}")
        sys.exit(1)

    # Ensure correct answers are populated
    for ch in target:
        if not ch.get("correct_answer"):
            ch["correct_answer"] = verify_challenge(ch)

    print_header()
    print(f"  {CYAN}Model: {args.model} ({args.provider}){RESET}")
    print(f"  {CYAN}Challenges: {len(target)}{RESET}\n")

    results: list[dict] = []
    passed = 0

    for ch in target:
        prompt = build_prompt(ch)
        llm_answer: list[str] = []
        response = ""

        try:
            response = run_llm(args.provider, args.model, SYSTEM_PROMPT, prompt)
            llm_answer = parse_llm_response(response)
            comparison = compare_answers(ch["correct_answer"], llm_answer)
        except Exception as e:
            comparison = {
                "correct": False,
                "expected_count": len(ch["correct_answer"]),
                "actual_count": 0,
                "matching": 0,
                "missing": ch["correct_answer"],
                "extra": [],
            }
            response = f"ERROR: {e}"

        print_challenge_result(ch, comparison, args.verbose)

        if comparison["correct"]:
            passed += 1

        results.append(
            {
                "id": ch["id"],
                "name": ch["name"],
                "difficulty": ch["difficulty"],
                "expected": ch["correct_answer"],
                "actual": llm_answer,
                "raw_response": response,
                **comparison,
            }
        )

    print_summary(len(target), passed)

    if args.output:
        output = {
            "model": args.model,
            "provider": args.provider,
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "score": f"{passed}/{len(target)}",
            "challenges": results,
        }
        with open(args.output, "w") as f:
            json.dump(output, f, indent=2)
            f.write("\n")
        print(f"  {CYAN}Results saved to {args.output}{RESET}\n")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main() -> None:
    parser = argparse.ArgumentParser(
        description="The RRULE Gauntlet — 10 challenges that break LLMs on calendar math"
    )
    sub = parser.add_subparsers(dest="command")

    # verify
    p_verify = sub.add_parser("verify", help="Verify challenges against Truth Engine")
    p_verify.add_argument("--verbose", "-v", action="store_true")
    p_verify.add_argument("--output", "-o", help="Write verified challenges to file")

    # prompt
    p_prompt = sub.add_parser("prompt", help="Print LLM prompts")
    p_prompt.add_argument("--challenge", help="Specific challenge ID")

    # test
    p_test = sub.add_parser("test", help="Run an LLM against the gauntlet")
    p_test.add_argument("--model", default="gpt-4o", help="Model name (default: gpt-4o)")
    p_test.add_argument(
        "--provider",
        default="openai",
        choices=["openai", "anthropic"],
        help="LLM provider (default: openai)",
    )
    p_test.add_argument("--challenge", help="Specific challenge ID")
    p_test.add_argument("--output", "-o", help="Save results to JSON file")
    p_test.add_argument("--verbose", "-v", action="store_true")

    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        sys.exit(1)

    if args.command == "verify":
        cmd_verify(args)
    elif args.command == "prompt":
        cmd_prompt(args)
    elif args.command == "test":
        cmd_test(args)


if __name__ == "__main__":
    main()
