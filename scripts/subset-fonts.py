#!/usr/bin/env python3
"""Rebuilds the fonts in `assets/fonts/` from the upstream releases.

The app embeds three small font files (see `src/ui/fonts.rs`):

* Inter Regular and Inter SemiBold (SIL OFL 1.1), reduced to the scripts a settings window needs;
* Phosphor Regular and Phosphor Fill (MIT), reduced to the icons named in `src/ui/symbols.rs`.

Nothing here runs at build time: the output is committed, so a normal build needs no Python.
Run it only when an icon is added to (or removed from) `symbols.rs`:

    npm pack @phosphor-icons/web@2.1.2 @expo-google-fonts/inter@0.4.2
    python3 scripts/subset-fonts.py <dir with the two unpacked packages>

Requires `pip install fonttools`.
"""

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "assets" / "fonts"
SYMBOLS = ROOT / "src" / "ui" / "symbols.rs"

# Scripts kept from Inter: Basic Latin, Latin-1, Latin Extended A and B, Greek, Cyrillic, general
# punctuation, currency, arrows, the Mac key symbols and a few shapes.
INTER_RANGES = (
    "U+0020-007E,U+00A0-024F,U+0300-036F,U+0370-03FF,U+0400-04FF,U+1E00-1EFF,"
    "U+2000-206F,U+20A0-20CF,U+2190-21FF,U+2212,U+2300-23FF,U+25A0-25FF"
)


def icon_codepoints() -> list[int]:
    """The private-use code points that `symbols.rs` uses, read from its `'\\u{...}'` literals."""
    text = SYMBOLS.read_text()
    return sorted({int(m, 16) for m in re.findall(r"'\\u\{([0-9a-fA-F]+)\}'", text)})


def subset(src: Path, dst: Path, unicodes: str, features: str) -> None:
    subprocess.run(
        [
            "pyftsubset", str(src), f"--unicodes={unicodes}", f"--layout-features={features}",
            "--no-hinting", "--desubroutinize", "--drop-tables+=DSIG,STAT,MVAR,HVAR,GDEF", f"--output-file={dst}",
        ],
        check=True,
    )
    print(f"{dst.name}: {dst.stat().st_size} bytes")


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    pkgs = Path(sys.argv[1])
    phosphor = pkgs / "x_phosphor-icons-web-2.1.2" / "package" / "src"
    inter = pkgs / "x_expo-google-fonts-inter-0.4.2" / "package"
    OUT.mkdir(parents=True, exist_ok=True)
    icons = ",".join(f"U+{c:04X}" for c in icon_codepoints())
    subset(phosphor / "regular" / "Phosphor.ttf", OUT / "Icons-Regular.ttf", icons, "")
    subset(phosphor / "fill" / "Phosphor-Fill.ttf", OUT / "Icons-Fill.ttf", icons, "")
    subset(inter / "400Regular" / "Inter_400Regular.ttf", OUT / "Inter-Regular.ttf", INTER_RANGES, "kern")
    subset(inter / "600SemiBold" / "Inter_600SemiBold.ttf", OUT / "Inter-SemiBold.ttf", INTER_RANGES, "kern")


if __name__ == "__main__":
    main()
