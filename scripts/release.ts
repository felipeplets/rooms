#!/usr/bin/env bun

/**
 * Release automation script for rooms
 *
 * This script:
 * - Detects version bump based on conventional commits
 * - Generates changelog from commits since last tag
 * - Updates Cargo.toml version
 *
 * Usage:
 *   bun scripts/release.ts [--dry-run] [--force-version=X.Y.Z]
 *
 * Output (to $GITHUB_OUTPUT if set):
 *   version - New version number
 *   changelog - Generated changelog markdown
 *   bump_type - major/minor/patch
 */

import { join } from "node:path";
import { readFileSync, writeFileSync, appendFileSync } from "node:fs";
import { spawnSync } from "node:child_process";

const ROOT_DIR = join(import.meta.dir, "..");

// Parse CLI arguments
const args = Bun.argv.slice(2);
const DRY_RUN = args.includes("--dry-run");
const FORCE_VERSION = args
  .find((a) => a.startsWith("--force-version="))
  ?.split("=")[1];

// Types
type BumpType = "major" | "minor" | "patch";

interface CommitTypeInfo {
  bump: BumpType;
  emoji: string;
  title: string;
}

interface RawCommit {
  hash: string;
  subject: string;
  author: string;
}

interface ParsedCommit {
  type: string;
  scope: string | null;
  breaking: boolean;
  description: string;
  pr: string | null;
}

interface ChangelogEntry extends ParsedCommit {
  author: string;
  hash: string;
}

// Conventional commit types and their categories
const COMMIT_TYPES: Record<string, CommitTypeInfo> = {
  feat: { bump: "minor", emoji: "🚀", title: "Features" },
  fix: { bump: "patch", emoji: "🩹", title: "Fixes" },
  docs: { bump: "patch", emoji: "📖", title: "Documentation" },
  style: { bump: "patch", emoji: "💄", title: "Styles" },
  refactor: { bump: "patch", emoji: "♻️", title: "Refactoring" },
  perf: { bump: "patch", emoji: "⚡", title: "Performance" },
  test: { bump: "patch", emoji: "🧪", title: "Tests" },
  build: { bump: "patch", emoji: "📦", title: "Build" },
  ci: { bump: "patch", emoji: "🤖", title: "CI" },
  chore: { bump: "patch", emoji: "🔧", title: "Chores" },
  revert: { bump: "patch", emoji: "⏪", title: "Reverts" },
};

/**
 * Execute a shell command and return stdout
 */
function exec(cmd: string, options: { allowFailure?: boolean } = {}): string {
  const result = spawnSync("sh", ["-c", cmd], {
    cwd: ROOT_DIR,
    encoding: "utf-8",
  });

  if (result.status !== 0) {
    if (options.allowFailure) {
      return "";
    }
    const stderr = result.stderr || "";
    throw new Error(`Command failed: ${cmd}\n${stderr}`);
  }

  return (result.stdout || "").trim();
}

/**
 * Get the latest git tag, or null if none exists
 */
function getLatestTag(): string | null {
  const tag = exec("git describe --tags --abbrev=0 2>/dev/null", {
    allowFailure: true,
  });
  return tag || null;
}

/**
 * Get current version from Cargo.toml
 */
function getCurrentVersion(): string {
  const cargoPath = join(ROOT_DIR, "Cargo.toml");
  const cargoToml = readFileSync(cargoPath, "utf-8");
  const match = cargoToml.match(/^version\s*=\s*"([^"]+)"/m);
  if (!match) {
    throw new Error("Could not find version in Cargo.toml");
  }
  return match[1];
}

/**
 * Get commits since the last tag (or all commits if no tag exists)
 */
function getCommitsSinceTag(tag: string | null): RawCommit[] {
  const range = tag ? `${tag}..HEAD` : "HEAD";
  const format = "%H|%s|%an";
  const log = exec(`git log ${range} --pretty=format:"${format}"`, {
    allowFailure: true,
  });

  if (!log) {
    return [];
  }

  return log.split("\n").map((line) => {
    const [hash, subject, author] = line.split("|");
    return { hash, subject, author };
  });
}

/**
 * Parse a conventional commit message
 * Returns: { type, scope, breaking, description, pr }
 */
function parseConventionalCommit(subject: string): ParsedCommit | null {
  // Pattern: type(scope)!: description (#PR)
  const pattern = /^(\w+)(?:\(([^)]+)\))?(!)?\s*:\s*(.+?)(?:\s*\(#(\d+)\))?$/;
  const match = subject.match(pattern);

  if (!match) {
    return null;
  }

  const [, type, scope, breaking, description, pr] = match;

  // Check for BREAKING CHANGE in the message
  const isBreaking = breaking === "!" || subject.includes("BREAKING CHANGE");

  return {
    type: type.toLowerCase(),
    scope: scope || null,
    breaking: isBreaking,
    description: description.trim(),
    pr: pr || null,
  };
}

/**
 * Determine the version bump type based on commits
 */
function determineBumpType(commits: RawCommit[]): BumpType | null {
  let hasBreaking = false;
  let hasFeature = false;
  let hasFix = false;

  for (const commit of commits) {
    const parsed = parseConventionalCommit(commit.subject);
    if (!parsed) continue;

    if (parsed.breaking) {
      hasBreaking = true;
    }
    if (parsed.type === "feat") {
      hasFeature = true;
    }
    if (COMMIT_TYPES[parsed.type]) {
      hasFix = true;
    }
  }

  if (hasBreaking) return "major";
  if (hasFeature) return "minor";
  if (hasFix) return "patch";
  return null;
}

/**
 * Bump version according to semver
 */
function bumpVersion(version: string, bumpType: BumpType): string {
  const [major, minor, patch] = version.split(".").map(Number);

  switch (bumpType) {
    case "major":
      return `${major + 1}.0.0`;
    case "minor":
      return `${major}.${minor + 1}.0`;
    case "patch":
      return `${major}.${minor}.${patch + 1}`;
    default:
      throw new Error(`Unknown bump type: ${bumpType}`);
  }
}

/**
 * Generate changelog from commits (Nx-style format)
 */
function generateChangelog(
  version: string,
  commits: RawCommit[],
  repoUrl: string
): string {
  const today = new Date().toISOString().split("T")[0];
  const lines: string[] = [`## ${version} (${today})`, ""];

  // Group commits by type
  const groups: Record<string, ChangelogEntry[]> = {
    breaking: [],
  };

  for (const commit of commits) {
    const parsed = parseConventionalCommit(commit.subject);
    if (!parsed) continue;

    const entry: ChangelogEntry = {
      ...parsed,
      author: commit.author,
      hash: commit.hash,
    };

    if (parsed.breaking) {
      groups.breaking.push(entry);
    }

    if (!groups[parsed.type]) {
      groups[parsed.type] = [];
    }
    groups[parsed.type].push(entry);
  }

  // Format a single entry
  const formatEntry = (entry: ChangelogEntry): string => {
    let line = "- ";
    if (entry.scope) {
      line += `**${entry.scope}:** `;
    }
    line += entry.description;
    if (entry.pr) {
      line += ` ([#${entry.pr}](${repoUrl}/pull/${entry.pr}))`;
    }
    return line;
  };

  // Add breaking changes first
  if (groups.breaking.length > 0) {
    lines.push("### 🚨 Breaking Changes");
    lines.push("");
    for (const entry of groups.breaking) {
      lines.push(formatEntry(entry));
    }
    lines.push("");
  }

  // Add other sections in order
  const typeOrder = [
    "feat",
    "fix",
    "perf",
    "refactor",
    "docs",
    "test",
    "build",
    "ci",
    "style",
    "chore",
    "revert",
  ];

  for (const type of typeOrder) {
    const typeInfo = COMMIT_TYPES[type];
    const entries = groups[type];

    if (!entries || entries.length === 0) continue;

    // Filter out breaking changes that were already listed
    const nonBreaking = entries.filter((e) => !e.breaking);
    if (nonBreaking.length === 0) continue;

    lines.push(`### ${typeInfo.emoji} ${typeInfo.title}`);
    lines.push("");
    for (const entry of nonBreaking) {
      lines.push(formatEntry(entry));
    }
    lines.push("");
  }

  return lines.join("\n").trim();
}

/**
 * Update Cargo.toml with new version
 */
function updateCargoToml(newVersion: string): string {
  const cargoPath = join(ROOT_DIR, "Cargo.toml");
  let content = readFileSync(cargoPath, "utf-8");

  content = content.replace(/^(version\s*=\s*)"[^"]+"/m, `$1"${newVersion}"`);

  if (!DRY_RUN) {
    writeFileSync(cargoPath, content);
  }

  return cargoPath;
}

/**
 * Write outputs to $GITHUB_OUTPUT if available
 */
function setOutput(name: string, value: string): void {
  const outputFile = process.env.GITHUB_OUTPUT;
  if (outputFile) {
    // Handle multiline values
    if (value.includes("\n")) {
      const delimiter = `EOF_${Date.now()}`;
      appendFileSync(
        outputFile,
        `${name}<<${delimiter}\n${value}\n${delimiter}\n`
      );
    } else {
      appendFileSync(outputFile, `${name}=${value}\n`);
    }
  }
  console.log(
    `::set-output:: ${name}=${value.split("\n")[0]}${value.includes("\n") ? "..." : ""}`
  );
}

/**
 * Main entry point
 */
async function main(): Promise<void> {
  console.log("🚀 Starting release process...\n");

  // Get repo URL for changelog links
  const repoUrl = "https://github.com/felipeplets/rooms";

  // Get current version and latest tag
  const currentVersion = getCurrentVersion();
  const latestTag = getLatestTag();

  console.log(`📦 Current version: ${currentVersion}`);
  console.log(`🏷️  Latest tag: ${latestTag || "(none)"}`);

  // Get commits since last tag
  const commits = getCommitsSinceTag(latestTag);
  console.log(`📝 Commits since last tag: ${commits.length}`);

  if (commits.length === 0) {
    console.log("\n⚠️  No commits found since last tag. Nothing to release.");
    process.exit(0);
  }

  // Show parsed commits
  console.log("\n📋 Parsed commits:");
  for (const commit of commits) {
    const parsed = parseConventionalCommit(commit.subject);
    if (parsed) {
      const breaking = parsed.breaking ? " [BREAKING]" : "";
      console.log(
        `  - ${parsed.type}${parsed.scope ? `(${parsed.scope})` : ""}: ${parsed.description}${breaking}`
      );
    } else {
      console.log(`  - (non-conventional) ${commit.subject}`);
    }
  }

  // Determine version bump
  let bumpType: BumpType | "forced" | null = determineBumpType(commits);
  let newVersion: string;

  if (FORCE_VERSION) {
    newVersion = FORCE_VERSION;
    bumpType = "forced";
    console.log(`\n🎯 Forced version: ${newVersion}`);
  } else if (bumpType) {
    newVersion = bumpVersion(currentVersion, bumpType);
    console.log(
      `\n📈 Version bump: ${bumpType} (${currentVersion} → ${newVersion})`
    );
  } else {
    console.log(
      "\n⚠️  No conventional commits found. Cannot determine version bump."
    );
    console.log("   Use --force-version=X.Y.Z to specify a version manually.");
    process.exit(1);
  }

  // Generate changelog
  const changelog = generateChangelog(newVersion, commits, repoUrl);
  console.log("\n📜 Generated changelog:");
  console.log("─".repeat(50));
  console.log(changelog);
  console.log("─".repeat(50));

  // Update Cargo.toml
  if (!DRY_RUN) {
    const cargoPath = updateCargoToml(newVersion);
    console.log(`\n✏️  Updated ${cargoPath}`);
  } else {
    console.log("\n🔍 Dry run - Cargo.toml not modified");
  }

  // Set outputs for GitHub Actions
  setOutput("version", newVersion);
  setOutput("changelog", changelog);
  setOutput("bump_type", bumpType);
  setOutput("previous_version", currentVersion);

  console.log("\n✅ Release preparation complete!");
  if (DRY_RUN) {
    console.log("   (Dry run - no files were modified)");
  }
}

main().catch((error: Error) => {
  console.error("❌ Release failed:", error.message);
  process.exit(1);
});
