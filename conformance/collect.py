"""Collect the synthesized witnesses into a suite directory of `.toml` files.

Reads the pipeline's output tree (`lang/src/synthesis/toml/z3_chc/target_*/`),
takes each target's accepted witness, and writes it as `<marker>.toml`. Extra
witnesses for a marker become `<marker>__2.toml`, `__3.toml`, and so on, so the
diff harness exercises all of them rather than only the first — one witness per
marker can easily be an input every implementation already handles.

usage: collect.py <synthesis_dir|results.jsonl> <suite_out_dir>

A `.jsonl` first argument is a sweep results file (`experiments/results/*.jsonl`)
and is the preferred source: `lang/src/synthesis/` is gitignored, so the suite has
to be rebuildable from tracked data for the results to reproduce from the artifact.
"""
import os
import re
import sys


def decode_witness_literal(text):
    """Decode the Rust debug string form printed by the recover CLI."""
    import ast
    try:
        return ast.literal_eval(text)
    except (ValueError, SyntaxError):
        pass
    if len(text) < 2 or text[0] != '"' or text[-1] != '"':
        raise ValueError("not a quoted witness")
    s = text[1:-1]
    out = []
    i = 0
    while i < len(s):
        c = s[i]
        if c != "\\":
            out.append(c)
            i += 1
            continue
        i += 1
        if i >= len(s):
            raise ValueError("trailing escape")
        e = s[i]
        i += 1
        if e == "n":
            out.append("\n")
        elif e == "r":
            out.append("\r")
        elif e == "t":
            out.append("\t")
        elif e in ('"', "\\"):
            out.append(e)
        elif e == "0":
            out.append("\0")
        elif e == "u" and i < len(s) and s[i] == "{":
            j = s.find("}", i + 1)
            if j == -1:
                raise ValueError("unterminated unicode escape")
            out.append(chr(int(s[i + 1:j], 16)))
            i = j + 1
        else:
            out.append(e)
    return "".join(out)


def marker_of(target_dir):
    """The marker a target directory belongs to, from its cosolve transcript."""
    for name in ("cosolve.txt", "replay.txt"):
        p = os.path.join(target_dir, name)
        if os.path.exists(p):
            head = open(p, encoding="utf-8", errors="replace").read(4000)
            m = re.search(r"^marker\s*:\s*(\S+)", head, re.M)
            if m:
                return m.group(1)
            m = re.search(r"reaches `([A-Za-z0-9_]+)`", head)
            if m:
                return m.group(1)
    return None


def witnesses(target_dir):
    """Accepted witnesses for a target: response.toml plus witnesses.txt."""
    out = []
    p = os.path.join(target_dir, "response.toml")
    if os.path.exists(p):
        out.append(open(p, encoding="utf-8", errors="replace").read())
    extra = os.path.join(target_dir, "witnesses.txt")
    if os.path.exists(extra):
        body = open(extra, encoding="utf-8", errors="replace").read()
        out.extend(w for w in body.split("\n---\n") if w)
    return out


def from_jsonl(path, dst):
    """Build the suite from a sweep results file — the tracked source of truth."""
    import json
    n = 0
    seen = set()
    for line in open(path, encoding="utf-8"):
        try:
            r = json.loads(line)
        except ValueError:
            continue
        if r["marker"] in seen:
            continue
        seen.add(r["marker"])
        for i, w in enumerate(r.get("witnesses") or []):
            # Witnesses are recorded as Rust debug strings; decode that format.
            try:
                text = decode_witness_literal(w)
            except (ValueError, SyntaxError):
                continue
            suffix = "" if i == 0 else f"__{i + 1}"
            with open(os.path.join(dst, f"{r['marker']}{suffix}.toml"), "w",
                      encoding="utf-8") as f:
                f.write(text)
            n += 1
    print(f"{n} inputs written to {dst} from {path}")


def main():
    src, dst = sys.argv[1], sys.argv[2]
    if src.endswith(".jsonl"):
        os.makedirs(dst, exist_ok=True)
        return from_jsonl(src, dst)
    os.makedirs(dst, exist_ok=True)
    n_targets = n_written = 0
    unnamed = []
    for entry in sorted(os.listdir(src)):
        d = os.path.join(src, entry)
        if not (entry.startswith("target_") and os.path.isdir(d)):
            continue
        n_targets += 1
        ws = witnesses(d)
        if not ws:
            continue
        marker = marker_of(d) or entry
        if marker == entry:
            unnamed.append(entry)
        for i, w in enumerate(ws):
            suffix = "" if i == 0 else f"__{i + 1}"
            with open(os.path.join(dst, f"{marker}{suffix}.toml"), "w",
                      encoding="utf-8") as f:
                f.write(w)
            n_written += 1
    print(f"{n_targets} targets scanned; {n_written} inputs written to {dst}")
    if unnamed:
        print(f"warning: {len(unnamed)} targets had no marker name in their "
              f"transcript: {', '.join(unnamed[:8])}")


if __name__ == "__main__":
    main()
