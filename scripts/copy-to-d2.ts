#!/usr/bin/env bun
/**
 * Safely copy a Diablo II save file into an explicitly selected destination.
 * The destination filename receives a `_d2a` suffix; originals are protected.
 *
 * Usage:
 *   bun run scripts/copy-to-d2.ts <source-file> <destination-directory>
 */

import { copyFile, stat, readFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { join, basename, extname } from "node:path";

const D2A_SUFFIX = "_d2a";

function d2aName(filename: string): string {
  const base = basename(filename);
  const ext = extname(base);
  const name = base.slice(0, base.length - ext.length);
  return `${name}${D2A_SUFFIX}${ext}`;
}

async function sha256(filePath: string): Promise<string> {
  const buf = await readFile(filePath);
  return createHash("sha256").update(buf).digest("hex");
}

async function main() {
  const args = process.argv.slice(2);
  if (args.length < 2) {
    console.error("Usage: bun run scripts/copy-to-d2.ts <source-file> <destination-directory>");
    process.exit(1);
  }

  const source = args[0];
  const destDir = args[1];

  // Validate source exists
  let srcStat;
  try {
    srcStat = await stat(source);
  } catch {
    console.error(`ERROR: source file not found: ${source}`);
    process.exit(1);
  }
  if (!srcStat.isFile()) {
    console.error(`ERROR: source is not a file: ${source}`);
    process.exit(1);
  }

  // Ensure dest dir exists
  try {
    await stat(destDir);
  } catch {
    await $`mkdir -p ${destDir}`;
  }

  const destName = d2aName(source);
  const destPath = join(destDir, destName);

  // Safety: refuse to write over an original (non-d2a) file
  const originalCandidate = join(destDir, basename(source));
  try {
    await stat(originalCandidate);
    console.error(
      `ERROR: refusing to overwrite original file: ${originalCandidate}\n` +
        `Use a different source name or copy to a different directory.`
    );
    process.exit(1);
  } catch {
    // original doesn't exist — good
  }

  // If a d2a copy already exists, back it up before replacing
  try {
    await stat(destPath);
    const backupPath = `${destPath}.bak-${Date.now()}`;
    await $`cp ${destPath} ${backupPath}`;
    console.warn(`WARN: existing d2a copy backed up to: ${backupPath}`);
  } catch {
    // no existing copy — fine
  }

  // Perform the copy
  await copyFile(source, destPath);

  // Verify: byte size must match
  const dstStat = await stat(destPath);
  if (dstStat.size !== srcStat.size) {
    console.error(
      `ERROR: copy size mismatch! source=${srcStat.size} dest=${dstStat.size}`
    );
    process.exit(1);
  }

  const srcHash = await sha256(source);
  const dstHash = await sha256(destPath);
  if (srcHash !== dstHash) {
    console.error("ERROR: copy hash mismatch! Copy may be corrupted.");
    process.exit(1);
  }

  console.log(`OK: copied ${source}`);
  console.log(`      -> ${destPath}`);
  console.log(`      size=${dstStat.size} bytes sha256=${dstHash}`);
  console.log(`      Original untouched.`);
}

main().catch((err) => {
  console.error("FATAL:", err);
  process.exit(1);
});
