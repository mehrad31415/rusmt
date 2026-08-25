"""Every number the paper reports, computed from the framework ledger.

usage: report.py <ledger.jsonl> [diff.json]

Prints the macro values for specops/main.tex plus the breakdowns the prose
quotes, so no number in the paper is transcribed by hand.
"""
import json
import os
import re
import sys
from collections import Counter

ROOT = __file__.rsplit("/", 2)[0]  # repo root, from specops/


def catalog():
    text = open(f"{ROOT}/lang/src/certify.rs", encoding="utf-8").read()
    m = re.search(r"TOML_NAMED_MARKERS[^=]*=\s*\[(.*?)\];", text, re.S)
    return re.findall(r'"([^"]+)"', m.group(1))


def load(path):
    recs = {}
    for line in open(path, encoding="utf-8"):
        try:
            r = json.loads(line)
        except ValueError:
            continue
        recs[r["marker"]] = r          # last record wins
    return recs


def main():
    markers = catalog()
    arg = sys.argv[1]
    if os.path.isdir(arg):
        arg = os.path.join(arg, "toml", "ledger.jsonl")
    recs = load(arg)
    by = lambda s: [m for m in markers if recs.get(m, {}).get("status") == s]
    stage1, unreach = by("STAGE1"), by("UNREACH")
    witness, none_, fail = by("WITNESS"), by("none"), by("FAIL")
    missing = [m for m in markers if m not in recs]
    covered = stage1 + witness

    vals = {}
    print(f"markers in catalog            : {len(markers)}")
    print(f"stage 1 sat                   : {len(stage1)}")
    print(f"stage 1 unsat (unreachable)   : {len(unreach)}")
    print(f"witness after stage 2         : {len(witness)}")
    print(f"COVERED (certcount)           : {len(covered)}")
    print(f"MISSES  (misscount)           : {len(none_)}")
    print(f"run failures                  : {len(fail)}")
    print(f"not attempted                 : {len(missing)}")
    # A coverage number alone cannot tell "found all it can" from "stopped early".
    stops = Counter(recs[m].get("stop", "?") for m in markers if m in recs)
    print("\nhow each marker's loop ended")
    for k, v in stops.most_common():
        print(f"  {k:<16} {v}")
    if none_:
        print(f"\n{len(none_)} markers have no witness. None of them is ruled out:")
        print("  a budget stop is a spending limit, and the proposer is stochastic,")
        print("  so another sample or a larger budget may still find one.")
        print(f"  -> COVERAGE IS A LOWER BOUND at RUSMT_ROUNDS attempts per marker")

    # What the counterexample step reported on each rejection: the evidence that
    # the extra solver call earns its place.
    named = valid = nothing = 0
    for m in markers:
        for o in (recs.get(m, {}).get("round_outcomes") or []):
            if not o.startswith("`unsat`"):
                continue
            if "reaches ERR" in o:
                named += 1
            elif " OK" in o or "simply valid" in o:
                valid += 1
            else:
                nothing += 1
    rej = named + valid + nothing
    if rej:
        print(f"\ncounterexample quality, over {rej} rejections")
        print(f"  {named:>4} ({100*named/rej:4.1f}%)  Z3 named a DIFFERENT marker")
        print(f"  {valid:>4} ({100*valid/rej:4.1f}%)  the document was simply valid")
        print(f"  {nothing:>4} ({100*nothing/rej:4.1f}%)  no observation (bare unsat)")
        vals.update(cexnamed=f"{100*named/rej:.1f}",
                    cexvalid=f"{100*valid/rej:.1f}",
                    cexnone=f"{100*nothing/rej:.1f}")

    # Cumulative coverage per round: the curve the round cap is set from.
    won = Counter()
    for m in markers:
        r = recs.get(m)
        if not r or not r.get("witnesses"):
            continue
        for i, o in enumerate(r.get("round_outcomes") or []):
            if o.startswith("ACCEPTED"):
                won[i + 1] += 1
                break
    # Coverage as a function of the budget, recovered from one run: a witness
    # accepted in round k would also have been accepted at any budget >= k.
    # This is what lets an arbitrary spending limit be reported honestly — the
    # reader sees whether the curve had flattened or was still climbing.
    print("\nsensitivity: what coverage WOULD have been at each budget")
    print("  (recovered from this single run; no extra solving)")

    print("\nwitnesses by round (the round-budget curve)")
    cum = 0
    rows, last = [], 0
    for k in range(1, max(won or [1]) + 1):
        cum += won.get(k, 0)
        print(f"  round {k}: +{won.get(k, 0):>3}  cumulative {cum}")
        rows.append(f"{k} & {won.get(k, 0)} & {cum} \\\\")
        if won.get(k):
            last = k
    vals["roundcurve"] = rows
    vals["lastwinround"] = last
    # How binding was the budget? If the last rounds still produced witnesses,
    # the curve had not flattened and coverage is climbing at the cap.
    tail = sum(won.get(k, 0) for k in range(max(1, last - 1), last + 1))
    print(f"\n  witnesses in the final two productive rounds: {tail}")
    if tail:
        print("  -> the curve had NOT flattened at the cap: a larger budget would")
        print("     very likely have covered more. Coverage is a lower bound.")
    else:
        print("  -> no witness arrived in the last rounds, so raising the cap")
        print("     would probably add little. Still not an exhaustion claim.")
    # How often the per-candidate Z3 budget was the binding constraint.
    und = vals.get("undecidedcount", 0)
    if und:
        print(f"\n  {und} acceptance checks hit RUSMT_Z3_SECS without a verdict:")
        print("     those candidates were neither accepted nor refuted, so that")
        print("     budget too can only cost coverage, never invent it.")

    # Where the misses fall, for the RQ3 prose.
    def bucket(m):
        for pre, name in [
            (("date_", "time_", "full_date", "partial_time", "numoffset", "datetime"), "date/time/offset"),
            (("integer_", "float_"), "numeric literal"),
            (("std_table", "array_table", "dotted_key", "inline_table", "toml_table"), "table/key redefinition"),
            (("ml_", "basic_string", "literal_string", "string_", "quoted_key"), "string"),
            (("array_",), "array"),
            (("boolean_",), "boolean"),
        ]:
            if m.startswith(pre):
                return name
        return "other"

    print(f"\nmisses by kind ({len(none_)} total)")
    kinds = Counter(bucket(m) for m in none_)
    for k, v in kinds.most_common():
        print(f"  {k:<24} {v}")
    vals.update(missdate=kinds["date/time/offset"], missnum=kinds["numeric literal"],
                missstring=kinds["string"], misstable=kinds["table/key redefinition"],
                missarray=kinds["array"],
                missother=kinds["other"] + kinds["boolean"])
    print("\nmisses by name:")
    for m in none_:
        print(f"  {m}")
    if fail:
        print("\nRUN FAILURES (must be zero before reporting):")
        for m in fail:
            print(f"  {m}: {recs[m].get('failure')}")

    if len(sys.argv) > 2:
        d = json.load(open(sys.argv[2], encoding="utf-8"))
        rows, dis = d["rows"], d["disagreements"]
        print(f"\nconformance diff: {len(rows)} inputs x {len(d['versions'])} impls")
        print(f"  divergences (diffcount)   : {len(dis)}")
        print(f"  markers involved          : {len({x[0] for x in dis})}")
        agreed = sum(
            1 for r in rows
            if all(i["verdict"] == r["oracle_verdict"] for i in r["impls"].values())
        )
        print(f"  inputs all four agreed on : {agreed}")
        for name in sorted(d["versions"]):
            n = sum(1 for x in dis if x[1] == name)
            print(f"    {name:<18} {len(rows) - n}/{len(rows)}  ({n} divergences)")
        print("\n  divergences:")
        for m, impl, orc, got in dis:
            print(f"    {m:<44} {impl:<16} {orc[:28]:<29} {got[:24]}")
        short = {"rust-toml": "rust", "py-tomllib": "py",
                 "go-burntsushi": "go", "node-smol-toml": "node"}
        vals.update(diffcount=len(dis), allagree=agreed,
                    divmarkers=len({x[0] for x in dis}))
        for name, tag in short.items():
            n = sum(1 for x in dis if x[1] == name)
            vals[f"agree{tag}"] = len(rows) - n
            vals[f"div{tag}"] = n
    latex_block(vals)


def latex_block(vals):
    """The macro block to paste into specops/main.tex, verbatim."""
    print("\n" + "=" * 62)
    print("PASTE INTO specops/main.tex (replacing the TODO values)")
    print("=" * 62)
    for k, v in vals.items():
        if k == "roundcurve":
            print("\\newcommand{\\roundcurve}{%")
            for line in v:
                print("  " + line)
            print("}")
        else:
            print(f"\\newcommand{{\\{k}}}{{{v}}}")
    print("=" * 62)
    print("Values the run cannot supply, fill by hand after triage:")
    print("  \\bugcount     divergences the standard settles against the parser")
    print("  \\strictcount  divergences where our reference is the strict one")
    print("  \\exampletime  seconds for the running example's accepted pin")
    print("               (read timing.txt for the running-example target directory)")


if __name__ == "__main__":
    main()
