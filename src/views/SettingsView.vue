<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRepoStore } from '@/stores/repo'
import { useSettingsStore } from '@/stores/settings'
import { settingsApi } from '@/services/api'

const repoStore = useRepoStore()
const settingsStore = useSettingsStore()

const claudeKey = ref('')
const claudeKeyStatus = ref<'idle' | 'checking' | 'valid' | 'invalid'>('idle')
const isSaving = ref(false)
const mcpStatus = ref<{ installed: boolean; path: string; registeredClaude: boolean } | null>(null)

onMounted(async () => {
  claudeKey.value = settingsStore.settings.claudeApiKey ?? ''
  try {
    const raw = await settingsApi.getMcpStatus()
    mcpStatus.value = raw as { installed: boolean; path: string; registeredClaude: boolean }
  } catch {
    mcpStatus.value = { installed: false, path: '', registeredClaude: false }
  }
})

const checkClaudeKey = async () => {
  if (!claudeKey.value.startsWith('sk-ant')) return
  claudeKeyStatus.value = 'checking'
  try {
    const valid = await settingsApi.validateClaudeKey(claudeKey.value)
    claudeKeyStatus.value = valid ? 'valid' : 'invalid'
  } catch {
    claudeKeyStatus.value = 'invalid'
  }
}

const save = async () => {
  isSaving.value = true
  try {
    await settingsStore.save({
      claudeApiKey: claudeKey.value || undefined,
      mcpEnabled: settingsStore.settings.mcpEnabled,
      autoIndexOnCommit: settingsStore.settings.autoIndexOnCommit,
    })
  } finally {
    isSaving.value = false
  }
}

const logout = () => repoStore.logout()

const setupMcp = async () => {
  const { Command } = await import('@tauri-apps/plugin-shell')
  const cmd = Command.create('node', [`${import.meta.env.HOME ?? ''}/Desktop/RepoMind/scripts/setup-mcp.js`])
  await cmd.execute()
  try {
    const raw = await settingsApi.getMcpStatus()
    mcpStatus.value = raw as { installed: boolean; path: string; registeredClaude: boolean }
  } catch { /* ignore */ }
}
</script>

<template>
  <div class="px-6 py-4 max-w-2xl space-y-5">
    <h1 class="text-xl font-semibold text-[#e6edf3]">设置</h1>

    <!-- Account -->
    <section class="card">
      <h2 class="mb-4 text-sm font-medium text-[#e6edf3]">账户</h2>
      <div v-if="repoStore.currentUser" class="flex items-center justify-between">
        <div class="flex items-center gap-3">
          <img :src="repoStore.currentUser.avatarUrl" class="h-10 w-10 rounded-full" />
          <div>
            <p class="text-sm font-medium text-[#e6edf3]">{{ repoStore.currentUser.name ?? repoStore.currentUser.login }}</p>
            <p class="text-xs text-[#8b949e]">@{{ repoStore.currentUser.login }}</p>
          </div>
        </div>
        <button class="btn-secondary text-sm text-[#f85149] hover:border-[#f85149]" @click="logout">退出登录</button>
      </div>
    </section>

    <!-- MCP Server -->
    <section class="card">
      <h2 class="mb-1 text-sm font-medium text-[#e6edf3]">MCP Server</h2>
      <p class="mb-4 text-xs text-[#8b949e]">为 Claude ​Code / Cursor / Codex 提供代码知识工具</p>

      <div v-if="mcpStatus" class="space-y-3">
        <!-- Status indicators -->
        <div class="flex items-center gap-3">
          <div class="flex items-center gap-2">
            <span class="h-2 w-2 rounded-full" :class="mcpStatus.installed ? 'bg-[#3fb950]' : 'bg-[#f85149]'" />
            <span class="text-xs text-[#8b949e]">二进制文件</span>
            <span class="text-xs" :class="mcpStatus.installed ? 'text-[#3fb950]' : 'text-[#f85149]'">
              {{ mcpStatus.installed ? '已构建' : '未构建' }}
            </span>
          </div>
          <div class="flex items-center gap-2">
            <span class="h-2 w-2 rounded-full" :class="mcpStatus.registeredClaude ? 'bg-[#3fb950]' : 'bg-[#8b949e]'" />
            <span class="text-xs text-[#8b949e]">Claude ​Code</span>
            <span class="text-xs" :class="mcpStatus.registeredClaude ? 'text-[#3fb950]' : 'text-[#8b949e]'">
              {{ mcpStatus.registeredClaude ? '已注册' : '未注册' }}
            </span>
          </div>
        </div>

        <div v-if="!mcpStatus.installed" class="rounded-lg bg-[#d29922]/10 border border-[#d29922]/30 p-3 text-xs text-[#d29922]">
          需要先构建 MCP 二进制：<code class="ml-1 font-mono bg-[#0d1117] px-1.5 py-0.5 rounded">pnpm build:mcp</code>
        </div>

        <button
          v-if="mcpStatus.installed"
          class="btn-secondary text-xs"
          @click="setupMcp"
        >
          {{ mcpStatus.registeredClaude ? '重新注册 MCP' : '一键注册到 Claude ​Code / Cursor' }}
        </button>

        <p v-if="mcpStatus.path" class="font-mono text-xs text-[#484f58] truncate">{{ mcpStatus.path }}</p>
      </div>
    </section>

    <!-- Claude AI -->
    <section class="card">
      <h2 class="mb-1 text-sm font-medium text-[#e6edf3]">Claude AI 集成</h2>
      <p class="mb-4 text-xs text-[#8b949e]">用于代码符号自动摘要（本地处理，不上传代码）</p>
      <div class="flex gap-2">
        <div class="relative flex-1">
          <input
            v-model="claudeKey"
            type="password"
            class="input pr-20"
            placeholder="sk-ant-api03-..."
            @blur="checkClaudeKey"
          />
          <span
            v-if="claudeKeyStatus !== 'idle'"
            class="absolute right-3 top-1/2 -translate-y-1/2 text-xs"
            :class="{
              'text-[#3fb950]': claudeKeyStatus === 'valid',
              'text-[#f85149]': claudeKeyStatus === 'invalid',
              'text-[#8b949e]': claudeKeyStatus === 'checking',
            }"
          >
            {{ claudeKeyStatus === 'checking' ? '验证中...' : claudeKeyStatus === 'valid' ? '✓ 有效' : '✗ 无效' }}
          </span>
        </div>
      </div>
      <p class="mt-2 text-xs text-[#484f58]">
        在 <a href="https://console.anthropic.com" class="text-[#388bfd] hover:underline" target="_blank">console.anthropic.com</a> 获取 API Key
      </p>
    </section>

    <!-- Indexing -->
    <section class="card">
      <h2 class="mb-4 text-sm font-medium text-[#e6edf3]">索引设置</h2>
      <div class="space-y-4">
        <label class="flex cursor-pointer items-center justify-between">
          <div>
            <p class="text-sm text-[#e6edf3]">提交后自动重索引</p>
            <p class="text-xs text-[#8b949e]">检测到 git commit 后自动更新知识图谱</p>
          </div>
          <button
            class="relative h-6 w-11 rounded-full transition-colors"
            :class="settingsStore.settings.autoIndexOnCommit ? 'bg-[#238636]' : 'bg-[#21262d]'"
            @click="settingsStore.settings.autoIndexOnCommit = !settingsStore.settings.autoIndexOnCommit"
          >
            <span class="absolute top-0.5 h-5 w-5 rounded-full bg-white shadow transition-all duration-200"
              :class="settingsStore.settings.autoIndexOnCommit ? 'left-5' : 'left-0.5'" />
          </button>
        </label>

        <label class="flex cursor-pointer items-center justify-between">
          <div>
            <p class="text-sm text-[#e6edf3]">启用 MCP Server</p>
            <p class="text-xs text-[#8b949e]">为 AI 编码工具提供代码知识上下文</p>
          </div>
          <button
            class="relative h-6 w-11 rounded-full transition-colors"
            :class="settingsStore.settings.mcpEnabled ? 'bg-[#238636]' : 'bg-[#21262d]'"
            @click="settingsStore.settings.mcpEnabled = !settingsStore.settings.mcpEnabled"
          >
            <span class="absolute top-0.5 h-5 w-5 rounded-full bg-white shadow transition-all duration-200"
              :class="settingsStore.settings.mcpEnabled ? 'left-5' : 'left-0.5'" />
          </button>
        </label>
      </div>
    </section>

    <!-- Storage info -->
    <section class="card">
      <h2 class="mb-2 text-sm font-medium text-[#e6edf3]">存储路径</h2>
      <p class="font-mono text-xs text-[#484f58] break-all">{{ settingsStore.settings.indexStoragePath }}</p>
    </section>

    <button class="btn-primary" :disabled="isSaving" @click="save">
      {{ isSaving ? '保存中...' : '保存设置' }}
    </button>
  </div>
</template>
