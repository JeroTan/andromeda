"""Native integration checks for repeated updates and rejection without losing an install."""
import os
import pathlib
import plistlib
import subprocess
import sys
import tempfile
import unittest

SCRIPT = pathlib.Path(__file__).with_name("install-macos.py")


@unittest.skipUnless(sys.platform == "darwin", "macOS code-signing integration")
class InstallTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = pathlib.Path(self.temporary.name)
        self.destination = self.root / "installed/Andromeda.app"
        self.destination.parent.mkdir()

    def bundle(self, name, version):
        app = self.root / name / "Andromeda.app"
        (app / "Contents/MacOS").mkdir(parents=True)
        (app / "Contents/Resources/bin").mkdir(parents=True)
        info = {"CFBundleIdentifier": "io.andromeda.desktop", "CFBundleExecutable": "andromeda-desktop", "CFBundleName": "Andromeda", "CFBundlePackageType": "APPL", "CFBundleShortVersionString": version}
        (app / "Contents/Info.plist").write_bytes(plistlib.dumps(info))
        source = app.parent / "fixture.c"
        source.write_text('#include <stdio.h>\n#include <string.h>\nint main(int argc,char**argv){ if(argc>1 && !strcmp(argv[1],"tools")) puts("[{\\"name\\":\\"adm_stop\\"}]"); else puts("andromeda-mcp ' + version + '"); return 0;}\n')
        cli = app / "Contents/Resources/bin/andromeda"
        subprocess.run(["/usr/bin/clang", str(source), "-o", str(cli)], check=True, capture_output=True)
        (app / "Contents/MacOS/andromeda-desktop").write_bytes(cli.read_bytes())
        (app / "Contents/MacOS/andromeda-desktop").chmod(0o755)
        return app

    def invoke(self, app, destination=None):
        return subprocess.run([sys.executable, str(SCRIPT), "--app", str(app), "--destination", str(destination or self.destination)], capture_output=True, text=True)

    def test_two_updates_preserve_old_inode_and_leave_a_verifiable_runnable_bundle(self):
        original = self.bundle("first", "0.1.1")
        updated = self.bundle("second", "0.1.2")
        self.assertEqual(self.invoke(original).returncode, 0)
        installed = self.destination / "Contents/Resources/bin/andromeda"
        old_bytes = installed.read_bytes()
        source_bytes = (updated / "Contents/Resources/bin/andromeda").read_bytes()
        with installed.open("rb") as old:
            result = self.invoke(updated)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(old.read(), old_bytes)
            self.assertNotEqual(installed.read_bytes(), old_bytes)
        self.assertEqual((updated / "Contents/Resources/bin/andromeda").read_bytes(), source_bytes)
        self.assertEqual(subprocess.check_output([str(installed), "--version"], text=True).strip(), "andromeda-mcp 0.1.2")
        self.assertEqual(subprocess.run(["codesign", "--verify", "--deep", "--strict", str(self.destination)], capture_output=True).returncode, 0)
        self.assertFalse(list(self.destination.parent.glob(".andromeda-install-*")))

    def test_invalid_candidate_keeps_existing_install_and_outside_symlink_target(self):
        original = self.bundle("first", "0.1.1")
        self.assertEqual(self.invoke(original).returncode, 0)
        installed = self.destination / "Contents/Resources/bin/andromeda"
        old = installed.read_bytes()
        broken = self.bundle("broken", "0.1.2")
        (broken / "Contents/Info.plist").write_bytes(plistlib.dumps({"CFBundleIdentifier": "io.andromeda.desktop", "CFBundleExecutable": "andromeda-desktop", "CFBundleShortVersionString": "9.9.9"}))
        self.assertNotEqual(self.invoke(broken).returncode, 0)
        self.assertEqual(installed.read_bytes(), old)
        link = self.destination.parent / "alias.app"
        link.symlink_to(self.destination, target_is_directory=True)
        self.assertNotEqual(self.invoke(original, link).returncode, 0)
        self.assertEqual(installed.read_bytes(), old)


if __name__ == "__main__":
    unittest.main()
