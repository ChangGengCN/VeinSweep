"""重新打包 VeinSweep 的 .hnmod，并按 Horizon 官方读取器
(horizon_mods::index::native::Bundle) 的规则做结构校验。

用法：python publish/pack.py
改动源码或 README 之后运行它，然后把打印出的 sha256 更新到
publish/registry/mods/veinsweep.toml 和 publish/SUBMIT.md。
"""

import hashlib
import json
import pathlib
import sys
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
OUT = ROOT / "publish" / "veinsweep-0.2.1.hnmod"

MOD_ID = "veinsweep"
VERSION = "0.2.1"
GAME = "26.3"
# 开发与实测所用的 Horizon 提交（Nightly 2026-11-13）。
HORIZON = "9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0"
# horizon-mixins 的提交：launcher 的 native_mods::MIXINS_REV。
MIXINS = "5a64af938ec86b00d53d16a408aabc0038fb3e57"

FILES = [
    "README.md",
    "chain.rs",
    "click.rs",
    "horizon-mixin.toml",
    "list.rs",
    "page.rs",
    "veinsweep.rs",
]

manifest = {
    "schema": 1,
    "id": MOD_ID,
    "version": VERSION,
    "game": GAME,
    "horizon": HORIZON,
    "mixins": MIXINS,
}


def allowed(rest: str) -> bool:
    parts = rest.split("/")
    if rest.endswith(".rs") or rest.endswith(".toml"):
        return True
    in_data = len(parts) >= 2 and (
        parts[0] == "data" or (len(parts) >= 3 and parts[1] == "data")
    )
    name = parts[-1]
    text = len(parts) <= 2 and (
        any(name.startswith(p) for p in ("README", "LICENSE", "LICENCE", "COPYING", "NOTICE"))
        or name == ".gitignore"
    )
    return in_data or text


def main() -> int:
    OUT.parent.mkdir(parents=True, exist_ok=True)
    entries = [("horizon-native.json", json.dumps(manifest, indent=2).encode() + b"\n")]
    for name in sorted(FILES):
        entries.append((f"mixins/{MOD_ID}/{name}", (ROOT / name).read_bytes()))

    with zipfile.ZipFile(OUT, "w", zipfile.ZIP_DEFLATED) as zf:
        for name, data in entries:
            info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 3
            info.external_attr = (0o100644 & 0xFFFF) << 16
            zf.writestr(info, data)

    data = OUT.read_bytes()
    problems = []
    names = set()
    total = 0
    with zipfile.ZipFile(OUT) as zf:
        for info in zf.infolist():
            if info.is_dir():
                continue
            name = info.filename
            mode = (info.external_attr >> 16) & 0xFFFF
            if (info.create_system == 3 and mode & 0o170000 not in (0, 0o100000)) \
                    or "\\" in name or ":" in name or name.startswith("/") \
                    or any(p in ("", ".", "..") for p in name.split("/")):
                problems.append(f"invalid path: {name}")
                continue
            if name in names:
                problems.append(f"duplicate path: {name}")
            names.add(name)
            if name == "mod.wasm":
                problems.append("mod.wasm is refused")
            if name == "horizon-mod.toml" or name.startswith(("assets/", "data/")):
                problems.append(f"content entry in a code-only .hnmod: {name}")
                continue
            raw = zf.read(info)
            total += len(raw)
            if len(raw) > 64 << 20 or total > 64 << 20:
                problems.append("bundle exceeds 64 MiB")
        if "horizon-native.json" not in names:
            problems.append("missing horizon-native.json")
        if f"mixins/{MOD_ID}/horizon-mixin.toml" not in names:
            problems.append("missing horizon-mixin.toml")
        for name in sorted(names):
            if name == "horizon-native.json":
                continue
            prefix = f"mixins/{MOD_ID}/"
            if not name.startswith(prefix):
                problems.append(f"unexpected native source: {name}")
            elif not allowed(name[len(prefix):]):
                problems.append(f"unexpected native source: {name}")
        parsed = json.loads(zf.read("horizon-native.json"))
        mixin = zf.read(f"mixins/{MOD_ID}/horizon-mixin.toml").decode()

    if len(names) - 1 > 512:
        problems.append("more than 512 entries")
    for key in ("horizon", "mixins"):
        value = parsed.get(key, "")
        if len(value) != 40 or any(c not in "0123456789abcdef" for c in value):
            problems.append(f"{key} must be 40 lowercase hex characters")

    def toml_value(section: str, key: str):
        current = None
        for line in mixin.splitlines():
            line = line.split("#", 1)[0].strip()
            if line.startswith("[") and line.endswith("]"):
                current = line[1:-1].strip()
            elif "=" in line and current == section:
                k, v = line.split("=", 1)
                if k.strip() == key:
                    return v.strip().strip('"')
        return None

    if toml_value("mod", "id") != parsed["id"]:
        problems.append("horizon-mixin.toml [mod] id differs from horizon-native.json")
    if toml_value("mod", "version") != parsed["version"]:
        problems.append("horizon-mixin.toml [mod] version differs from horizon-native.json")
    if toml_value("compatibility", "horizon_revision") != parsed["horizon"]:
        problems.append("horizon-mixin.toml horizon_revision differs from horizon-native.json")

    print(f"file:   {OUT}")
    print(f"size:   {len(data)} bytes")
    print(f"sha256: {hashlib.sha256(data).hexdigest()}")
    if problems:
        print("PROBLEMS:")
        for p in problems:
            print(f"  - {p}")
        return 1
    print("bundle check: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
