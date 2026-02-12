#!/usr/bin/env bun

import { Script } from "@codecoder-ai/script/version"
import { $ } from "bun"

if (!Script.preview) {
  await $`gh release edit v${Script.version} --draft=false`
}

await $`bun install`

await $`gh release download --pattern "codecoder-linux-*64.tar.gz" --pattern "codecoder-darwin-*64.zip" -D dist`

await import(`../packages/ccode/script/publish-registries.ts`)
