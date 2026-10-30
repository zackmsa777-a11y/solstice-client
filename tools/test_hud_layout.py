#!/usr/bin/env python3
"""Compile/run the host-free HUD model tests with an explicitly selectable rustc."""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rustc", default=os.environ.get("RUSTC", "rustc"))
    args = parser.parse_args()
    source = Path(__file__).resolve().parents[1] / "mixins/solstice/hud_layout.rs"
    with tempfile.TemporaryDirectory(prefix="sunset-hud-tests-") as directory:
        directory = Path(directory)
        wrapper = directory / "tests.rs"
        wrapper.write_text(f'#[path = "{source.as_posix()}"] mod hud_layout;\n', encoding="utf-8")
        binary = directory / ("hud-layout-tests.exe" if os.name == "nt" else "hud-layout-tests")
        subprocess.run([args.rustc, "--edition=2024", "--test", str(wrapper), "-o", str(binary)], check=True)
        subprocess.run([str(binary)], check=True)


if __name__ == "__main__":
    main()
