import { spawnSync } from 'node:child_process'
import { readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'

const root = resolve(import.meta.dir, '..')
const version = process.argv[2]
const dryRun = process.argv.includes('--dry-run')
const semver = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/

function fail(message: string): never {
  console.error(`release: ${message}`)
  process.exit(1)
}

function git(args: string[], allowFailure = false): string {
  const result = spawnSync('git', args, { cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] })
  if (result.status !== 0 && !allowFailure) {
    if (result.stderr) process.stderr.write(result.stderr)
    fail(`git ${args.join(' ')} failed`)
  }
  return result.stdout.trim()
}

function run(command: string, args: string[]): void {
  const result = spawnSync(command, args, { cwd: root, stdio: 'inherit' })
  if (result.status !== 0) fail(`${command} ${args.join(' ')} failed; fix the issue and rerun the release command`)
}

if (!version || !semver.test(version)) fail('usage: bun run release -- X.Y.Z [--dry-run]')
const tag = `v${version}`
if (git(['branch', '--show-current']) !== 'main') fail('switch to main before releasing')
if (git(['status', '--porcelain'])) fail('commit or discard all working-tree changes before releasing')
if (!git(['remote', 'get-url', 'origin'])) fail('git remote origin is not configured')

git(['fetch', 'origin', 'main', '--tags'])
if (spawnSync('git', ['merge-base', '--is-ancestor', 'origin/main', 'HEAD'], { cwd: root, stdio: 'ignore' }).status !== 0) {
  fail('update local main from origin/main before releasing')
}
if (git(['tag', '--list', tag]) || git(['ls-remote', '--tags', 'origin', `refs/tags/${tag}`])) fail(`${tag} already exists`)

const packagePath = resolve(root, 'package.json')
const tauriPath = resolve(root, 'src-rs/tauri.conf.json')
const cargoPath = resolve(root, 'src-rs/Cargo.toml')
const lockPath = resolve(root, 'src-rs/Cargo.lock')
const changelogPath = resolve(root, 'CHANGELOG.md')
const packageJson = JSON.parse(readFileSync(packagePath, 'utf8')) as Record<string, unknown>
const tauriJson = JSON.parse(readFileSync(tauriPath, 'utf8')) as Record<string, unknown>
const cargoToml = readFileSync(cargoPath, 'utf8')
const cargoLock = readFileSync(lockPath, 'utf8')
const lockPattern = /(\[\[package\]\]\nname = "d2-assistant"\nversion = ")([^"]+)(")/
const lockMatch = lockPattern.exec(cargoLock)
const versions = [packageJson.version, tauriJson.version, /^version = "([^"]+)"/m.exec(cargoToml)?.[1], lockMatch?.[2]]
if (versions.some((v) => typeof v !== 'string' || v !== versions[0])) fail(`version files disagree: ${versions.join(', ')}`)
const current = String(versions[0])
if (version.localeCompare(current, undefined, { numeric: true }) <= 0) fail(`version must be greater than ${current}`)

const changelog = readFileSync(changelogPath, 'utf8')
const start = changelog.indexOf('## Unreleased')
if (start < 0) fail('CHANGELOG.md is missing ## Unreleased')
const bodyStart = changelog.indexOf('\n', start) + 1
const nextHeader = changelog.indexOf('\n## ', bodyStart)
const bodyEnd = nextHeader < 0 ? changelog.length : nextHeader + 1
const notes = changelog.slice(bodyStart, bodyEnd).trim()
if (!notes || !notes.includes('- ')) fail('add release notes under ## Unreleased in CHANGELOG.md')
const date = new Intl.DateTimeFormat('en-CA', { timeZone: 'America/Mexico_City' }).format(new Date())
const nextChangelog = `${changelog.slice(0, bodyStart)}\n## ${version} — ${date}\n\n${notes}\n\n${changelog.slice(bodyEnd)}`

if (dryRun) {
  console.log(`Preflight passed. Would bump ${current} to ${version}, run quality checks, commit, tag ${tag}, and push main plus the tag.`)
  process.exit(0)
}

packageJson.version = version
tauriJson.version = version
writeFileSync(packagePath, `${JSON.stringify(packageJson, null, 2)}\n`)
writeFileSync(tauriPath, `${JSON.stringify(tauriJson, null, 2)}\n`)
writeFileSync(cargoPath, cargoToml.replace(/^version = "[^"]+"/m, `version = "${version}"`))
writeFileSync(lockPath, cargoLock.replace(lockPattern, `$1${version}$3`))
writeFileSync(changelogPath, nextChangelog)

try {
  run('bun', ['run', 'build'])
  run('bun', ['run', 'test:frontend'])
  run('cargo', ['fmt', '--manifest-path', 'src-rs/Cargo.toml', '--', '--check'])
  run('bun', ['run', 'lint:rust'])
  run('bun', ['run', 'metrics:duplication'])
  run('bun', ['run', 'metrics:complexity'])
  run('bun', ['run', 'test:rust'])
} catch (error) {
  console.error(`Version and changelog edits remain in the working tree for review. ${String(error)}`)
  process.exit(1)
}

git(['add', 'package.json', 'src-rs/Cargo.toml', 'src-rs/Cargo.lock', 'src-rs/tauri.conf.json', 'CHANGELOG.md'])
git(['commit', '-m', `release ${tag}`])
run('git', ['tag', '-a', tag, '-m', `release ${tag}`])
run('git', ['push', '--atomic', 'origin', 'HEAD:main', `refs/tags/${tag}`])
console.log(`Pushed ${tag}; GitHub Actions will build and publish the desktop installers after its release quality gate passes.`)
