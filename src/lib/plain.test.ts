import { describe, expect, it } from "vitest";
import { plainText } from "./plain";

// R214: lists show a card's text, never its markup.
describe("plainText", () => {
  it("drops tags and keeps the words", () => {
    expect(plainText('Top to bottom: <span class="cloze">application</span>, presentation')).toBe("Top to bottom: application, presentation");
  });
  it("turns entities back into characters", () => {
    expect(plainText("i &lt;= n &amp;&amp; done &quot;ok&quot;")).toBe('i <= n && done "ok"');
  });
  it("puts a space where a line break or block was", () => {
    expect(plainText("First line<br>second<div>third</div>")).toBe("First line second third");
  });
  it("leaves plain text alone, including a lone < that isn't a tag", () => {
    expect(plainText("a < b and c > d")).toBe("a < b and c > d");
  });
  it("collapses runs of spaces", () => {
    expect(plainText("  lots   of\n\nspace ")).toBe("lots of space");
  });
});
