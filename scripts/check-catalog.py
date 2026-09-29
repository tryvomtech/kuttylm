#!/usr/bin/env python3
"""Checks catalog/models.json (the "Picked for you" list) before it goes live.

  python3 scripts/check-catalog.py            # format + Hugging Face checks
  python3 scripts/check-catalog.py --offline  # format checks only

Uses only the Python standard library. Exit code 1 when a problem is found.
"""

import json
import re
import sys
import urllib.error
import urllib.request
from pathlib import Path

LIST = Path(__file__).resolve().parent.parent / "catalog" / "models.json"
HF = "https://huggingface.co"
SAFE = re.compile(r"^[A-Za-z0-9_-][A-Za-z0-9._-]{0,199}$")
SHA256 = re.compile(r"^[0-9a-fA-F]{64}$")

problems: list[str] = []
warnings: list[str] = []


def safe_name(s) -> bool:
    return isinstance(s, str) and bool(SAFE.match(s)) and ".." not in s


def check_file(model_id: str, f, what: str) -> None:
    if not isinstance(f, dict):
        problems.append(f"{model_id}: {what} is not an object")
        return
    name = f.get("name")
    if not safe_name(name) or not name.lower().endswith(".gguf"):
        problems.append(f"{model_id}: {what} has a bad name {name!r} (must end in .gguf, no '/')")
    size = f.get("size_bytes")
    if not isinstance(size, int) or size <= 0:
        problems.append(f"{model_id}: {what} {name} needs a size_bytes above 0")
    sha = f.get("sha256")
    if sha is not None and not (isinstance(sha, str) and SHA256.match(sha)):
        problems.append(f"{model_id}: {what} {name} has a bad sha256")


def check_format(data) -> list[dict]:
    if not isinstance(data, dict) or data.get("version") != 1:
        problems.append('the top level must be an object with "version": 1')
        return []
    models = data.get("models")
    if not isinstance(models, list) or not models:
        problems.append('"models" must be a list with at least one model')
        return []
    seen = set()
    for m in models:
        mid = m.get("id") if isinstance(m, dict) else None
        if not safe_name(mid):
            problems.append(f"bad or missing id {mid!r} (letters, digits, '.', '_', '-')")
            continue
        if mid in seen:
            problems.append(f"{mid}: the id is used twice")
        seen.add(mid)
        if not isinstance(m.get("name"), str) or not m["name"].strip():
            problems.append(f"{mid}: missing name")
        repo = m.get("hf_repo", "")
        owner, _, rest = repo.partition("/") if isinstance(repo, str) else ("", "", "")
        if not (safe_name(owner) and safe_name(rest)):
            problems.append(f"{mid}: hf_repo must be owner/name, got {repo!r}")
        revision = m.get("revision", "main")
        if not safe_name(revision):
            problems.append(f"{mid}: bad revision {revision!r}")
        elif revision == "main":
            warnings.append(f"{mid}: revision is 'main'; a commit id is safer (the files cannot change)")
        files = m.get("files")
        if not isinstance(files, list) or not files:
            problems.append(f"{mid}: needs at least one file")
        else:
            for f in files:
                check_file(mid, f, "file")
        if m.get("mmproj") is not None:
            check_file(mid, m["mmproj"], "mmproj")
    return [m for m in models if isinstance(m, dict)]


def get_json(url: str):
    req = urllib.request.Request(url, headers={"User-Agent": "KuttyLM-catalog-check"})
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.load(r)


def check_online(model: dict) -> None:
    mid, repo = model["id"], model["hf_repo"]
    revision = model.get("revision", "main")
    try:
        info = get_json(f"{HF}/api/models/{repo}?expand[]=gated&expand[]=private")
    except urllib.error.HTTPError as e:
        problems.append(f"{mid}: {repo} cannot be opened (HTTP {e.code}); it may be gated or deleted")
        return
    if info.get("gated") not in (False, None) or info.get("private"):
        problems.append(f"{mid}: {repo} needs a login (gated); KuttyLM cannot download it")
        return
    try:
        tree = get_json(f"{HF}/api/models/{repo}/tree/{revision}")
    except urllib.error.HTTPError as e:
        problems.append(f"{mid}: revision {revision} of {repo} was not found (HTTP {e.code})")
        return
    online = {e["path"]: e for e in tree if e.get("type") == "file"}
    files = list(model["files"]) + ([model["mmproj"]] if model.get("mmproj") else [])
    for f in files:
        entry = online.get(f["name"])
        if entry is None:
            problems.append(f"{mid}: {f['name']} does not exist in {repo} at {revision}")
            continue
        if entry.get("size") != f["size_bytes"]:
            problems.append(f"{mid}: {f['name']} size is {entry.get('size')} online, {f['size_bytes']} in the list")
        oid = (entry.get("lfs") or {}).get("oid")
        if f.get("sha256") and oid and oid.lower() != f["sha256"].lower():
            problems.append(f"{mid}: {f['name']} sha256 does not match the file online")
    print(f"  ok  {mid}")


def main() -> int:
    try:
        data = json.loads(LIST.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as e:
        print(f"ERROR: cannot read {LIST}: {e}")
        return 1
    models = check_format(data)
    if not problems and "--offline" not in sys.argv:
        print(f"Checking {len(models)} models on Hugging Face…")
        for m in models:
            check_online(m)
    for w in warnings:
        print(f"WARNING: {w}")
    for p in problems:
        print(f"ERROR: {p}")
    if problems:
        print(f"\n{len(problems)} problem(s). Fix them before the list goes live.")
        return 1
    print(f"\nThe list is OK ({len(models)} models).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
