#!/usr/bin/env node
// Copyright 2026 inno-tauri-utils contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Covector uses this to check whether the exact DLL version has a GitHub
// release already, including a draft left by an earlier successful run.
const https = require("node:https");

const packageName = process.argv[2]?.replaceAll("_", "-");
const version = process.argv[3];
const repository = process.env.GITHUB_REPOSITORY;

if (!packageName || !version || !/^[\w.-]+\/[\w.-]+$/.test(repository ?? "")) {
  console.error("Usage: package-latest-version.js <package> <version>; set GITHUB_REPOSITORY=owner/repo");
  process.exitCode = 1;
} else {
  const tag = `${packageName}-v${version}`;
  const headers = {
    Accept: "application/vnd.github+json",
    "User-Agent": "inno-tauri-utils-covector",
    "X-GitHub-Api-Version": "2022-11-28",
  };
  if (process.env.GITHUB_TOKEN) {
    headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;
  }
  const request = https.get(
    `https://api.github.com/repos/${repository}/releases/tags/${encodeURIComponent(tag)}`,
    { headers },
    (response) => {
      let body = "";
      response.setEncoding("utf8");
      response.on("data", (chunk) => { body += chunk; });
      response.on("error", fail);
      response.on("end", () => {
        if (response.statusCode === 404) {
          console.log("0.0.0");
        } else if (response.statusCode !== 200) {
          fail(new Error(`GitHub release lookup returned HTTP ${response.statusCode}`));
        } else {
          try {
            const release = JSON.parse(body);
            if (release.tag_name !== tag) throw new Error("GitHub returned an unexpected release tag");
            console.log(version);
          } catch (error) {
            fail(error);
          }
        }
      });
    },
  );
  request.setTimeout(30000, () => request.destroy(new Error("GitHub release lookup timed out")));
  request.on("error", fail);
}

function fail(error) {
  console.error(error.message);
  process.exitCode = 1;
}
