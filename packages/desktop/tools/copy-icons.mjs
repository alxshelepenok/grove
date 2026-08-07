import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
} from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

const pkgRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = join(pkgRoot, "..", "..");
const srcDir = join(repoRoot, "icons");
const outDir = join(pkgRoot, "icons");
const manifestPath = join(pkgRoot, "tools", "icons.manifest.json");
const outManifest = join(pkgRoot, "manifests", "icons.ts");

const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
mkdirSync(outDir, { recursive: true });

const syncFromRepo = existsSync(srcDir);
const camel = (slug) => slug.replace(/-(\w)/g, (_, c) => c.toUpperCase());

const imports = [];
const entries = [];
const missing = [];

for (const slug of manifest) {
  if (syncFromRepo) {
    const src = join(srcDir, `${slug}-solid-rounded.svg`);
    try {
      copyFileSync(src, join(outDir, `${slug}.svg`));
    } catch {
      missing.push(`${slug}-solid-rounded.svg`);
      continue;
    }
  } else if (!existsSync(join(outDir, `${slug}.svg`))) {
    missing.push(`${slug}.svg`);
    continue;
  }

  const ident = `icon${camel(slug).replace(/^./, (c) => c.toUpperCase())}`;
  imports.push(`import ${ident} from "../icons/${slug}.svg" with { type: "text" };`);
  entries.push(`  "${slug}": ${ident},`);
}

if (missing.length > 0) {
  console.error(`Missing icons (${missing.length}):`);
  for (const file of missing) console.error(`  ${file}`);
  process.exit(1);
}

const ts = `// @ts-nocheck
${imports.join("\n")}

export const icons: Record<string, string> = {
${entries.join("\n")}
};
`;

writeFileSync(outManifest, ts, "utf8");
console.log(
  `Generated manifests/icons.ts (${entries.length} icons${syncFromRepo ? `, synced from ${srcDir}` : ""})`,
);
