#!/usr/bin/env python3
"""Check docs/microsoft-store-listing.md against Partner Center field limits.

Run from the repo root:  python docs/check-store-limits.py
Exits non-zero if any field is over its limit.
"""
import pathlib
import re
import sys

# Partner Center limits: section heading prefix -> (per-item limit, max items)
LIMITS = {
    "Product name": (256, 1),
    "Short description": (1000, 1),
    "Description": (10000, 1),
    "Product features": (200, 20),
    "Search terms": (30, 7),
    "What's new in this version": (1500, 1),
    "Additional system requirements": (200, 11),
    "Copyright and trademark info": (200, 1),
}

listing = pathlib.Path(__file__).with_name("microsoft-store-listing.md")
text = listing.read_text(encoding="utf-8")

# Split on "## " headings, keeping the heading with its body.
sections = re.split(r"^## ", text, flags=re.M)[1:]
failures = []

for section in sections:
    heading = section.splitlines()[0]
    name = next((k for k in LIMITS if heading.startswith(k)), None)
    if name is None:
        continue
    limit, max_items = LIMITS[name]

    blocks = re.findall(r"^```\n(.*?)^```", section, flags=re.M | re.S)
    # Search terms live as one block of newline-separated terms.
    if name == "Search terms":
        items = [t for b in blocks for t in b.strip().splitlines() if t.strip()]
    else:
        items = [b.strip() for b in blocks if b.strip()]

    if not items:
        continue

    longest = max(len(i) for i in items)
    status = "ok"
    if len(items) > max_items:
        failures.append(f"{name}: {len(items)} items, max {max_items}")
        status = "TOO MANY"
    over = [i for i in items if len(i) > limit]
    if over:
        failures.append(f"{name}: {len(over)} item(s) exceed {limit} chars")
        status = "OVER LIMIT"

    print(f"{name:34} {len(items):>2} item(s)  longest {longest:>5}/{limit:<5} {status}")

if failures:
    print("\nFAILED:")
    for f in failures:
        print("  -", f)
    sys.exit(1)

print("\nAll fields within Partner Center limits.")
