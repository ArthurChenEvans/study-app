"""End-to-end: drives the real app window through tauri-driver and converts slides
over the same IPC the preview uses. Run with e2e/run.sh."""
import base64, os, pathlib, sys
from selenium import webdriver
from selenium.webdriver.common.options import ArgOptions

root = pathlib.Path(__file__).resolve().parent.parent
app = os.environ.get("APP", str(root / "target/debug/study-app"))
fixture = root / "crates/study-api/tests/fixtures/pipeline.pptx"

options = ArgOptions()
options.set_capability("tauri:options", {"application": app})
options.set_capability("browserName", "wry")
driver = webdriver.Remote("http://127.0.0.1:4444", options=options)
driver.set_script_timeout(120)
try:
    result = driver.execute_async_script(
        """
        const [b64, done] = arguments;
        window.__TAURI_INTERNALS__.invoke("slides_to_pdf", { name: "pipeline.pptx", data: b64 })
          .then((out) => { const view = new Uint8Array(out); done({ ok: true, size: view.length, head: String.fromCharCode(...view.slice(0, 5)) }); })
          .catch((error) => done({ ok: false, error: JSON.stringify(error) }));
        """,
        base64.b64encode(fixture.read_bytes()).decode(),
    )
    print(result)
    assert result.get("ok") and result["head"] == "%PDF-", result
    print("PASS: slides converted to PDF in the running app")
finally:
    driver.quit()
