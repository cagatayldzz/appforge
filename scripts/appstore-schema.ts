//! Downloads Apple's OpenAPI document and compares endpoint and component changes.

import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const source = "https://developer.apple.com/sample-code/app-store-connect/app-store-connect-openapi-specification.zip";
const baseline = fileURLToPath(new URL("../crates/appstore/schema/openapi.oas.json", import.meta.url));
const methods = new Set(["get", "put", "post", "delete", "options", "head", "patch", "trace"]);

type ObjectValue = Record<string, unknown>;
type Spec = { info: { version: string }; paths: Record<string, ObjectValue>; components?: Record<string, ObjectValue> };

function parse(bytes: Buffer): Spec {
  const value = JSON.parse(bytes.toString("utf8"));
  if (!value?.openapi || value.info?.title !== "App Store Connect API" || typeof value.info.version !== "string" || !Object.keys(value.paths ?? {}).length) {
    throw new Error("Expected Apple's App Store Connect OpenAPI document.");
  }
  return value;
}

function command(tool: string, args: string[]): Buffer {
  const result = spawnSync(tool, args, { maxBuffer: 32 * 1024 * 1024 });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${tool} failed: ${result.stderr.toString().trim()}`);
  return result.stdout;
}

function entries(spec: Spec): Map<string, unknown> {
  const entries = new Map<string, unknown>();
  for (const [path, item] of Object.entries(spec.paths)) {
    for (const [method, operation] of Object.entries(item)) {
      if (methods.has(method)) {
        entries.set(`${method.toUpperCase()} ${path}`, { operation, parameters: item.parameters, servers: item.servers });
      }
    }
  }
  for (const [group, definitions] of Object.entries(spec.components ?? {})) {
    for (const [name, definition] of Object.entries(definitions)) entries.set(`components/${group}/${name}`, definition);
  }
  return entries;
}

function comparable(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(comparable);
  if (value === null || typeof value !== "object") return value;
  const object = value as ObjectValue;
  return Object.fromEntries(Object.keys(object).sort().map((key) => [key, comparable(object[key])]));
}

function deprecated(value: unknown): boolean {
  return (value as { operation?: { deprecated?: boolean } } | undefined)?.operation?.deprecated === true;
}

function compare(before: Spec, after: Spec) {
  const old = entries(before);
  const latest = entries(after);
  const changes: string[] = [];
  for (const key of Array.from(new Set([...old.keys(), ...latest.keys()])).sort()) {
    if (!old.has(key)) changes.push(`Added    ${key}${deprecated(latest.get(key)) ? " (deprecated)" : ""}`);
    else if (!latest.has(key)) changes.push(`Removed  ${key}`);
    else if (JSON.stringify(comparable(old.get(key))) !== JSON.stringify(comparable(latest.get(key)))) {
      const notice = deprecated(latest.get(key)) === deprecated(old.get(key)) ? "" :
        deprecated(latest.get(key)) ? " (newly deprecated)" : " (deprecation removed)";
      changes.push(`Changed  ${key}${notice}`);
    }
  }
  console.log(`App Store Connect API ${before.info.version} → ${after.info.version}`);
  console.log(changes.length ? changes.join("\n") : "No endpoint or component changes.");
}

const args = process.argv.slice(2);
const write = args.includes("--write");
const files = args.filter((arg) => arg !== "--write");
if (files.length > 1 || files.some((arg) => arg.startsWith("-"))) {
  console.error("Usage: bun run appstore:schema [openapi.oas.json | archive.zip] [--write]");
  process.exit(2);
}

const scratch = mkdtempSync(join(tmpdir(), "appforge-openapi-"));
try {
  let input = files[0];
  if (!input) {
    input = join(scratch, "spec.zip");
    command("curl", ["--fail", "--silent", "--show-error", "--location", "--max-time", "60", "--output", input, source]);
  }
  const bytes = input.endsWith(".zip") ? command("unzip", ["-p", input, "openapi.oas.json"]) : readFileSync(input);
  const spec = parse(bytes);
  compare(parse(readFileSync(baseline)), spec);
  if (write) {
    const staging = mkdtempSync(join(dirname(baseline), ".openapi-"));
    try {
      const candidate = join(staging, "openapi.oas.json");
      writeFileSync(candidate, bytes);
      renameSync(candidate, baseline);
      console.log(`Saved ${spec.info.version} to crates/appstore/schema/openapi.oas.json.`);
    } finally {
      rmSync(staging, { recursive: true, force: true });
    }
  }
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
} finally {
  rmSync(scratch, { recursive: true, force: true });
}
