#!/usr/bin/env node

const { existsSync } = require("node:fs");
const path = require("node:path");

const root = path.join(__dirname, "..");
const platforms = [
  ["darwin-x64", "trough"],
  ["darwin-arm64", "trough"],
  ["linux-x64", "trough"],
  ["linux-arm64", "trough"],
  ["win32-x64", "trough.exe"],
  ["win32-arm64", "trough.exe"]
];

const missing = platforms
  .map(([platform, executable]) => path.join(root, "dist", platform, executable))
  .filter((binary) => !existsSync(binary));

if (missing.length > 0) {
  console.error("Missing npm release binaries:");
  for (const binary of missing) {
    console.error(`  ${binary}`);
  }
  process.exit(1);
}
