// Every command the frontend invokes must be registered in the Tauri shell,
// otherwise the app fails at runtime with "Command ... not found".
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const api = readFileSync("src/lib/api.ts", "utf8");
const shell = readFileSync("src-tauri/src/lib.rs", "utf8");
const handler = shell.slice(shell.indexOf("generate_handler!["), shell.indexOf("]", shell.indexOf("generate_handler![")));

describe("Tauri commands", () => {
  const invoked = [...api.matchAll(/invoke<[^>]*>\("(\w+)"/g)].map((match) => match[1]);
  it.each(invoked)("%s is registered", (name) => {
    expect(handler).toMatch(new RegExp(`\\b${name}\\b`));
    expect(shell).toMatch(new RegExp(`fn ${name}\\b`));
  });
});
