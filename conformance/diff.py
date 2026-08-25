"""Differential conformance harness for a synthesized TOML suite.

Runs every input in the suite through the RuSmt oracle and each independent
implementation, then reports where an implementation disagrees with the oracle
about whether the document is *accepted*.

Only accept/reject is compared. Error *messages* are recorded for triage but not
diffed: implementations word them differently and no spec requires otherwise, so
a message mismatch is not a conformance bug. An accept/reject mismatch is a
candidate one, and each has to be triaged against the TOML spec by hand — this
script flags, it does not adjudicate.

usage: diff.py <suite_dir> [--json out.json]
       suite_dir holds <marker>.toml files produced by the framework's
       --suite-out option.
"""
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
R = os.path.join(HERE, "runners")

# name -> (argv prefix, version probe)
IMPLS = {
    "rust-toml": (
        [os.path.join(R, "rust_toml/target/debug/rust_toml_runner")],
        ["sh", "-c", "grep -m1 -A1 'name = \"toml\"' " + os.path.join(R, "rust_toml/Cargo.lock") + " | tail -1"],
    ),
    "py-tomllib": (
        [sys.executable, os.path.join(R, "py_tomllib.py")],
        [sys.executable, "-c", "import sys;print('CPython '+sys.version.split()[0])"],
    ),
    "go-burntsushi": (
        [os.path.join(R, "go_bs/go_bs_runner")],
        ["sh", "-c", "grep -m1 BurntSushi " + os.path.join(R, "go_bs/go.mod")],
    ),
    "node-smol-toml": (
        ["node", os.path.join(R, "node_smol/run.mjs")],
        ["sh", "-c", "node -e \"console.log(require('" + os.path.join(R, "node_smol/node_modules/smol-toml/package.json") + "').version)\""],
    ),
}


def run(argv, timeout=20):
    """First stdout line, or a marker for crash/timeout."""
    try:
        p = subprocess.run(argv, capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired:
        return "TIMEOUT"
    out = p.stdout.strip().splitlines()
    if not out:
        return f"CRASH(rc={p.returncode})"
    return out[0].strip()


def verdict(line):
    """Collapse a runner line to accept / reject / neither."""
    if line == "OK":
        return "accept"
    if line.startswith("ERR") or line == "NOMATCH":
        return "reject"
    return "other"


ORACLE = os.path.join(ROOT, "target/debug/rusmt-lang")


def oracle(path):
    """The RuSmt reference parser's canonical observable behaviour: `OK`,
    `ERR <marker>`, `NOMATCH`, or `TIMEOUT`."""
    return run([ORACLE, "observe", "toml", path], timeout=60)


def versions():
    v = {}
    for name, (_, probe) in IMPLS.items():
        try:
            v[name] = run(probe, timeout=30)
        except Exception as e:  # a missing runner is reported, not fatal
            v[name] = f"unavailable: {e}"
    return v


def main():
    suite = sys.argv[1]
    out_json = None
    if "--json" in sys.argv:
        out_json = sys.argv[sys.argv.index("--json") + 1]

    inputs = sorted(f for f in os.listdir(suite) if f.endswith(".toml"))
    if not inputs:
        print(f"no .toml inputs in {suite}", file=sys.stderr)
        return 1

    available = {
        n: argv for n, (argv, _) in IMPLS.items()
        if os.path.exists(argv[-1]) or n == "py-tomllib"
    }
    missing = set(IMPLS) - set(available)
    if missing:
        print(f"# skipping unavailable implementations: {', '.join(sorted(missing))}")

    vers = versions()
    print("# implementation versions")
    for n in sorted(available):
        print(f"#   {n}: {vers.get(n, '?')}")

    rows, disagreements = [], []
    for fname in inputs:
        path = os.path.join(suite, fname)
        marker = fname[: -len(".toml")]
        orc_line = oracle(path)
        orc = verdict(orc_line)
        row = {"marker": marker, "input": open(path, "rb").read().decode("utf-8", "replace"),
               "oracle": orc_line, "oracle_verdict": orc, "impls": {}}
        for name, argv in available.items():
            line = run(argv + [path])
            row["impls"][name] = {"raw": line, "verdict": verdict(line)}
            if verdict(line) != orc and orc in ("accept", "reject"):
                disagreements.append((marker, name, orc_line, line))
        rows.append(row)

    print(f"\n# {len(rows)} inputs x {len(available)} implementations")
    print(f"# {len(disagreements)} accept/reject disagreements with the oracle\n")
    if disagreements:
        print(f"{'marker':<40} {'implementation':<16} {'oracle':<26} implementation")
        for m, n, o, i in disagreements:
            print(f"{m:<40} {n:<16} {o[:25]:<26} {i[:40]}")

    # Two different findings, and the paper must not conflate them.
    #
    #  * an implementation disagreeing with the ORACLE is a conformance result:
    #    the executable specification says the document violates a named rule and
    #    the implementation does not, so it is a candidate bug against the spec;
    #  * implementations disagreeing with EACH OTHER is a differential result: it
    #    shows the generated suite discriminates between real parsers, and holds
    #    whether or not our formalisation is right.
    cross = []
    for r in rows:
        verdicts = {n: i["verdict"] for n, i in r["impls"].items()}
        if len({v for v in verdicts.values() if v in ("accept", "reject")}) > 1:
            cross.append((r["marker"], verdicts))
    print(f"\n# {len(cross)} inputs on which the independent implementations "
          f"disagree with EACH OTHER")
    print("#   (a differential result: the suite discriminates between parsers,")
    print("#    independently of whether our reference is right)")
    for marker, v in cross:
        summary = ", ".join(f"{n}={x}" for n, x in sorted(v.items()))
        print(f"    {marker:<44} {summary}")

    # Per-implementation agreement summary.
    print("\n# agreement with the RuSmt oracle (accept/reject only)")
    for name in sorted(available):
        agree = sum(
            1 for r in rows
            if r["impls"][name]["verdict"] == r["oracle_verdict"]
            and r["oracle_verdict"] in ("accept", "reject")
        )
        total = sum(1 for r in rows if r["oracle_verdict"] in ("accept", "reject"))
        print(f"#   {name:<16} {agree}/{total}")

    if out_json:
        json.dump({"versions": vers, "rows": rows,
                   "disagreements": disagreements,
                   "cross_implementation": [
                       {"marker": m, "verdicts": v} for m, v in cross
                   ]}, open(out_json, "w"), indent=1)
        print(f"\n# wrote {out_json}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
