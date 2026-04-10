<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useSkillStore } from '@/stores/skill'
import { skillApi } from '@/services/api'
import type { SkillGraphData } from '@/types'
import SkillCard from '@/components/SkillCard.vue'

const skillStore = useSkillStore()
const searchQuery = ref('')
const platformFilter = ref('')
const categoryFilter = ref('')
const activeTab = ref<'list' | 'graph'>('list')
const graphData = ref<SkillGraphData | null>(null)
const graphLoading = ref(false)

const platformOptions = [
  { value: '', label: '全部平台' },
  { value: 'cursor', label: 'Cursor' },
  { value: 'claude', label: 'Claude' },
  { value: 'codex', label: 'Codex' },
]

const categoryOptions = computed(() => {
  const keys = skillStore.stats ? Object.keys(skillStore.stats.byCategory) : []
  return [{ value: '', label: '全部分类' }, ...keys.map(k => ({ value: k, label: k }))]
})

const platformSummary = computed(() => {
  const m = skillStore.stats?.byPlatform ?? {}
  return Object.entries(m)
    .map(([k, v]) => `${k}: ${v}`)
    .join(' · ') || '—'
})

const nodeLabelMap = computed(() => {
  const m = new Map<string, string>()
  for (const n of graphData.value?.nodes ?? []) {
    m.set(n.id, n.label)
  }
  return m
})

async function applyFilters() {
  await skillStore.loadSkills(
    platformFilter.value || undefined,
    categoryFilter.value || undefined,
    searchQuery.value.trim() || undefined,
  )
}

let searchDebounce: ReturnType<typeof setTimeout>
watch(searchQuery, () => {
  clearTimeout(searchDebounce)
  searchDebounce = setTimeout(() => {
    void applyFilters()
  }, 320)
})

watch([platformFilter, categoryFilter], () => {
  void applyFilters()
})

async function loadGraph() {
  graphLoading.value = true
  try {
    graphData.value = await skillApi.getSkillGraph()
  } finally {
    graphLoading.value = false
  }
}

watch(activeTab, tab => {
  if (tab === 'graph' && !graphData.value) void loadGraph()
})

onMounted(async () => {
  await Promise.all([skillStore.loadStats(), skillStore.loadWorkflows(), applyFilters()])
})

async function onRescan() {
  await skillStore.scanSkills()
  await applyFilters()
  if (activeTab.value === 'graph') await loadGraph()
}
</script>

<template>
  <div class="flex h-full flex-col px-6 py-4">
    <div class="mb-5 flex flex-wrap items-start justify-between gap-3">
      <div>
        <h1 class="text-xl font-semibold text-[#e6edf3]">Skills</h1>
        <p class="mt-0.5 text-sm text-[#8b949e]">浏览与管理本地扫描的 Agent Skills</p>
      </div>
      <button
        class="flex items-center gap-1.5 rounded-lg border border-[#30363d] bg-[#21262d] px-3 py-1.5 text-sm text-[#e6edf3] transition-colors hover:border-[#388bfd] hover:text-[#388bfd] disabled:opacity-50"
        :disabled="skillStore.scanning"
        @click="onRescan"
      >
        <svg
          class="h-4 w-4"
          :class="skillStore.scanning ? 'animate-spin' : ''"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
          />
        </svg>
        {{ skillStore.scanning ? '扫描中…' : '重新扫描' }}
      </button>
    </div>

    <!-- Stats -->
    <div class="mb-4 grid gap-3 sm:grid-cols-3">
      <div class="card py-3">
        <p class="text-xs text-[#8b949e]">Skill 总数</p>
        <p class="mt-1 text-2xl font-semibold text-[#e6edf3]">
          {{ skillStore.stats?.totalSkills ?? skillStore.skillCount }}
        </p>
      </div>
      <div class="card py-3">
        <p class="text-xs text-[#8b949e]">平台分布</p>
        <p class="mt-1 text-sm text-[#c9d1d9]">{{ platformSummary }}</p>
      </div>
      <div class="card py-3">
        <p class="text-xs text-[#8b949e]">Workflow 模板</p>
        <p class="mt-1 text-2xl font-semibold text-[#e6edf3]">
          {{ skillStore.stats?.totalWorkflows ?? skillStore.workflowCount }}
        </p>
      </div>
    </div>

    <!-- Filters -->
    <div class="mb-4 flex flex-wrap items-center gap-3">
      <div class="flex rounded-lg border border-[#30363d] bg-[#161b22] p-1">
        <button
          class="rounded-md px-3 py-1.5 text-sm font-medium transition-colors"
          :class="activeTab === 'list' ? 'bg-[#21262d] text-[#e6edf3]' : 'text-[#8b949e] hover:text-[#e6edf3]'"
          @click="activeTab = 'list'"
        >
          列表
        </button>
        <button
          class="rounded-md px-3 py-1.5 text-sm font-medium transition-colors"
          :class="activeTab === 'graph' ? 'bg-[#21262d] text-[#e6edf3]' : 'text-[#8b949e] hover:text-[#e6edf3]'"
          @click="activeTab = 'graph'"
        >
          图谱
        </button>
      </div>
      <input v-model="searchQuery" class="input min-w-[200px] flex-1" placeholder="搜索名称、描述、标签…" />
      <select v-model="platformFilter" class="input max-w-[160px]">
        <option v-for="o in platformOptions" :key="o.value || 'all'" :value="o.value">
          {{ o.label }}
        </option>
      </select>
      <select v-model="categoryFilter" class="input max-w-[180px]">
        <option v-for="o in categoryOptions" :key="o.value || 'all-cat'" :value="o.value">
          {{ o.label }}
        </option>
      </select>
    </div>

    <!-- List tab -->
    <div v-if="activeTab === 'list'" class="flex-1 overflow-auto">
      <div v-if="skillStore.loading" class="flex h-40 items-center justify-center">
        <div class="h-6 w-6 animate-spin rounded-full border-2 border-[#388bfd] border-t-transparent" />
      </div>
      <div
        v-else-if="skillStore.skills.length === 0"
        class="flex h-40 flex-col items-center justify-center gap-2 text-[#484f58]"
      >
        <p class="text-sm">暂无 Skills，点击「重新扫描」从本机目录发现 SKILL.md</p>
      </div>
      <div v-else class="grid grid-cols-1 gap-3 lg:grid-cols-2 xl:grid-cols-3">
        <SkillCard v-for="s in skillStore.skills" :key="s.id" :skill="s" />
      </div>
    </div>

    <!-- Graph tab -->
    <div v-else class="flex-1 overflow-auto">
      <div class="mb-3 flex items-center justify-between">
        <p class="text-xs text-[#8b949e]">基于调用与边关系的概览（节点大小表示 invoke 次数）</p>
        <button class="btn-secondary py-1 text-xs" :disabled="graphLoading" @click="loadGraph">
          {{ graphLoading ? '加载中…' : '刷新图谱' }}
        </button>
      </div>
      <div v-if="graphLoading && !graphData" class="flex h-40 items-center justify-center">
        <div class="h-6 w-6 animate-spin rounded-full border-2 border-[#388bfd] border-t-transparent" />
      </div>
      <div v-else-if="graphData" class="space-y-4">
        <div class="card">
          <p class="mb-3 text-xs font-medium text-[#8b949e]">节点 ({{ graphData.nodes.length }})</p>
          <div class="flex flex-wrap gap-2">
            <div
              v-for="n in graphData.nodes"
              :key="n.id"
              class="rounded-md border border-[#30363d] bg-[#0d1117] px-2.5 py-1.5 text-xs"
            >
              <span class="font-medium text-[#e6edf3]">{{ n.label }}</span>
              <span class="ml-2 text-[#484f58]">{{ n.platform }}</span>
              <span v-if="n.invokeCount" class="ml-2 text-[#388bfd]">{{ n.invokeCount }}</span>
            </div>
          </div>
        </div>
        <div class="card">
          <p class="mb-3 text-xs font-medium text-[#8b949e]">连接 ({{ graphData.edges.length }})</p>
          <ul class="space-y-1.5 text-xs text-[#c9d1d9]">
            <li v-for="(e, i) in graphData.edges" :key="`${e.source}-${e.target}-${i}`" class="font-mono">
              {{ nodeLabelMap.get(e.source) ?? e.source }}
              <span class="text-[#484f58]"> → </span>
              {{ nodeLabelMap.get(e.target) ?? e.target }}
              <span class="text-[#8b949e]"> (×{{ e.weight }})</span>
            </li>
          </ul>
        </div>
      </div>
    </div>
  </div>
</template>
