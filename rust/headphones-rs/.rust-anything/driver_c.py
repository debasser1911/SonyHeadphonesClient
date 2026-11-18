
import ctypes
import json
import locale
import math
import os
import signal
import sys
import time

# Python switches C's locale to the user's at startup; a C program stays in "C" unless it calls
# setlocale, so the library must see "C" here (isspace, toupper, strtod depend on it).
locale.setlocale(locale.LC_ALL, "C")


def enc(v):
    if isinstance(v, float):
        if math.isnan(v):
            return {"$num": "NaN"}
        if math.isinf(v):
            return {"$num": "Infinity" if v > 0 else "-Infinity"}
        if v == 0 and math.copysign(1, v) < 0:
            return {"$num": "-0"}
    if isinstance(v, bytes):
        return {"$bytes": list(v)}
    return v


def dec(v):
    if isinstance(v, dict) and len(v) == 1:
        (key, inner), = v.items()
        if key == "$undefined":
            return None
        if key == "$num":
            return {"NaN": math.nan, "Infinity": math.inf, "-Infinity": -math.inf, "-0": -0.0}[inner]
    return v


def bind(path, signatures):
    lib = ctypes.CDLL(path)
    functions = {}
    for name, sig in signatures.items():
        try:
            fn = getattr(lib, name)
        except AttributeError:
            continue
        fn.argtypes = [getattr(ctypes, t) for t in sig["args"]]
        fn.restype = None if sig["ret"] == "None" else getattr(ctypes, sig["ret"])
        functions[name] = (fn, sig)
    return functions


def call(functions, c):
    fn, sig = functions[c["fn"]]
    args = []
    for value, kind in zip([dec(a) for a in c["args"]], sig["args"]):
        if kind == "c_char_p":
            args.append(None if value is None else str(value).encode("utf-8"))
        elif kind in ("c_float", "c_double"):
            args.append(float(value) if isinstance(value, (int, float)) else 0.0)
        elif kind == "c_bool":
            args.append(bool(value))
        else:
            args.append(int(value) if isinstance(value, (int, float)) and math.isfinite(value) else 0)
    while len(args) < len(sig["args"]):
        args.append(None if sig["args"][len(args)] == "c_char_p" else 0)
    return {"ok": enc(fn(*args))}


def isolated(functions, c):
    """Runs one case in a forked child: undefined behavior in C can crash the process."""
    read_end, write_end = os.pipe()
    pid = os.fork()
    if pid == 0:
        os.close(read_end)
        try:
            result = call(functions, c)
        except Exception as e:  # ctypes conversion errors
            result = {"error": "%s: %s" % (type(e).__name__, e)}
        os.write(write_end, json.dumps(result).encode())
        os._exit(0)
    os.close(write_end)
    data = b""
    while True:
        chunk = os.read(read_end, 65536)
        if not chunk:
            break
        data += chunk
    os.close(read_end)
    _, status = os.waitpid(pid, 0)
    if os.WIFSIGNALED(status):
        return {"error": "crash: " + signal.Signals(os.WTERMSIG(status)).name}
    return json.loads(data) if data else {"error": "crash: no result"}


def cases_of(path):
    return [json.loads(line) for line in open(path, encoding="utf-8") if line.strip()]


def record(lib, sigs, inputs, out_path):
    functions = bind(lib, json.load(open(sigs)))
    progress = os.environ.get("RUST_ANYTHING_PROGRESS")
    with open(out_path, "w", encoding="utf-8") as out:
        for index, c in enumerate(json.load(open(inputs, encoding="utf-8"))):
            if progress:
                with open(progress, "a", encoding="utf-8") as p:
                    p.write("%d\n" % index)
            line = {"fn": c["fn"], "args": c["args"]}
            line.update(isolated(functions, c))
            out.write(json.dumps(line, ensure_ascii=False) + "\n")
            out.flush()


def diff(lib, sigs, cases_path, out_path):
    functions = bind(lib, json.load(open(sigs)))
    summary = {"total": 0, "passed": 0, "failed": 0, "missing": [], "mismatches": []}
    for c in cases_of(cases_path):
        summary["total"] += 1
        if c["fn"] not in functions:
            if c["fn"] not in summary["missing"]:
                summary["missing"].append(c["fn"])
            summary["failed"] += 1
            continue
        if str(c.get("error", "")).startswith("crash:"):
            # The original's behavior is undefined there: any answer of the port passes.
            summary["passed"] += 1
            continue
        got = isolated(functions, c)
        expected = {"ok": c.get("ok")}
        if json.dumps(got) == json.dumps(expected):
            summary["passed"] += 1
        else:
            summary["failed"] += 1
            summary["mismatches"].append({"fn": c["fn"], "args": c["args"], "expected": expected, "actual": got})
    json.dump(summary, open(out_path, "w", encoding="utf-8"), ensure_ascii=False)


def bench(original, rust, sigs, cases_path, out_path, min_ms):
    signatures = json.load(open(sigs))
    old, new = bind(original, signatures), bind(rust, signatures)
    groups = {}
    for c in cases_of(cases_path):
        if "error" not in c:
            groups.setdefault(c["fn"], []).append(c)
    rows = []
    for name, cases in groups.items():
        if name not in old or name not in new:
            continue

        def timed(functions):
            def once():
                for c in cases:
                    call(functions, c)
            once()
            passes, start = 0, time.perf_counter()
            while True:
                once()
                passes += 1
                elapsed = time.perf_counter() - start
                if elapsed * 1000 >= min_ms:
                    return elapsed * 1e9 / (passes * len(cases))
        original_ns, rust_ns = timed(old), timed(new)
        rows.append({"fn": name, "cases": len(cases), "original_ns": original_ns, "rust_ns": rust_ns, "speedup": original_ns / rust_ns})
    json.dump(rows, open(out_path, "w"))


if __name__ == "__main__":
    mode, *rest = sys.argv[1:]
    if mode == "record":
        record(*rest)
    elif mode == "diff":
        diff(*rest)
    elif mode == "bench":
        bench(rest[0], rest[1], rest[2], rest[3], rest[4], int(rest[5]))
