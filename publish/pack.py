"""打包 VeinSweep 的 .hmod，并按新版包读取器（horizon_mod_package::Package /
horizon_mod_package::index::native::Bundle）的规则做结构校验。

用法：python publish/pack.py
改动源码后运行它，然后把打印出的 sha256 更新到
publish/registry/mods/veinsweep.toml 和 publish/SUBMIT.md。

新版格式（Horizon Nightly 2026-11-18，0b14f8a）：
- 每个 mod 一个 .hmod（zip），根目录是 horizon-mod.toml；
- 原生 mod 带 Cargo.toml、Cargo.lock 与 src/；
- 可选 icon.png、assets/<id>/、data/<id>/；
- .horizon/（SDK 缓存）永远不入包。
"""

import hashlib
import pathlib
import re
import sys
import tomllib
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
MOD = ROOT / "mod"
OUT = ROOT / "publish" / "veinsweep-0.5.0.hmod"

MOD_ID = "veinsweep"
VERSION = "0.5.0"
GAME = "26.3"
# 开发与实测所用的 Horizon 提交（Nightly 2026-11-20）。
HORIZON = "0a875da91dbb0fdcb7ba8c42c11341aa229af51a"
MIXINS = "e59da9094f0c2fdda6c627fe9eff22dec6a906f0"

# 根目录允许的固定文件；除此之外只允许 src/、assets/、data/ 下的文件。
ROOT_FILES = {
    "horizon-mod.toml",
    "Cargo.toml",
    "Cargo.lock",
    "icon.png",
    "README.md",
    "README.txt",
    "LICENSE",
    "LICENSE.md",
    "LICENCE",
    "COPYING",
    "NOTICE",
    ".gitignore",
}
ROOT_DIRS = ("src/", "assets/", "data/")


def collect() -> list[pathlib.Path]:
    files = []
    for path in sorted(MOD.rglob("*")):
        if not path.is_file():
            continue
        rel = path.relative_to(MOD).as_posix()
        if rel.startswith(".horizon/") or rel.startswith("target/"):
            continue
        if rel in ROOT_FILES or rel.startswith(ROOT_DIRS):
            files.append(path)
        else:
            raise SystemExit(f"unexpected package path: {rel}")
    return files


def main() -> int:
    OUT.parent.mkdir(parents=True, exist_ok=True)
    files = collect()
    entries = [(p.relative_to(MOD).as_posix(), p.read_bytes()) for p in files]

    with zipfile.ZipFile(OUT, "w", zipfile.ZIP_DEFLATED) as zf:
        for name, data in entries:
            info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 3
            info.external_attr = (0o100644 & 0xFFFF) << 16
            zf.writestr(info, data)

    data = OUT.read_bytes()
    problems: list[str] = []
    names: set[str] = set()
    total = 0
    manifest = tomllib.loads((MOD / "horizon-mod.toml").read_text(encoding="utf-8"))

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
            raw = zf.read(info)
            total += len(raw)
            if len(raw) > 64 << 20 or total > 64 << 20:
                problems.append("package exceeds 64 MiB")
        if "horizon-mod.toml" not in names:
            problems.append("missing horizon-mod.toml")
        if "Cargo.toml" not in names or not any(n.startswith("src/") for n in names):
            problems.append("native mod needs Cargo.toml and src/")
        for name in sorted(names):
            if name in ROOT_FILES or name.startswith(ROOT_DIRS):
                continue
            problems.append(f"unexpected package path: {name}")
        if "mod.wasm" in names:
            problems.append("mod.wasm is refused in a native mod")
        # 解包后总大小与文件数限制
    if len(names) > 512:
        problems.append("more than 512 entries")

    # 清单一致性
    if manifest.get("id") != MOD_ID:
        problems.append("manifest id differs")
    if manifest.get("version") != VERSION:
        problems.append("manifest version differs")
    native = manifest.get("native", {})
    if native.get("game") != GAME:
        problems.append("manifest [native] game differs")
    for key, expected in (("horizon", HORIZON), ("mixins", MIXINS)):
        value = native.get(key, "")
        if value != expected:
            problems.append(f"manifest [native] {key} differs from the pinned build")
    if not re.fullmatch(r"[a-z0-9_]{1,64}", manifest.get("id", "")):
        problems.append("invalid mod id")
    if manifest.get("api") != "0.2":
        problems.append('manifest api must be "0.2"')

    cargo = tomllib.loads((MOD / "Cargo.toml").read_text(encoding="utf-8"))
    deps = cargo.get("dependencies", {})
    api = deps.get("horizon-mod-api", {})
    if api.get("path") != ".horizon/sdk/api/crates/horizon-mod-api":
        problems.append("Cargo.toml must name the SDK by .horizon/sdk/api/crates/horizon-mod-api")

    print(f"file:   {OUT}")
    print(f"size:   {len(data)} bytes")
    print(f"sha256: {hashlib.sha256(data).hexdigest()}")
    print(f"files:  {len(names)}")
    if problems:
        print("PROBLEMS:")
        for p in problems:
            print(f"  - {p}")
        return 1
    print("bundle check: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())

