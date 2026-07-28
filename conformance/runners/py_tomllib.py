"""Runner: CPython's stdlib tomllib. Prints `OK` or `ERR <class>`."""
import sys, tomllib
try:
    with open(sys.argv[1], "rb") as f:
        tomllib.load(f)
    print("OK")
except tomllib.TOMLDecodeError as e:
    # tomllib gives prose, not a rule id; keep the first clause as the class.
    print("ERR " + str(e).split("(")[0].strip().replace("\n", " ")[:60])
except Exception as e:
    print(f"ERR {type(e).__name__}")
