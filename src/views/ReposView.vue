<script setup lang="ts">
import { onMounted, ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useRepoStore } from '@/stores/repo'
import type { Repo } from '@/types'
import RepoCard from '@/components/RepoCard.vue'

const router = useRouter()
const repoStore = useRepoStore()
const searchQuery = ref('')
const activeTab = ref<'indexed' | 'github'>('indexed')
const isAdding = ref<string | null>(null)

// URL 添加
const urlInput = ref('')
const showUrlInput = ref(false)
const urlError = ref('')
const isAddingUrl = ref(false)

onMounted(async () => {
  await repoStore.loadIndexedRepos()
})

const onTabChange = async (tab: 'indexed' | 'github') => {
  activeTab.value = tab
  if (tab === 'github' && repoStore.githubRepos.length === 0) {
    await repoStore.loadGithubRepos()
  }
}

const filteredRepos = computed(() => {
  const list = activeTab.value === 'indexed' ? repoStore.indexedRepos : repoStore.githubRepos
  if (!searchQuery.value) return list
  const q = searchQuery.value.toLowerCase()
  return list.filter(r => r.name.toLowerCase().includes(q) || r.fullName.toLowerCase().includes(q))
})

// 修复：用函数而非表达式
const onRepoClick = (repo: Repo) => {
  if (activeTab.value === 'indexed') {
    router.push(`/repos/${repo.id}`)
  }
}

const addRepo = async (repo: Repo) => {
  isAdding.value = repo.id
  try {
    await repoStore.addAndIndexRepo(repo.fullName)
    activeTab.value = 'indexed'
  } finally {
    isAdding.value = null
  }
}

const isIndexed = (repo: Repo) =>
  repoStore.indexedRepos.some(r => r.fullName === repo.fullName)

const addByUrl = async () => {
  const url = urlInput.value.trim()
  if (!url) return
  urlError.value = ''
  isAddingUrl.value = true
  try {
    await repoStore.addRepoByUrl(url)
    urlInput.value = ''
    showUrlInput.value = false
    activeTab.value = 'indexed'
  } catch (e) {
    urlError.value = String(e)
  } finally {
    isAddingUrl.value = false
  }
}

const onUrlKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Enter') addByUrl()
  if (e.key === 'Escape') { showUrlInput.value = false; urlInput.value = '' }
}
</script>

<template>
  <div class="flex flex-col h-full px-6 py-4">
    <!-- Header -->
    <div class="mb-5 flex items-center justify-between">
      <div>
        <h1 class="text-xl font-semibold text-[#e6edf3]">代码仓库</h1>
        <p class="mt-0.5 text-sm text-[#8b949e]">{{ repoStore.indexedRepos.length }} 个仓库已索引</p>
      </div>
      <!-- Add by URL button -->
      <button
        class="flex items-center gap-1.5 rounded-lg border border-[#30363d] bg-[#21262d] px-3 py-1.5 text-sm text-[#e6edf3] hover:border-[#388bfd] hover:text-[#388bfd] transition-colors"
        @click="showUrlInput = !showUrlInput"
      >
        <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
        </svg>
        通过链接添加
      </button>
    </div>

    <!-- URL input panel -->
    <div v-if="showUrlInput" class="mb-4 rounded-lg border border-[#388bfd]/40 bg-[#161b22] p-4">
      <p class="mb-2 text-xs text-[#8b949e]">支持 GitHub 仓库链接、SSH 地址或 owner/repo 格式</p>
      <div class="flex gap-2">
        <input
          v-model="urlInput"
          class="input flex-1 font-mono text-sm"
          placeholder="https://github.com/owner/repo  或  owner/repo"
          autofocus
          @keydown="onUrlKeydown"
        />
        <button
          class="btn-primary flex-shrink-0"
          :disabled="isAddingUrl || !urlInput.trim()"
          @click="addByUrl"
        >
          {{ isAddingUrl ? '添加中...' : '添加并索引' }}
        </button>
        <button class="btn-secondary flex-shrink-0" @click="showUrlInput = false">取消</button>
      </div>
      <p v-if="urlError" class="mt-2 text-xs text-[#f85149]">{{ urlError }}</p>
      <p class="mt-2 text-xs text-[#484f58]">
        例：<span class="font-mono">https://github.com/vercel/next.js</span> ·
        <span class="font-mono">git@github.com:vuejs/vue.git</span> ·
        <span class="font-mono">facebook/react</span>
      </p>
    </div>

    <!-- Tabs + Search -->
    <div class="mb-4 flex items-center gap-3">
      <div class="flex rounded-lg border border-[#30363d] bg-[#161b22] p-1">
        <button
          v-for="tab in [{ key: 'indexed', label: '已索引' }, { key: 'github', label: '我的 GitHub' }]"
          :key="tab.key"
          class="rounded-md px-3 py-1.5 text-sm font-medium transition-colors"
          :class="activeTab === tab.key ? 'bg-[#21262d] text-[#e6edf3]' : 'text-[#8b949e] hover:text-[#e6edf3]'"
          @click="onTabChange(tab.key as 'indexed' | 'github')"
        >
          {{ tab.label }}
        </button>
      </div>
      <input v-model="searchQuery" class="input flex-1" placeholder="搜索仓库..." />
    </div>

    <!-- Repo list -->
    <div class="flex-1 overflow-auto">
      <div v-if="repoStore.isLoadingGithub && activeTab === 'github'"
        class="flex h-40 items-center justify-center">
        <div class="h-6 w-6 animate-spin rounded-full border-2 border-[#388bfd] border-t-transparent" />
      </div>

      <div v-else-if="filteredRepos.length === 0"
        class="flex h-40 flex-col items-center justify-center gap-2 text-[#484f58]">
        <svg class="h-8 w-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 7h18M3 12h18M3 17h18" />
        </svg>
        <p class="text-sm">
          {{ activeTab === 'indexed'
            ? '暂无已索引仓库，通过链接或从 GitHub 仓库添加'
            : '未找到仓库' }}
        </p>
      </div>

      <div v-else class="grid grid-cols-1 gap-3 xl:grid-cols-2">
        <RepoCard
          v-for="repo in filteredRepos"
          :key="repo.id"
          :repo="repo"
          :show-add-button="activeTab === 'github' && !isIndexed(repo)"
          :is-adding="isAdding === repo.id"
          :is-indexing="repoStore.isIndexing(repo.id)"
          :progress="repoStore.getProgress(repo.id)"
          :clickable="activeTab === 'indexed'"
          @add="addRepo(repo)"
          @click="onRepoClick(repo)"
        />
      </div>
    </div>
  </div>
</template>
