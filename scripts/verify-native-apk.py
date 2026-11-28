#!/usr/bin/env python3
"""Fail if the signed native APK violates the initial Android packaging contract."""

import argparse
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from zipfile import ZipFile


def output(*args):
    return subprocess.check_output(args, text=True)


def require(condition, message):
    if not condition:
        raise SystemExit(message)


def verify(apk):
    sdk = Path(os.environ["ANDROID_HOME"])
    pins = Path(__file__).resolve().parents[1] / "android"
    build_tools = sdk / "build-tools" / (pins / "build-tools-version.txt").read_text().strip()
    ndk = Path(os.environ.get("ANDROID_NDK_HOME", sdk / "ndk" / (pins / "ndk-version.txt").read_text().strip()))
    readelf, = ndk.glob("toolchains/llvm/prebuilt/*/bin/llvm-readelf")
    subprocess.run([build_tools / "apksigner", "verify", apk], check=True)
    subprocess.run([build_tools / "zipalign", "-c", "-P", "16", "4", apk], check=True)

    aapt = build_tools / "aapt2"
    badging = output(aapt, "dump", "badging", apk)
    package = re.search(r"^package: name='tech\.lolli\.toolbox' versionCode='(\d+)'", badging, re.M)
    require(package and int(package[1]) > 192503, "Unexpected ID or non-increasing baseline versionCode")
    require("native-code: 'arm64-v8a'" in badging, "Expected arm64-v8a")
    require("launchable-activity: name='tech.lolli.toolbox.NativeClientActivity'" in badging, "Unexpected launcher")
    manifest = output(aapt, "dump", "xmltree", "--file", "AndroidManifest.xml", apk)
    for field, value in [("minSdkVersion", "33"), ("allowBackup", "false")]:
        require(re.search(rf"android:{field}\(0x[\da-f]+\)={value}$", manifest, re.M), f"Incorrect {field}")
    require(re.search(r"^\s*uses-feature: name='android.hardware.vulkan.version' version='4206592'$", badging, re.M), "Missing required Vulkan 1.3 feature")

    with ZipFile(apk) as archive, tempfile.TemporaryDirectory() as temp:
        members = archive.namelist()
        require(len(members) == len(set(members)), "Duplicate APK entries")
        require("classes.dex" in members, "Missing Android bytecode")
        dex = Path(temp) / "classes.dex"
        dex.write_bytes(archive.read("classes.dex"))
        require("'Ltech/lolli/toolbox/NativeClientActivity;'" in output(build_tools / "dexdump", dex), "Missing activity in DEX")
        libraries = [name for name in members if name.endswith(".so")]
        entry = "lib/arm64-v8a/libruserbox.so"
        require(entry in libraries, "NativeActivity library missing")
        for name in libraries:
            require(re.fullmatch(r"lib/arm64-v8a/[^/]+\.so", name), f"Unexpected library {name}")
            lib = Path(temp) / "library.so"
            lib.write_bytes(archive.read(name))
            header = output(readelf, "-h", lib)
            require(re.search(r"Machine:\s+AArch64\b", header), f"{name} is not AArch64")
            segments = [line.split() for line in output(readelf, "-lW", lib).splitlines() if line.split()[:1] == ["LOAD"]]
            require(segments, f"No ELF LOAD segments in {name}")
            for segment in segments:
                offset, vaddr, alignment = (int(segment[i], 16) for i in (1, 2, -1))
                require(alignment >= 16384 and (vaddr - offset) % 16384 == 0, f"ELF segment fails 16 KiB alignment in {name}")
            if name == entry:
                symbols = output(readelf, "--dyn-syms", "-W", lib)
                for symbol in ["ANativeActivity_onCreate", "android_main"]:
                    require(re.search(rf"GLOBAL\s+DEFAULT\s+\d+\s+{symbol}$", symbols, re.M), f"Missing export {symbol}")
    print(f"Verified {apk} (versionCode {package[1]}, {len(libraries)} arm64 native libraries)")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("apk", type=Path)
    try:
        verify(parser.parse_args().apk)
    except (OSError, subprocess.CalledProcessError) as err:
        sys.exit(str(err))
