#!/usr/bin/env node
/**
 * RepoMind Version Bump Script
 * Usage: node scripts/bump-version.js [patch|minor|major]
 * Syncs version across package.json, tauri.conf.json, Cargo.toml
 */

const fs = require('fs')
const path = require('path')

const root = path.resolve(__dirname, '..')
const type = process.argv[2] || 'patch'

// Read current version from package.json
const pkg = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8'))
const [major, minor, patch] = pkg.version.split('.').map(Number)

let next
if (type === 'major') next = `${major + 1}.0.0`
else if (type === 'minor') next = `${major}.${minor + 1}.0`
else next = `${major}.${minor}.${patch + 1}`

console.log(`${pkg.version} → ${next}`)

// package.json
pkg.version = next
fs.writeFileSync(path.join(root, 'package.json'), JSON.stringify(pkg, null, 2) + '\n')

// tauri.conf.json
const tauriConf = path.join(root, 'src-tauri', 'tauri.conf.json')
const tauri = JSON.parse(fs.readFileSync(tauriConf, 'utf8'))
tauri.version = next
fs.writeFileSync(tauriConf, JSON.stringify(tauri, null, 2) + '\n')

// Cargo.toml (replace version line in [package] section)
const cargoPath = path.join(root, 'src-tauri', 'Cargo.toml')
const cargo = fs.readFileSync(cargoPath, 'utf8')
const updated = cargo.replace(/^version = "[\d.]+"$/m, `version = "${next}"`)
fs.writeFileSync(cargoPath, updated)

// Prepend CHANGELOG entry
const changelogPath = path.join(root, 'CHANGELOG.md')
const date = new Date().toISOString().slice(0, 10)
const entry = `## [${next}] - ${date}\n\n### Added\n- \n\n### Fixed\n- \n\n---\n\n`
const existing = fs.existsSync(changelogPath) ? fs.readFileSync(changelogPath, 'utf8') : '# Changelog\n\n'
const header = existing.startsWith('# Changelog') ? existing : '# Changelog\n\n' + existing
const afterHeader = header.replace('# Changelog\n\n', '')
fs.writeFileSync(changelogPath, `# Changelog\n\n${entry}${afterHeader}`)

console.log(`✓ Bumped to ${next} in package.json, tauri.conf.json, Cargo.toml`)
console.log(`✓ CHANGELOG.md updated — fill in the details`)
