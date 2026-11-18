#!/usr/bin/env python3
"""Keep the launcher's language tables in shape for the offline build.

Upstream re-translates launcher/lang/*.json constantly, so the patches touch Rust
only. Right after git am the release workflow runs this script: it applies the
key operations in lang-overrides.json (drop every "delete" key, add every "add"
key with the per-language text, English fallback for languages the data does not
know). The launcher's own tables are sparse by design: a key a table lacks shows
in English at runtime, and its i18n test counts missing/stale entries as
coverage information instead of failing. So keys the patched code asks for that
no table knows are reported, not written — dummy English entries in every table
would only bury them and block real translations later. Idempotent;
round-trips upstream's layout.

Usage: python3 offline/lang-sync.py [--lang-dir launcher/lang] [--overrides FILE]
                                    [--no-auto-add]
"""

import argparse
import json
import re
import sys
from pathlib import Path


# The tr()/trf() key scan, mirroring launcher/src/i18n/tests.rs: token-level,
# macro-recursing, first literal argument only.

def _unescape(s: str) -> str:
    out = []
    i, n = 0, len(s)
    while i < n:
        c = s[i]
        if c == "\\" and i + 1 < n:
            d = s[i + 1]
            # Rust string continuation: backslash + newline strips the newline
            # and all leading whitespace of the next line.
            if d == "\n" or (d == "\r" and i + 2 < n and s[i + 2] == "\n"):
                i += 2 if d == "\n" else 3
                while i < n and s[i] in " \t\n":
                    i += 1
                continue
            simple = {"n": "\n", "r": "\r", "t": "\t", "\\": "\\",
                      '"': '"', "'": "'", "0": "\0"}
            if d in simple:
                out.append(simple[d]); i += 2; continue
            if d == "x" and i + 3 < n:
                try:
                    out.append(chr(int(s[i + 2:i + 4], 16))); i += 4; continue
                except ValueError:
                    pass
            if d == "u" and i + 2 < n and s[i + 2] == "{":
                j = s.find("}", i + 3)
                if j > 0:
                    try:
                        out.append(chr(int(s[i + 3:j], 16))); i = j + 1; continue
                    except ValueError:
                        pass
            out.append(c); i += 1
        else:
            out.append(c); i += 1
    return "".join(out)


def _tokenize(text: str):
    toks = []
    i, n = 0, len(text)
    while i < n:
        c = text[i]
        if text[i:i + 2] == "//":
            j = text.find("\n", i)
            i = (j + 1) if j >= 0 else n
            continue
        if text[i:i + 2] == "/*":
            depth, i = 1, i + 2
            while i < n and depth > 0:
                if text[i:i + 2] == "/*":
                    depth += 1; i += 2
                elif text[i:i + 2] == "*/":
                    depth -= 1; i += 2
                else:
                    i += 1
            continue
        # raw strings: r"…" r#"…"# and byte variants br"…" br#"…"#
        off = 0
        if c == "r":
            off = 1
        elif c == "b" and text[i:i + 2] == "br":
            off = 2
        if off:
            m = re.match(r'(#*)"', text[i + off:])
            if m:
                h = m.group(1)
                start = i + off + len(h) + 1
                end = text.find('"' + h, start)
                content = text[start:end] if end >= 0 else text[start:]
                toks.append(("bytestr" if off == 2 else "rawstr", content))
                i = (end + 1 + len(h)) if end >= 0 else n
                continue
        if c == "b" and i + 1 < n and text[i + 1] == '"':
            j = i + 2
            buf = []
            while j < n:
                if text[j] == "\\":
                    buf.append(text[j:j + 2]); j += 2; continue
                if text[j] == '"':
                    break
                buf.append(text[j]); j += 1
            toks.append(("bytestr", "".join(buf))); i = j + 1; continue
        if c == "b" and i + 1 < n and text[i + 1] == "'":
            j = i + 2
            if j < n and text[j] == "\\":
                j += 2
            else:
                j += 1
            if j < n and text[j] == "'":
                j += 1
            toks.append(("char", "")); i = j; continue
        if c == '"':
            j = i + 1
            buf = []
            while j < n:
                if text[j] == "\\":
                    buf.append(text[j:j + 2]); j += 2; continue
                if text[j] == '"':
                    break
                buf.append(text[j]); j += 1
            toks.append(("str", _unescape("".join(buf)))); i = j + 1; continue
        if c == "'":
            if i + 1 < n and text[i + 1] == "\\":
                j = i + 2
                if j < n and text[j] == "u" and j + 1 < n and text[j + 1] == "{":
                    k = text.find("}", j)
                    j = (k + 1) if k >= 0 else n
                else:
                    j += 2
                if j < n and text[j] == "'":
                    j += 1
                toks.append(("char", "")); i = j; continue
            if i + 2 < n and text[i + 2] == "'" and text[i + 1] != "\n":
                toks.append(("char", "")); i = i + 3; continue
            j = i + 1
            while j < n and (text[j].isalnum() or text[j] == "_"):
                j += 1
            toks.append(("lifetime", "")); i = j; continue
        if c.isalpha() or c == "_":
            j = i
            while j < n and (text[j].isalnum() or text[j] == "_"):
                j += 1
            toks.append(("ident", text[i:j])); i = j; continue
        if not c.isspace():
            toks.append(("punct", c))
        i += 1
    return toks


def _first_arg_string(toks, open_idx):
    """The first token inside the argument group, when it is a string literal."""
    depth = 0
    j = open_idx + 1
    while j < len(toks):
        k, v = toks[j]
        if k == "punct" and v == "(":
            depth += 1
        elif k == "punct" and v == ")":
            if depth == 0:
                return None
            depth -= 1
        else:
            if k in ("str", "rawstr"):
                return v
            return None
        j += 1
    return None


def asked_keys(src_dir: Path) -> set:
    """Every English string the launcher's code asks for via tr()/trf()."""
    asked = set()
    for path in sorted(src_dir.rglob("*.rs")):
        toks = _tokenize(path.read_text(encoding="utf-8"))
        for idx, (k, v) in enumerate(toks):
            if k == "ident" and v in ("tr", "trf"):
                if idx + 1 < len(toks) and toks[idx + 1] == ("punct", "("):
                    val = _first_arg_string(toks, idx + 1)
                    if val:
                        asked.add(val)
    return asked


def main() -> int:
    here = Path(__file__).resolve().parent
    parser = argparse.ArgumentParser(
        description="Apply the offline build's language-table key operations.")
    parser.add_argument("--lang-dir", default="launcher/lang",
                        help="language table directory, relative to the repo root")
    parser.add_argument("--overrides", default=str(here / "lang-overrides.json"),
                        help="the add/delete key data (default: beside this script)")
    parser.add_argument("--root", default=str(here.parent),
                        help="the repo root (default: the parent of offline/)")
    parser.add_argument("--no-auto-add", action="store_true",
                        help="do not scan and report keys no table knows yet")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    lang_dir = root / args.lang_dir
    if not lang_dir.is_dir():
        print(f"lang-sync: no language directory at {lang_dir}", file=sys.stderr)
        return 1
    tables = sorted(lang_dir.glob("*.json"))
    if not tables:
        print(f"lang-sync: no language tables inside {lang_dir}", file=sys.stderr)
        return 1
    data = json.loads(Path(args.overrides).read_text(encoding="utf-8"))
    adds: dict = data.get("add", {})
    deletes: list = data.get("delete", [])

    loaded = {path: json.loads(path.read_text(encoding="utf-8"))
              for path in tables}
    touched = 0
    for path, table in loaded.items():
        dropped = [key for key in deletes if key in table]
        added = 0
        for key, translations in adds.items():
            if key in table:
                continue
            table[key] = translations.get(path.stem, translations.get("en", key))
            added += 1
        for key in dropped:
            del table[key]
        if dropped or added:
            path.write_text(
                json.dumps(table, ensure_ascii=False, indent=2) + "\n",
                encoding="utf-8")
            touched += 1
            print(f"lang-sync: {path.name}: -{len(dropped)} +{added}")

    # Sparse tables: report keys the patched code asks for that no table knows.
    # They show in English at runtime (the launcher's own fallback), so writing
    # them into every table is not needed and would only add dummy entries.
    if not args.no_auto_add:
        src_dir = root / "launcher" / "src"
        if src_dir.is_dir():
            wanted = asked_keys(src_dir)
            present = set().union(*(set(t) for t in loaded.values()))
            unknown = sorted(k for k in wanted if k and k not in present)
            for key in unknown[:12]:
                print(f"lang-sync: no table knows {key!r} yet")
            if len(unknown) > 12:
                print(f"lang-sync: ... and {len(unknown) - 12} more")
            if unknown:
                print(f"lang-sync: {len(unknown)} code key(s) are in no table; "
                      "the launcher shows them in English until they are "
                      "translated (offline/lang-overrides.json)")

    print(f"lang-sync: {touched} table(s) updated; sparse tables are fine, "
          "missing keys fall back to English at runtime")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
