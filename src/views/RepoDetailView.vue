<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useRepoStore } from '@/stores/repo'
import { searchApi, indexApi } from '@/services/api'
import type { SearchResult, ImpactResult, GraphNode, GraphEdge } from '@/types'
import IndexBadge from '@/components/IndexBadge.vue'
import GraphView from '@/components/GraphView.vue'
import { symbolTypeClassMap as typeColors } from '@/constants/colors'

const route = useRoute()
const repoStore = useRepoStore()
const repoId = computed(() => route.params.id as string)
const repo = computed(() => repoStore.indexedRepos.find(r => r.id === repoId.value))

const mainTab = ref<'explore' | 'graph'>('explore')

// Search state
const query = ref('')
const results = ref<SearchResult[]>([])
const isSearching = ref(false)
const activeResult = ref<SearchResult | null>(null)
const detailTab = ref<'code' | 'impact' | 'summary'>('code')

// Impact state
const impact = ref<ImpactResult | null>(null)
const isLoadingImpact = ref(false)

// Summary state
const summary = ref('')
const isLoadingSummary = ref(false)

// Graph state
const graphNodes = ref<GraphNode[]>([])
const graphEdges = ref<GraphEdge[]>([])
const isLoadingGraph = ref(false)
const focusNodeId = ref<string | undefined>()

onMounted(async () => {
  if (mainTab.value === 'graph') loadGraph()
})

watch(mainTab, (tab) => {
  if (tab === 'graph' && !graphNodes.value.length) loadGraph()
})

let searchTimer: ReturnType<typeof setTimeout>
const onInput = () => { clearTimeout(searchTimer); searchTimer = setTimeout(doSearch, 300) }

const doSearch = async () => {
  if (!query.value.trim()) { results.value = []; return }
  isSearching.value = true
  try {
    results.value = await searchApi.search(repoId.value, query.value)
    if (results.value.length) activeResult.value = results.value[0]
  } finally {
    isSearching.value = false
  }
}

const loadImpact = async (symbol: string) => {
  isLoadingImpact.value = true
  try { impact.value = await searchApi.getImpact(repoId.value, symbol) }
  finally { isLoadingImpact.value = false }
}

const loadSummary = async (symbol: string) => {
  isLoadingSummary.value = true
  summary.value = ''
  try { summary.value = await searchApi.getSummary(repoId.value, symbol) }
  catch (e) { summary.value = `无法生成摘要：${e}` }
  finally { isLoadingSummary.value = false }
}

const loadGraph = async () => {
  isLoadingGraph.value = true
  try {
    const data = await searchApi.getGraph(repoId.value, 600)
    graphNodes.value = data.nodes as unknown as GraphNode[]
    graphEdges.value = data.edges as unknown as GraphEdge[]
  } finally {
    isLoadingGraph.value = false
  }
}

const onDetailTab = (tab: 'code' | 'impact' | 'summary') => {
  detailTab.value = tab
  if (!activeResult.value) return
  if (tab === 'impact' && !impact.value) loadImpact(activeResult.value.symbol)
  if (tab === 'summary' && !summary.value) loadSummary(activeResult.value.symbol)
}

const onGraphNodeClick = async (nodeId: string) => {
  // Switch to explore tab and search for the node
  mainTab.value = 'explore'
  focusNodeId.value = nodeId
  query.value = nodeId
  await doSearch()
}

const reindex = () => indexApi.startIndex(repoId.value)
</script>

<template>
  <div class="flex flex-col h-full px-6 py-4">
    <!-- Header -->
    <div class="mb-4 flex items-center justify-between flex-shrink-0">
      <div class="flex items-center gap-3">
        <h1 class="text-xl font-semibold text-[#e6edf3]">{{ repo?.name ?? repoId }}</h1>
        <IndexBadge v-if="repo" :status="repo.indexStatus" />
      </div>
      <div v-if="repo?.indexStatus === 'stale'" class="flex items-center gap-2">
        <span class="text-xs text-[#d29922]">代码已变更</span>
        <button class="btn-primary py-1 text-xs" @click="reindex">重新索引</button>
      </div>
    </div>

    <!-- Main tabs -->
    <div class="mb-4 flex gap-1 flex-shrink-0">
      <button v-for="t in [{ key: 'explore', label: '代码探索' }, { key: 'graph', label: '知识图谱' }]"
        :key="t.key"
        class="rounded-md px-4 py-1.5 text-sm font-medium transition-colors"
        :class="mainTab === t.key ? 'bg-[#21262d] text-[#e6edf3]' : 'text-[#8b949e] hover:text-[#e6edf3]'"
        @click="mainTab = t.key as 'explore' | 'graph'"
      >{{ t.label }}</button>
    </div>

    <!-- ── Explore Tab ── -->
    <div v-if="mainTab === 'explore'" class="flex flex-1 gap-4 overflow-hidden min-h-0">
      <!-- Search sidebar -->
      <div class="flex w-72 flex-shrink-0 flex-col gap-3">
        <div class="relative">
          <svg class="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-[#484f58]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-4.35-4.35M17 11A6 6 0 1 1 5 11a6 6 0 0 1 12 0z" />
          </svg>
          <input v-model="query" class="input pl-10" placeholder="搜索符号..." @input="onInput" @keyup.enter="doSearch" />
          <div v-if="isSearching" class="absolute right-3 top-1/2 -translate-y-1/2">
            <div class="h-4 w-4 animate-spin rounded-full border-2 border-[#388bfd] border-t-transparent" />
          </div>
        </div>

        <div class="flex-1 overflow-auto space-y-1">
          <div v-if="!query" class="py-8 text-center text-sm text-[#484f58]">输入关键词搜索代码</div>
          <div v-else-if="results.length === 0 && !isSearching" class="py-8 text-center text-sm text-[#484f58]">未找到 "{{ query }}"</div>

          <button v-for="r in results" :key="`${r.file}:${r.line}`"
            class="w-full rounded-lg border p-3 text-left transition-colors"
            :class="activeResult === r ? 'border-[#388bfd] bg-[#161b22]' : 'border-[#30363d] bg-[#0d1117] hover:border-[#388bfd]/50'"
            @click="activeResult = r; detailTab = 'code'; impact = null; summary = ''"
          >
            <div class="flex items-center gap-2 mb-1">
              <span class="tag text-xs" :class="typeColors[r.type]">{{ r.type }}</span>
              <span class="text-sm font-medium text-[#e6edf3] truncate">{{ r.symbol }}</span>
            </div>
            <p class="text-xs text-[#8b949e] truncate">{{ r.file }}:{{ r.line }}</p>
          </button>
        </div>
      </div>

      <!-- Detail panel -->
      <div class="flex flex-1 flex-col overflow-hidden rounded-lg border border-[#30363d] bg-[#161b22] min-w-0">
        <div v-if="activeResult" class="flex items-center gap-1 border-b border-[#30363d] px-4 pt-3 flex-shrink-0">
          <button v-for="t in [{ key: 'code', label: '代码' }, { key: 'impact', label: '影响分析' }, { key: 'summary', label: 'AI 摘要' }]"
            :key="t.key"
            class="px-3 py-1.5 text-sm font-medium transition-colors"
            :class="detailTab === t.key ? 'border-b-2 border-[#388bfd] text-[#388bfd]' : 'text-[#8b949e] hover:text-[#e6edf3]'"
            @click="onDetailTab(t.key as 'code' | 'impact' | 'summary')"
          >{{ t.label }}</button>
        </div>

        <!-- Code view -->
        <div v-if="activeResult && detailTab === 'code'" class="flex-1 overflow-auto p-4">
          <div class="mb-3 flex items-center gap-3">
            <span class="tag py-1 px-3" :class="typeColors[activeResult.type]">{{ activeResult.type }}</span>
            <h2 class="text-base font-semibold text-[#e6edf3]">{{ activeResult.symbol }}</h2>
          </div>
          <p class="mb-3 text-xs text-[#8b949e]">{{ activeResult.file }}:{{ activeResult.line }}</p>
          <pre class="overflow-auto rounded-lg bg-[#0d1117] p-4 text-sm text-[#e6edf3] leading-relaxed">{{ activeResult.snippet }}</pre>
        </div>

        <!-- Impact view -->
        <div v-else-if="activeResult && detailTab === 'impact'" class="flex-1 overflow-auto p-4">
          <div v-if="isLoadingImpact" class="flex h-32 items-center justify-center">
            <div class="h-6 w-6 animate-spin rounded-full border-2 border-[#388bfd] border-t-transparent" />
          </div>
          <div v-else-if="impact" class="space-y-4">
            <h3 class="font-semibold text-[#e6edf3]">{{ impact.symbol }} 的影响范围</h3>
            <div v-if="impact.directlyAffected.length">
              <p class="mb-2 text-xs font-medium text-[#f85149]">直接影响 — 必须更新</p>
              <div class="space-y-1">
                <div v-for="n in impact.directlyAffected" :key="n.symbol"
                  class="flex items-center gap-2 rounded bg-[#f85149]/10 px-3 py-2 text-sm">
                  <span class="font-mono text-[#e6edf3]">{{ n.symbol }}</span>
                  <span class="ml-auto text-xs text-[#8b949e] truncate">{{ n.file }}</span>
                  <span class="text-xs text-[#f85149]">{{ Math.round(n.confidence * 100) }}%</span>
                </div>
              </div>
            </div>
            <div v-if="impact.indirectlyAffected.length">
              <p class="mb-2 text-xs font-medium text-[#d29922]">间接影响 — 建议测试</p>
              <div class="space-y-1">
                <div v-for="n in impact.indirectlyAffected" :key="n.symbol"
                  class="flex items-center gap-2 rounded bg-[#d29922]/10 px-3 py-2 text-sm">
                  <span class="font-mono text-[#e6edf3]">{{ n.symbol }}</span>
                  <span class="ml-auto text-xs text-[#8b949e] truncate">{{ n.file }}</span>
                </div>
              </div>
            </div>
            <div v-if="impact.processes.length">
              <p class="mb-2 text-xs font-medium text-[#388bfd]">相关执行流程</p>
              <div class="flex flex-wrap gap-2">
                <span v-for="p in impact.processes" :key="p" class="tag bg-[#388bfd]/10 text-[#388bfd]">{{ p }}</span>
              </div>
            </div>
            <div v-if="!impact.directlyAffected.length && !impact.indirectlyAffected.length"
              class="py-8 text-center text-sm text-[#3fb950]">
              ✓ 该符号无已知依赖，修改风险较低
            </div>
          </div>
        </div>

        <!-- AI Summary view -->
        <div v-else-if="activeResult && detailTab === 'summary'" class="flex-1 overflow-auto p-4">
          <div v-if="isLoadingSummary" class="flex h-32 items-center justify-center">
            <div class="h-6 w-6 animate-spin rounded-full border-2 border-[#388bfd] border-t-transparent" />
          </div>
          <div v-else-if="summary" class="space-y-3">
            <div class="flex items-center gap-2">
              <span class="text-xs font-medium text-[#a371f7]">AI 摘要</span>
              <span class="text-xs text-[#484f58]">claude-haiku-4-5</span>
            </div>
            <p class="text-sm text-[#e6edf3] leading-relaxed">{{ summary }}</p>
          </div>
          <div v-else class="flex h-32 items-center justify-center text-sm text-[#484f58]">
            需要配置 Claude API Key（设置页）才能使用 AI 摘要
          </div>
        </div>

        <div v-if="!activeResult" class="flex flex-1 items-center justify-center text-sm text-[#484f58]">
          选择一个搜索结果查看详情
        </div>
      </div>
    </div>

    <!-- ── Graph Tab ── -->
    <div v-else-if="mainTab === 'graph'" class="flex-1 min-h-0 rounded-lg border border-[#30363d] overflow-hidden">
      <div v-if="isLoadingGraph" class="flex h-full items-center justify-center">
        <div class="flex flex-col items-center gap-3 text-[#8b949e]">
          <div class="h-8 w-8 animate-spin rounded-full border-2 border-[#388bfd] border-t-transparent" />
          <p class="text-sm">构建知识图谱...</p>
        </div>
      </div>
      <GraphView
        v-else
        :nodes="graphNodes"
        :edges="graphEdges"
        :focus-node-id="focusNodeId"
        class="h-full"
        @node-click="onGraphNodeClick"
      />
    </div>
  </div>
</template>
