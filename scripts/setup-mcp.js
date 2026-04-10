#!/usr/bin/env node
/**
 * RepoMind MCP Setup Script
 * Registers repomind-mcp server with Claude ​Code and Cursor
 *
 * Usage: node scripts/setup-mcp.js
 */

const fs = require('fs')
const path = require('path')
const os = require('os')
const { execSync } = require('child_process')

const HOME = os.homedir()
const REPO_ROOT = path.resolve(__dirname, '..')

// Find repomind-mcp binary
function findBinary() {
  const candidates = [
    path.join(REPO_ROOT, 'src-tauri', 'target', 'release', 'repomind-mcp'),
    path.join(REPO_ROOT, 'src-tauri', 'target', 'debug', 'repomind-mcp'),
    '/usr/local/bin/repomind-mcp',
  ]
  for (const c of candidates) {
    if (fs.existsSync(c)) return c
  }
  return null
}

const bin = findBinary()
if (!bin) {
  console.error('❌  repomind-mcp binary not found. Run: cargo build --release -p repomind-mcp')
  console.error(`   Or build with: cd ${REPO_ROOT} && cargo build --release --manifest-path src-tauri/Cargo.toml --bin repomind-mcp`)
  process.exit(1)
}

console.log(`✓  Found binary: ${bin}`)

const mcpConfig = {
  mcpServers: {
    repomind: {
      command: bin,
      args: [],
      env: {}
    }
  }
}

// ── Claude ​Code ──────────────────────────────────────────────────────────────
const claudeSettings = path.join(HOME, '.claude', 'settings.json')
try {
  let settings = {}
  if (fs.existsSync(claudeSettings)) {
    settings = JSON.parse(fs.readFileSync(claudeSettings, 'utf8'))
  }
  settings.mcpServers = settings.mcpServers || {}
  settings.mcpServers.repomind = mcpConfig.mcpServers.repomind
  fs.mkdirSync(path.dirname(claudeSettings), { recursive: true })
  fs.writeFileSync(claudeSettings, JSON.stringify(settings, null, 2))
  console.log(`✓  Claude ​Code: updated ${claudeSettings}`)
} catch (e) {
  console.warn(`⚠  Claude ​Code setup failed: ${e.message}`)
}

// ── Cursor ────────────────────────────────────────────────────────────────────
const cursorMcp = path.join(HOME, '.cursor', 'mcp.json')
try {
  let existing = {}
  if (fs.existsSync(cursorMcp)) {
    existing = JSON.parse(fs.readFileSync(cursorMcp, 'utf8'))
  }
  existing.mcpServers = existing.mcpServers || {}
  existing.mcpServers.repomind = mcpConfig.mcpServers.repomind
  fs.mkdirSync(path.dirname(cursorMcp), { recursive: true })
  fs.writeFileSync(cursorMcp, JSON.stringify(existing, null, 2))
  console.log(`✓  Cursor: updated ${cursorMcp}`)
} catch (e) {
  console.warn(`⚠  Cursor setup failed: ${e.message}`)
}

// ── Codex ─────────────────────────────────────────────────────────────────────
const codexMcp = path.join(HOME, '.codex', 'mcp.json')
try {
  let existing = {}
  if (fs.existsSync(codexMcp)) {
    existing = JSON.parse(fs.readFileSync(codexMcp, 'utf8'))
  }
  existing.mcpServers = existing.mcpServers || {}
  existing.mcpServers.repomind = mcpConfig.mcpServers.repomind
  fs.mkdirSync(path.dirname(codexMcp), { recursive: true })
  fs.writeFileSync(codexMcp, JSON.stringify(existing, null, 2))
  console.log(`✓  Codex: updated ${codexMcp}`)
} catch (e) {
  console.warn(`⚠  Codex setup failed: ${e.message}`)
}

console.log('\n🎉  RepoMind MCP setup complete!')
console.log('   Restart Claude ​Code / Cursor to activate.')
console.log('\n   Available tools:')
console.log('   • list_repos  — 查看已索引仓库')
console.log('   • search      — 搜索代码符号')
console.log('   • context     — 360° 符号上下文')
console.log('   • impact      — 改动影响分析')
console.log('   • cypher      — 自定义图谱查询')
