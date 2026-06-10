#!/usr/bin/env python3

import os
from pathlib import Path
from string import Template
from urllib.parse import quote


RELEASE_BASE_URL = "https://github.com/medulla-sh/cuet/releases/download"


def main() -> None:
    version = os.environ["VERSION"]
    encoded_tag = quote(os.environ["TAG_NAME"], safe="")

    dist_dir = Path(os.environ.get("DIST_DIR", "dist"))
    tap_dir = Path(os.environ.get("TAP_DIR", "tap"))
    template_path = Path(
        os.environ.get("FORMULA_TEMPLATE", tap_dir / "templates" / "cuet.rb.tpl")
    )
    formula_path = Path(os.environ.get("FORMULA_OUTPUT", tap_dir / "Formula" / "cuet.rb"))

    artifacts = {
        "macos_arm": f"cuet-{version}-aarch64-apple-darwin.tar.gz",
        "macos_x86": f"cuet-{version}-x86_64-apple-darwin.tar.gz",
        "linux_arm": f"cuet-{version}-aarch64-unknown-linux-gnu.tar.gz",
        "linux_x86": f"cuet-{version}-x86_64-unknown-linux-gnu.tar.gz",
    }

    values = {"version": version}
    for platform, artifact in artifacts.items():
        values[f"{platform}_url"] = f"{RELEASE_BASE_URL}/{encoded_tag}/{artifact}"
        values[f"{platform}_sha256"] = (dist_dir / f"{artifact}.sha256").read_text().split()[0]

    formula = Template(template_path.read_text()).substitute(values)

    formula_path.parent.mkdir(parents=True, exist_ok=True)
    formula_path.write_text(formula)


if __name__ == "__main__":
    main()
