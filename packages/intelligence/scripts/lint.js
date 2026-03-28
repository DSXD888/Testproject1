import { join } from "node:path";

const root = new URL("..", import.meta.url);
const srcDir = join(root.pathname, "src");
const bannedPatterns = [
  "../apps/",
  "../../apps/",
  "apps/desktop",
  "crates/",
  "src-tauri",
  "supabase",
  "daemon_bridge",
  "transport.ts",
];

const files = [];
for await (const file of new Bun.Glob("**/*.ts").scan({
  cwd: srcDir,
  absolute: true,
})) {
  files.push(file);
}

const violations = [];
for (const file of files) {
  let text;
  try {
    text = await Bun.file(file).text();
  } catch (error) {
    violations.push(
      `${file}: unable to read source file (${error instanceof Error ? error.message : String(error)})`,
    );
    continue;
  }

  for (const pattern of bannedPatterns) {
    if (text.includes(pattern)) {
      violations.push(`${file}: banned import/reference pattern "${pattern}"`);
    }
  }
}

if (violations.length > 0) {
  console.error("packages/intelligence lint failed:");
  for (const violation of violations) {
    console.error(`- ${violation}`);
  }
  process.exit(1);
}

console.log(`packages/intelligence lint passed (${files.length} source file(s) checked)`);
