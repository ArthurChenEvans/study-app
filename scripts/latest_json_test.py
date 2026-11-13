"""Tests for latest-json.py (updates.feature, R165). Run: python3 scripts/latest_json_test.py"""
import json, pathlib, subprocess, sys, tempfile, unittest

SCRIPT = pathlib.Path(__file__).with_name("latest-json.py")


class LatestJson(unittest.TestCase):
    def test_lists_each_package_under_its_platform_with_its_signature(self):
        with tempfile.TemporaryDirectory() as folder:
            folder = pathlib.Path(folder)
            (folder / "app.rpm.sig").write_text("rpm-signature\n")
            (folder / "setup.exe.sig").write_text("exe-signature\n")
            (folder / "notes.md").write_text("- Contents in the preview\n")
            out = folder / "latest.json"
            subprocess.run([sys.executable, str(SCRIPT), "0.1.9", str(folder / "notes.md"), str(out),
                            f"linux-x86_64-rpm={folder / 'app.rpm'}=https://example.com/app.rpm",
                            f"windows-x86_64-nsis={folder / 'setup.exe'}=https://example.com/setup.exe"], check=True, capture_output=True)
            feed = json.loads(out.read_text())
            self.assertEqual(feed["version"], "0.1.9")
            self.assertEqual(feed["notes"], "- Contents in the preview")
            self.assertEqual(feed["platforms"]["linux-x86_64-rpm"], {"signature": "rpm-signature", "url": "https://example.com/app.rpm"})
            self.assertEqual(feed["platforms"]["windows-x86_64-nsis"], {"signature": "exe-signature", "url": "https://example.com/setup.exe"})

    def test_refuses_a_package_with_no_signature(self):
        with tempfile.TemporaryDirectory() as folder:
            result = subprocess.run([sys.executable, str(SCRIPT), "0.1.9", "none.md", f"{folder}/latest.json",
                                     f"linux-x86_64-rpm={folder}/app.rpm=https://example.com/app.rpm"], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("no signature", result.stderr)


if __name__ == "__main__":
    unittest.main()
