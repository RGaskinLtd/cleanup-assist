/**
 * Downloads the current release's installers into site/assets so Vercel serves
 * them directly from the site's own domain.
 *
 * Why this exists: the Microsoft Store needs a stable download URL that returns
 * the file itself, not a redirect. GitHub's /releases/latest/download/ path is a
 * 302, so the bytes have to be served by us. Fetching at build time keeps ~16 MB
 * of binaries per release out of git history.
 *
 * site/assets/manifest.json names the release to fetch, so no GitHub API call is
 * needed (and no rate limit applies). The release workflow updates that manifest,
 * and the resulting push triggers a Vercel rebuild.
 *
 * Run from the repo root:  node scripts/fetch-release-assets.mjs
 */

import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";

const REPO = "RGaskinLtd/cleanup-assist";
const ASSETS_DIR = join("site", "assets");
const MANIFEST = join(ASSETS_DIR, "manifest.json");

/** Stable local filenames. These form the public URLs and must never change. */
const STABLE_NAMES = {
  setup: "CleanupAssist-Setup.exe",
  msi: "CleanupAssist.msi",
  portable: "CleanupAssist-Portable.exe",
};

async function main() {
  const manifest = JSON.parse(await readFile(MANIFEST, "utf8"));
  const { tag, assets, sha256 = {} } = manifest;
  if (!tag || !assets) {
    throw new Error("manifest.json must define 'tag' and 'assets'");
  }

  await mkdir(ASSETS_DIR, { recursive: true });
  console.log(`Fetching ${tag} assets from ${REPO}`);

  for (const [key, remoteName] of Object.entries(assets)) {
    const localName = STABLE_NAMES[key];
    if (!localName) {
      console.warn(`  ! no stable name mapped for "${key}", skipping`);
      continue;
    }
    const url =
      `https://github.com/${REPO}/releases/download/${encodeURIComponent(tag)}/` +
      encodeURIComponent(remoteName);

    // redirect: "follow" is the default; GitHub sends us to its asset CDN.
    const res = await fetch(url, { redirect: "follow" });
    if (!res.ok) {
      throw new Error(`${res.status} ${res.statusText} fetching ${url}`);
    }
    const bytes = Buffer.from(await res.arrayBuffer());

    // If the manifest carries a checksum, refuse to ship a mismatched file.
    const digest = createHash("sha256").update(bytes).digest("hex");
    const expected = sha256[key];
    if (expected && expected.toLowerCase() !== digest) {
      throw new Error(
        `checksum mismatch for ${remoteName}\n  expected ${expected}\n  got      ${digest}`
      );
    }

    await writeFile(join(ASSETS_DIR, localName), bytes);
    const mb = (bytes.length / 1024 / 1024).toFixed(2);
    console.log(`  ${localName.padEnd(28)} ${mb.padStart(6)} MB  ${expected ? "sha256 ok" : ""}`);
  }

  console.log("Done.");
}

main().catch((err) => {
  console.error(`\nfetch-release-assets failed: ${err.message}`);
  // Fail the Vercel build rather than deploying a site with missing downloads.
  // Vercel keeps the previous deployment live, so the URLs stay working.
  process.exit(1);
});
