"""Resumable per-marker sweep: Stage 1, then the AI⇄Z3 co-solving loop.

One `recover` invocation per marker, appending a JSON line per result. Already
recorded markers are skipped, so the sweep can be stopped and resumed as often
as needed without losing or repeating work — which matters because each round
costs a real model call.

usage: sweep.py <lang> <entry_fn> <results.jsonl> [--markers a,b,c] [--limit N]
                [--timeout SECS]
"""
import json
import os
import re
import subprocess
import sys
import threading
import time
from concurrent.futures import ThreadPoolExecutor, as_completed

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BIN = os.path.join(ROOT, "target/debug/rusmt-smt-derive")


def all_markers(lang):
    """Marker names, read from the catalog the oracle exposes."""
    src = os.path.join(ROOT, "lang/src/certify.rs")
    text = open(src, encoding="utf-8").read()
    const = "TOML_NAMED_MARKERS" if lang == "toml" else "IMP_MARKERS"
    m = re.search(const + r"[^=]*=\s*\[(.*?)\];", text, re.S)
    if not m:
        raise SystemExit(f"cannot find {const} in {src}")
    return re.findall(r'"([^"]+)"', m.group(1))


def done_markers(path):
    if not os.path.exists(path):
        return set()
    out = set()
    with open(path, encoding="utf-8") as f:
        for line in f:
            try:
                out.add(json.loads(line)["marker"])
            except Exception:
                pass
    return out


def parse_run(text):
    """Pull the structured result out of the `recover` CLI's output."""
    rec = {
        "stage1": None,
        "rounds": 0,
        "witnesses": [],
        "round_outcomes": [],
        "restored": [],
    }
    m = re.search(r"stage 1 verdict: (.*)", text)
    if m:
        rec["stage1"] = m.group(1).strip()
    rec["rounds"] = len(re.findall(r"^\[recover\] round \d+ \(", text, re.M))
    rec["round_ms"] = [int(x) for x in re.findall(r"^\[recover\] round \d+ \((\d+) ms\)", text, re.M)]
    for line in text.splitlines():
        s = line.strip()
        if s.startswith("[recover]   restored:"):
            rec["restored"].extend(
                x.strip() for x in s.split("restored:", 1)[1].split(",") if x.strip()
            )
        if s.startswith("[recover]   outcome:"):
            rec["round_outcomes"].append(s.split("outcome:", 1)[1].strip()[:220])
    # Witness lines are the quoted entries after the RESULT header.
    tail = text.split("RESULT:", 1)
    if len(tail) == 2:
        rec["witnesses"] = [
            w for w in re.findall(r'^\[recover\]   ("(?:[^"\\]|\\.)*")\s*$', tail[1], re.M)
        ]
    rec["stage1_solved"] = bool(rec["stage1"] and rec["stage1"].startswith("sat"))
    return rec


def main():
    lang, entry, out = sys.argv[1], sys.argv[2], sys.argv[3]
    argv = sys.argv[4:]
    timeout = 900
    if "--timeout" in argv:
        timeout = int(argv[argv.index("--timeout") + 1])
    if "--markers" in argv:
        markers = argv[argv.index("--markers") + 1].split(",")
    else:
        markers = all_markers(lang)
    limit = int(argv[argv.index("--limit") + 1]) if "--limit" in argv else None

    already = done_markers(out)
    todo = [m for m in markers if m not in already]
    if limit:
        todo = todo[:limit]
    print(f"# {len(markers)} markers, {len(already)} already recorded, running {len(todo)}",
          flush=True)

    jobs = int(argv[argv.index("--jobs") + 1]) if "--jobs" in argv else 1
    lock = threading.Lock()
    done = [0]

    def one(marker):
        t0 = time.time()
        try:
            p = subprocess.run(
                [BIN, "recover", lang, entry, marker],
                capture_output=True, text=True, timeout=timeout,
            )
            rec = parse_run(p.stdout + p.stderr)
            rec["exit"] = p.returncode
        except subprocess.TimeoutExpired as e:
            # On a kill these come back as bytes even under text=True.
            def _txt(x):
                if x is None:
                    return ""
                return x.decode("utf-8", "replace") if isinstance(x, bytes) else x
            rec = parse_run(_txt(e.stdout) + _txt(e.stderr))
            rec["exit"] = "timeout"
        rec["marker"] = marker
        rec["wall_s"] = round(time.time() - t0, 1)
        status = (
            "STAGE1" if rec["stage1_solved"]
            else ("WITNESS" if rec["witnesses"] else "none")
        )
        # One writer at a time: a partial line would corrupt the resume set.
        with lock:
            done[0] += 1
            with open(out, "a", encoding="utf-8") as f:
                f.write(json.dumps(rec) + "\n")
            print(f"[{done[0]}/{len(todo)}] {marker:<42} {status:<8} "
                  f"rounds={rec['rounds']} {rec['wall_s']}s "
                  f"{rec['witnesses'][0] if rec['witnesses'] else ''}", flush=True)
        return rec

    if jobs <= 1:
        for m in todo:
            one(m)
    else:
        with ThreadPoolExecutor(max_workers=jobs) as ex:
            futs = [ex.submit(one, m) for m in todo]
            for f in as_completed(futs):
                try:
                    f.result()
                except Exception as exc:      # keep the sweep alive
                    print(f"# worker error: {exc}", flush=True)


if __name__ == "__main__":
    main()
