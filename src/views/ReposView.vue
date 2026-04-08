<script setup lang="ts">
import { onMounted, ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useRepoStore } from '@/stores/repo'
import type { Repo } from '@/types'
import RepoCard from '@/components/RepoCard.vue'
import IndexBadge from '@/components/IndexBadge.vue'

const router = useRouter()
const repoStore = useRepoStore()
const searchQuery = ref('')
const activeTab = ref<'indexed' | 'github'>('indexed')
const isAdding = ref<string | null>(null)

onMounted(async () => {
  await repoStore.loadIndexedRepos()
  if (activeTab.value === 'github') {
    await repoStore.loadGithubRepos()
  }
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
</script>

<template>
  <div class="flex flex-col h-full px-6 py-4">
    <!-- Header -->
    <div class="mb-6 flex items-center justify-between">
      <div>
        <h1 class="text-xl font-semibold text-[#e6edf3]">代码仓库</h1>
        <p class="mt-0.5 text-sm text-[#8b949e]">
          {{ repoStore.indexedRepos.length }} 个仓库已索引
        </p>
      </div>
    </div>

    <!-- Tabs + Search -->
    <div class="mb-4 flex items-center gap-4">
      <div class="flex rounded-lg border border-[#30363d] bg-[#161b22] p-1">
        <button
          v-for="tab in [{ key: 'indexed', label: '已索引' }, { key: 'github', label: 'GitHub 仓库' }]"
          :key="tab.key"
          class="rounded-md px-3 py-1.5 text-sm font-medium transition-colors"
          :class="activeTab === tab.key
            ? 'bg-[#21262d] text-[#e6edf3]'
            : 'text-[#8b949e] hover:text-[#e6edf3]'"
          @click="onTabChange(tab.key as 'indexed' | 'github')"
        >
          {{ tab.label }}
        </button>
      </div>

      <input
        v-model="searchQuery"
        class="input flex-1"
        placeholder="搜索仓库..."
      />
    </div>

    <!-- Repo list -->
    <div class="flex-1 overflow-auto">
      <!-- Loading state -->
      <div v-if="repoStore.isLoadingGithub && activeTab === 'github'" class="flex h-40 items-center justify-center">
        <div class="h-6 w-6 animate-spin rounded-full border-2 border-[#388bfd] border-t-transparent" />
      </div>

      <!-- Empty state -->
      <div v-else-if="filteredRepos.length === 0" class="flex h-40 flex-col items-center justify-center gap-2 text-[#484f58]">
        <svg class="h-8 w-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 7h18M3 12h18M3 17h18" />
        </svg>
        <p class="text-sm">{{ activeTab === 'indexed' ? '暂无已索引仓库，从 GitHub 仓库选择添加' : '未找到仓库' }}</p>
      </div>

      <!-- Repo cards grid -->
      <div v-else class="grid grid-cols-1 gap-3 xl:grid-cols-2">
        <RepoCard
          v-for="repo in filteredRepos"
          :key="repo.id"
          :repo="repo"
          :show-add-button="activeTab === 'github' && !isIndexed(repo)"
          :is-adding="isAdding === repo.id"
          :is-indexing="repoStore.isIndexing(repo.id)"
          :progress="repoStore.getProgress(repo.id)"
          @add="addRepo(repo)"
          @click="activeTab === 'indexed' && router.push(`/repos/${repo.id}`)"
        />
      </div>
    </div>
  </div>
</template>
