#!/usr/bin/env python3
"""Install a locally built Andromeda bundle without overwriting loaded Mach-O files."""
import argparse
import json
import os
import pathlib
import plistlib
import shutil
import subprocess
import sys
import tempfile
import uuid

IDENTIFIER = "io.andromeda.desktop"


def run(command):
    result = subprocess.run(command, capture_output=True, text=True, timeout=20)
    if result.returncode:
        # Do not dump CLI configuration, environment or private paths on failure.
        raise RuntimeError(f"{pathlib.Path(command[0]).name} failed with exit {result.returncode}")
    return result.stdout


def regular_file(app, relative):
    path = app
    for component in pathlib.Path(relative).parts:
        path = path / component
        if path.is_symlink():
            raise RuntimeError("Bundle metadata and executable paths cannot contain symbolic links")
    if not path.is_file() or not path.resolve().is_relative_to(app.resolve()):
        raise RuntimeError("Bundle file is missing or resolves outside the application")
    return path


def info(app):
    with regular_file(app, "Contents/Info.plist").open("rb") as file:
        metadata = plistlib.load(file)
    if metadata.get("CFBundleIdentifier") != IDENTIFIER:
        raise RuntimeError("Source and destination must be Andromeda application bundles")
    executable = metadata.get("CFBundleExecutable", "")
    if not executable or pathlib.Path(executable).name != executable:
        raise RuntimeError("Invalid application executable name")
    regular_file(app, "Contents/MacOS/" + executable)
    return metadata


def verify(app):
    metadata = info(app)
    cli = regular_file(app, "Contents/Resources/bin/andromeda")
    run(["/usr/bin/codesign", "--verify", "--deep", "--strict", str(app)])
    version = run([str(cli), "--version"]).strip()
    if version != "andromeda-mcp " + metadata["CFBundleShortVersionString"]:
        raise RuntimeError("Desktop and MCP server versions do not match")
    catalog = json.loads(run([str(cli), "tools"]))
    if not isinstance(catalog, list) or not catalog or not all(
        isinstance(tool, dict) and tool.get("name", "").startswith("adm_") for tool in catalog
    ):
        raise RuntimeError("Bundled MCP server returned an invalid tool catalog")
    return {"version": metadata["CFBundleShortVersionString"], "tools": len(catalog)}


def install(source, destination):
    if sys.platform != "darwin":
        raise RuntimeError("This installer requires macOS")
    source = source.resolve(strict=True)
    destination = destination.absolute()
    if destination.suffix != ".app" or destination.is_symlink():
        raise RuntimeError("Destination must be an application path, not a symbolic link")
    if source == destination.resolve() or source in destination.resolve().parents or destination.resolve() in source.parents:
        raise RuntimeError("Source and destination must be separate bundles")
    info(source)
    if destination.exists():
        installed_info = info(destination)
        main = destination / "Contents/MacOS" / installed_info["CFBundleExecutable"]
        processes = subprocess.check_output(["ps", "-axo", "comm="], text=True)
        if str(main) in {line.strip() for line in processes.splitlines()}:
            raise RuntimeError("Close Andromeda before installing the update")
    if not destination.parent.is_dir():
        raise RuntimeError("Destination parent directory does not exist")
    with tempfile.TemporaryDirectory(prefix=".andromeda-install-", dir=destination.parent) as temporary:
        temporary = pathlib.Path(temporary)
        candidate = temporary / destination.name
        previous = destination.parent / (".andromeda-previous-" + uuid.uuid4().hex + ".app")
        shutil.copytree(source, candidate, symlinks=True)
        info(candidate)
        cli = regular_file(candidate, "Contents/Resources/bin/andromeda")
        # Local ad-hoc signing supplies bundle integrity, not Developer ID or notarization.
        run(["/usr/bin/codesign", "--force", "--sign", "-", str(cli)])
        run(["/usr/bin/codesign", "--force", "--sign", "-", str(candidate)])
        verify(candidate)
        had_previous = destination.exists()
        if had_previous:
            os.rename(destination, previous)
        try:
            os.rename(candidate, destination)
            result = verify(destination)
        except BaseException:
            if destination.exists():
                shutil.rmtree(destination)
            if had_previous:
                try:
                    os.rename(previous, destination)
                except OSError as error:
                    raise RuntimeError(f"Update failed; previous app preserved at {previous}") from error
            raise
        if had_previous:
            shutil.rmtree(previous)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", type=pathlib.Path, required=True)
    parser.add_argument("--destination", type=pathlib.Path, default=pathlib.Path("/Applications/Andromeda.app"))
    args = parser.parse_args()
    try:
        print(json.dumps({"installed": True, **install(args.app, args.destination)}))
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
        parser.exit(1, f"Andromeda installation failed: {error}\n")


if __name__ == "__main__":
    main()
