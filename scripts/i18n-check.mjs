import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const localesDir = path.join(root, "src", "i18n", "locales");
const componentDir = path.join(root, "src", "components");
const allowlist = new Map([
  ["src\\components\\ConnectionList.vue", ["rect.right || y", "rect.right || clientY"]],
  ["src\\components\\ConnectionTreeItem.vue", ["item.name", "rect.right || y"]],
  ["src\\components\\TransferList.vue", ["visibleItems[virtualItem.index].name", "visibleItems[virtualItem.index].error"]],
  ["src\\components\\TunnelModal.vue", ["user@jump:22", "ssh -W %h:%p bastion"]],
]);

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, "utf8"));
}

function isObject(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function deepMerge(base, extra) {
  const output = { ...base };
  for (const [key, value] of Object.entries(extra)) {
    if (isObject(output[key]) && isObject(value)) {
      output[key] = deepMerge(output[key], value);
    } else {
      output[key] = value;
    }
  }
  return output;
}

function flatten(obj, prefix = "") {
  const keys = [];
  for (const [key, value] of Object.entries(obj)) {
    const nextKey = prefix ? `${prefix}.${key}` : key;
    if (isObject(value)) keys.push(...flatten(value, nextKey));
    else keys.push(nextKey);
  }
  return keys;
}

function diffKeys(a, b) {
  const aSet = new Set(a);
  const bSet = new Set(b);
  return {
    onlyA: [...aSet].filter((key) => !bSet.has(key)).sort(),
    onlyB: [...bSet].filter((key) => !aSet.has(key)).sort(),
  };
}

function walkVueFiles(dir) {
  const files = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) files.push(...walkVueFiles(full));
    else if (entry.isFile() && full.endsWith(".vue")) files.push(full);
  }
  return files;
}

function findHardcodedCandidates(file) {
  const source = fs.readFileSync(file, "utf8");
  const matches = [];
  const patterns = [
    /\b(?:title|placeholder)="([^"]*[A-Za-z\u4e00-\u9fff][^"]*)"/g,
    /\b(?:title|placeholder)='([^']*[A-Za-z\u4e00-\u9fff][^']*)'/g,
    />([^<>{}\n]*[A-Za-z\u4e00-\u9fff][^<>{}\n]*)</g,
  ];

  for (const pattern of patterns) {
    let match;
    while ((match = pattern.exec(source))) {
      const value = match[1].trim();
      if (!value) continue;
      if (value.startsWith("t(") || value.startsWith("{{") || value.includes("file.path")) continue;
      if (/^(https?:\/\/|sk-\.\.\.|gpt-3\.5-turbo|id_ed25519|-----BEGIN OPENSSH PRIVATE KEY-----\.\.\.)/.test(value)) continue;
      if (/^[A-Za-z0-9_./:%@+\- ()[\]{},"'|&]+$/.test(value) === false && /[\u4e00-\u9fff]/.test(value) === false) continue;
      matches.push(value);
    }
  }

  return [...new Set(matches)];
}

const zh = readJson(path.join(localesDir, "zh.json"));
const en = readJson(path.join(localesDir, "en.json"));
const zhExtra = readJson(path.join(localesDir, "zh.extra.json"));
const enExtra = readJson(path.join(localesDir, "en.extra.json"));

const baseDiff = diffKeys(flatten(zh), flatten(en));
const mergedZh = deepMerge(zh, zhExtra);
const mergedEn = deepMerge(en, enExtra);
const mergedDiff = diffKeys(flatten(mergedZh), flatten(mergedEn));

const candidates = walkVueFiles(componentDir)
  .map((file) => {
    const rel = path.relative(root, file);
    const allowed = new Set(allowlist.get(rel) || []);
    return {
      file: rel,
      matches: findHardcodedCandidates(file).filter((value) => !allowed.has(value)),
    };
  })
  .filter((entry) => entry.matches.length > 0);

let hasError = false;

function reportDiff(title, diff) {
  if (diff.onlyA.length === 0 && diff.onlyB.length === 0) return;
  hasError = true;
  console.error(`\n${title}`);
  if (diff.onlyA.length > 0) console.error(`Only in A:\n${diff.onlyA.join("\n")}`);
  if (diff.onlyB.length > 0) console.error(`Only in B:\n${diff.onlyB.join("\n")}`);
}

reportDiff("Base locale key mismatch (zh.json vs en.json)", baseDiff);
reportDiff("Merged locale key mismatch ((base+extra) zh vs en)", mergedDiff);

if (candidates.length > 0) {
  hasError = true;
  console.log("\nPotential hardcoded UI strings:");
  for (const entry of candidates) {
    console.log(`- ${entry.file}`);
    for (const match of entry.matches.slice(0, 12)) {
      console.log(`  ${match}`);
    }
  }
}

if (hasError) {
  process.exitCode = 1;
} else {
  console.log("i18n checks passed.");
}
