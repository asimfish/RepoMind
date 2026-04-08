<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRepoStore } from '@/stores/repo'
import { useSettingsStore } from '@/stores/settings'

const repoStore = useRepoStore()
const settingsStore = useSettingsStore()
const githubToken = ref('')
const claudeKey = ref('')
const isSaving = ref(false)

onMounted(() => {
  githubToken.value = settingsStore.settings.githubToken ?? ''
  claudeKey.value = settingsStore.settings.claudeApiKey ?? ''
})

const save = async () => {
  isSaving.value = true
  try {
    await settingsStore.save({
      githubToken: githubToken.value || undefined,
      claudeApiKey: claudeKey.value || undefined,
      mcpEnabled: settingsStore.settings.mcpEnabled,
      autoIndexOnCommit: settingsStore.settings.autoIndexOnCommit,
    })
  } finally {
    isSaving.value = false
  }
}

const logout = async () => {
  await repoStore.logout()
}
</script>

<template>
  <div class="px-6 py-4 max-w-2xl">
    <h1 class="text-xl font-semibold text-[#e6edf3] mb-6">设置</h1>

    <!-- Account -->
    <section class="mb-6 card">
      <h2 class="text-sm font-medium text-[#e6edf3] mb-4">账户</h2>
      <div v-if="repoStore.currentUser" class="flex items-center justify-between">
        <div class="flex items-center gap-3">
          <img :src="repoStore.currentUser.avatarUrl" class="h-10 w-10 rounded-full" />
          <div>
            <p class="text-sm font-medium text-[#e6edf3]">{{ repoStore.currentUser.name ?? repoStore.currentUser.login }}</p>
            <p class="text-xs text-[#8b949e]">@{{ repoStore.currentUser.login }}</p>
          </div>
        </div>
        <button class="btn-secondary text-[#f85149] hover:border-[#f85149]" @click="logout">退出登录</button>
      </div>
    </section>

    <!-- AI Integration -->
    <section class="mb-6 card">
      <h2 class="text-sm font-medium text-[#e6edf3] mb-4">AI 集成</h2>
      <div class="space-y-4">
        <div>
          <label class="mb-1.5 block text-xs text-[#8b949e]">Claude API Key（用于 AI 摘要）</label>
          <input v-model="claudeKey" type="password" class="input" placeholder="sk-ant-..." />
        </div>
      </div>
    </section>

    <!-- Indexing -->
    <section class="mb-6 card">
      <h2 class="text-sm font-medium text-[#e6edf3] mb-4">索引设置</h2>
      <div class="space-y-4">
        <label class="flex items-center justify-between cursor-pointer">
          <div>
            <p class="text-sm text-[#e6edf3]">提交后自动重索引</p>
            <p class="text-xs text-[#8b949e]">检测到 git commit 后自动更新知识图谱</p>
          </div>
          <div
            class="relative h-6 w-11 rounded-full transition-colors"
            :class="settingsStore.settings.autoIndexOnCommit ? 'bg-[#238636]' : 'bg-[#21262d]'"
            @click="settingsStore.settings.autoIndexOnCommit = !settingsStore.settings.autoIndexOnCommit"
          >
            <span
              class="absolute top-0.5 h-5 w-5 rounded-full bg-white shadow transition-transform"
              :class="settingsStore.settings.autoIndexOnCommit ? 'left-5' : 'left-0.5'"
            />
          </div>
        </label>

        <label class="flex items-center justify-between cursor-pointer">
          <div>
            <p class="text-sm text-[#e6edf3]">启用 MCP Server</p>
            <p class="text-xs text-[#8b949e]">为 Claude ​Code / Cursor 提供代码知识工具</p>
          </div>
          <div
            class="relative h-6 w-11 rounded-full transition-colors"
            :class="settingsStore.settings.mcpEnabled ? 'bg-[#238636]' : 'bg-[#21262d]'"
            @click="settingsStore.settings.mcpEnabled = !settingsStore.settings.mcpEnabled"
          >
            <span
              class="absolute top-0.5 h-5 w-5 rounded-full bg-white shadow transition-transform"
              :class="settingsStore.settings.mcpEnabled ? 'left-5' : 'left-0.5'"
            />
          </div>
        </label>
      </div>
    </section>

    <button class="btn-primary" :disabled="isSaving" @click="save">
      {{ isSaving ? '保存中...' : '保存设置' }}
    </button>
  </div>
</template>
