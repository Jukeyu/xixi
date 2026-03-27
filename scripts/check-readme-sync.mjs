#!/usr/bin/env node

import { execSync } from 'node:child_process'

const baseRef = process.argv[2] ?? 'HEAD~1'
const headRef = process.argv[3] ?? 'HEAD'

const ZERO_SHA = '0000000000000000000000000000000000000000'

function run(cmd) {
  return execSync(cmd, { encoding: 'utf8' }).trim()
}

function getChangedFiles(base, head) {
  if (!base || base === ZERO_SHA) {
    return []
  }
  const output = run(`git diff --name-only ${base} ${head}`)
  if (!output) {
    return []
  }
  return output
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
}

const changedFiles = getChangedFiles(baseRef, headRef)

if (changedFiles.length === 0) {
  console.log(
    '[readme-sync] No diff detected for this range. Skipping README consistency gate.'
  )
  process.exit(0)
}

const readmeTouched = changedFiles.includes('README.md')

const productChangePrefixes = [
  'apps/desktop/src/',
  'apps/desktop/src-tauri/src/',
  'docs/skills/',
]

const productChanged = changedFiles.some((file) =>
  productChangePrefixes.some((prefix) => file.startsWith(prefix))
)

if (!productChanged) {
  console.log(
    '[readme-sync] No product-facing desktop changes detected. README update not required.'
  )
  process.exit(0)
}

if (readmeTouched) {
  console.log(
    '[readme-sync] README.md updated together with product changes. Check passed.'
  )
  process.exit(0)
}

console.error('[readme-sync] Product-facing changes detected, but README.md was not updated.')
console.error('[readme-sync] Changed files:')
for (const file of changedFiles) {
  console.error(`- ${file}`)
}
console.error(
  '[readme-sync] Please update README intro/status sections in the same commit to keep GitHub page aligned.'
)
process.exit(1)
