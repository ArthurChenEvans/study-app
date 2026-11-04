import { describe, expect, it } from "vitest";
import { countList, counted } from "./transfer";

describe("export and import words", () => {
  it("counts in the singular and plural", () => {
    expect(counted({ what: "Quizzes", count: 1 })).toBe("1 quiz");
    expect(counted({ what: "card reviews", count: 214 })).toBe("214 card reviews");
    expect(counted({ what: "Sources", count: 1 })).toBe("1 source");
  });

  it("lists counts in a sentence, leaving out images and notes", () => {
    expect(countList([{ what: "Questions", count: 18 }, { what: "images", count: 2 }, { what: "Cards", count: 12 }, { what: "Sources", count: 1 }])).toBe("18 questions, 12 cards and 1 source");
    expect(countList([{ what: "topics", count: 1 }])).toBe("1 topic");
    expect(countList([])).toBe("nothing");
  });
});
