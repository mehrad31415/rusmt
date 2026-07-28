// Runner: a JS TOML parser. Prints `OK` or `ERR <class>`.
import { readFileSync } from "node:fs";
const src = readFileSync(process.argv[2], "utf8");
try {
  const { parse } = await import("smol-toml");
  parse(src);
  console.log("OK");
} catch (e) {
  console.log("ERR " + String(e.message || e.name).split("\n")[0].slice(0, 60));
}
