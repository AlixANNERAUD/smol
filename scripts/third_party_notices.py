#!/usr/bin/env python3
"""Write THIRD_PARTY_NOTICES.md and licenses/ next to a bundled native runtime.

The SDK packages ship libkrun, libkrunfw (with its Linux kernel), the macOS GPU
libraries libkrun loads and an Alpine-based guest rootfs. This records each
component that is actually in DEST, where its source is, and its license text.

Usage: third_party_notices.py DEST [--rootfs agent-rootfs.tar] [--engine DIR]
  DEST     directory holding the bundled libs (written into)
  --rootfs guest rootfs tarball; its Alpine package list is read from it
  --engine smolvm engine checkout; its libkrun/libkrunfw submodule pins are read
"""
import argparse
import shutil
import subprocess
import sys
import tarfile
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
LICENSES = REPO / "licenses"

# (bundled file prefix, name, license, upstream, license files)
COMPONENTS = [
    ("libkrun.", "libkrun", "Apache-2.0", "https://github.com/smol-machines/libkrun",
     ["libkrun-LICENSE.txt"]),
    ("libkrunfw.", "libkrunfw and the Linux kernel it embeds",
     "LGPL-2.1-only (library), GPL-2.0-only (kernel)",
     "https://github.com/smol-machines/libkrunfw",
     ["libkrunfw-LGPL-2.1-only.txt", "linux-GPL-2.0-only.txt"]),
    ("libvirglrenderer.", "virglrenderer", "MIT",
     "https://gitlab.freedesktop.org/virgl/virglrenderer", ["virglrenderer-COPYING.txt"]),
    ("libepoxy.", "libepoxy", "MIT", "https://github.com/anholt/libepoxy",
     ["libepoxy-COPYING.txt"]),
    ("libMoltenVK.", "MoltenVK", "Apache-2.0", "https://github.com/KhronosGroup/MoltenVK",
     ["MoltenVK-LICENSE.txt"]),
]


def submodule_pins(engine):
    """{"libkrun": sha, "libkrunfw": sha} from the engine checkout, if readable."""
    if engine is None:
        return {}
    try:
        out = subprocess.run(
            ["git", "-C", str(engine), "ls-tree", "HEAD", "libkrun", "libkrunfw"],
            capture_output=True, text=True, check=True,
        ).stdout
    except (OSError, subprocess.CalledProcessError):
        return {}
    pins = {}
    for line in out.splitlines():
        meta, path = line.split("\t", 1)
        pins[path] = meta.split()[2]
    return pins


def read_member(tar, path):
    member = next((m for m in tar.getmembers() if m.name.lstrip("./") == path), None)
    return None if member is None else tar.extractfile(member).read().decode()


def alpine_packages(rootfs):
    """(release, [(name, version, license, origin)]) from the rootfs apk database."""
    with tarfile.open(rootfs) as tar:
        db = read_member(tar, "lib/apk/db/installed")
        release = (read_member(tar, "etc/alpine-release") or "").strip()
    if db is None:
        return release, []
    packages = []
    for record in db.split("\n\n"):
        fields = {}
        for line in record.splitlines():
            if len(line) > 2 and line[1] == ":":
                fields.setdefault(line[0], line[2:])
        if "P" in fields:
            packages.append((fields["P"], fields.get("V", ""), fields.get("L", ""),
                             fields.get("o", fields["P"])))
    return release, sorted(packages)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("dest", type=Path)
    parser.add_argument("--rootfs", type=Path)
    parser.add_argument("--engine", type=Path)
    args = parser.parse_args()

    present = [f.name for f in args.dest.iterdir()]
    pins = submodule_pins(args.engine)
    out = ["# Third-party notices", "",
           "This package bundles the components below. Each keeps its own license; "
           "the texts are in `licenses/`. The smolmachines code itself is Apache-2.0 "
           "(`licenses/smolmachines-LICENSE.txt`).", ""]
    copied = {"smolmachines-LICENSE.txt"}
    (args.dest / "licenses").mkdir(exist_ok=True)
    shutil.copyfile(REPO / "LICENSE", args.dest / "licenses" / "smolmachines-LICENSE.txt")

    for prefix, name, license_id, upstream, files in COMPONENTS:
        if not any(p.startswith(prefix) for p in present):
            continue
        pin = pins.get(name.split()[0])
        source = f"{upstream}/tree/{pin}" if pin else upstream
        out += [f"## {name}", "", f"- License: {license_id}", f"- Source: {source}"]
        if name.startswith("libkrunfw"):
            out.append("- The kernel release and the patches applied to it are named by "
                       "the `Makefile` and `patches/` at that source revision.")
        out.append(f"- License text: {', '.join(f'`licenses/{f}`' for f in files)}")
        out.append("")
        for f in files:
            if f not in copied:
                shutil.copyfile(LICENSES / f, args.dest / "licenses" / f)
                copied.add(f)

    if args.rootfs is not None and args.rootfs.is_file():
        release, packages = alpine_packages(args.rootfs)
        if not packages:
            sys.exit(f"third_party_notices: no Alpine package database in {args.rootfs}")
        out += ["## Guest root filesystem (`agent-rootfs.tar`)", "",
                f"An Alpine Linux {release} root filesystem with the smolvm guest agent "
                "(Apache-2.0). Source for each Alpine package is in "
                "[aports](https://gitlab.alpinelinux.org/alpine/aports), on the "
                "branch for that release, under the origin name and version below. "
                "The GPL-2.0 text is in `licenses/linux-GPL-2.0-only.txt`.", "",
                "| Package | Version | License | Origin |", "| --- | --- | --- | --- |"]
        out += [f"| {p} | {v} | {lic} | {o} |" for p, v, lic, o in packages]
        out.append("")
        if "linux-GPL-2.0-only.txt" not in copied:
            shutil.copyfile(LICENSES / "linux-GPL-2.0-only.txt",
                            args.dest / "licenses" / "linux-GPL-2.0-only.txt")

    (args.dest / "THIRD_PARTY_NOTICES.md").write_text("\n".join(out))
    print(f"third_party_notices: wrote {args.dest / 'THIRD_PARTY_NOTICES.md'}")


if __name__ == "__main__":
    main()
