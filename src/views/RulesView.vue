<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { rulesApi } from '@/services/api'
import type { BehaviorRule, RuleStats } from '@/types'

const rules = ref<BehaviorRule[]>([])
const stats = ref<RuleStats | null>(null)
const loading = ref(false)
const statusFilter = ref('')
const categoryFilter = ref('')
const scanOpen = ref(false)
const createOpen = ref(false)
const scanPathsText = ref('')
const scanBusy = ref(false)
const createBusy = ref(false)
const actionBusy = ref<string | null>(null)
const pageError = ref('')

const newRule = ref({
  title: '',
  content: '',
  category: '',
  scope: 'global',
  priority: 3,
})

const statusOptions: { value: string; label: string }[] = [
  { value: '', label: '全部状态' },
  { value: 'candidate', label: '待确认' },
  { value: 'approved', label: '已确认' },
  { value: 'rejected', label: '已否决' },
]

const statusLabel = (s: string) => {
  const m: Record<string, string> = {
    candidate: '待确认',
    approved: '已确认',
    rejected: '已否决',
  }
  return m[String(s).toLowerCase()] ?? s
}

const categoryOptions = computed(() => {
  const keys = stats.value ? Object.keys(stats.value.byCategory ?? {}) : []
  return [{ value: '', label: '全部分类' }, ...keys.map(k => ({ value: k, label: k }))]
})

const isCandidate = (r: BehaviorRule) => String(r.status).toLowerCase() === 'candidate'

async function loadStats() {
  try {
    stats.value = await rulesApi.getStats()
  } catch {
    stats.value = null
  }
}

async function loadRules() {
  loading.value = true
  pageError.value = ''
  try {
    rules.value = await rulesApi.listRules(
      statusFilter.value || undefined,
      categoryFilter.value || undefined,
    )
  } catch (e) {
    pageError.value = e instanceof Error ? e.message : String(e)
    rules.value = []
  } finally {
    loading.value = false
  }
}

watch([statusFilter, categoryFilter], () => {
  void loadRules()
})

onMounted(async () => {
  await loadStats()
  await loadRules()
})

function openScan() {
  scanPathsText.value = ''
  scanOpen.value = true
}

async function runScan() {
  const paths = scanPathsText.value
    .split(/\r?\n/)
    .map(s => s.trim())
    .filter(Boolean)
  scanBusy.value = true
  pageError.value = ''
  try {
    await rulesApi.scanSources(paths)
    await loadStats()
    await loadRules()
    scanOpen.value = false
  } catch (e) {
    pageError.value = e instanceof Error ? e.message : String(e)
  } finally {
    scanBusy.value = false
  }
}

function openCreate() {
  newRule.value = { title: '', content: '', category: '', scope: 'global', priority: 3 }
  createOpen.value = true
}

async function submitCreate() {
  if (!newRule.value.title.trim()) return
  createBusy.value = true
  pageError.value = ''
  try {
    await rulesApi.createRule({
      title: newRule.value.title.trim(),
      content: newRule.value.content.trim(),
      category: newRule.value.category.trim() || 'general',
      scope: newRule.value.scope,
      priority: newRule.value.priority,
    })
    await loadStats()
    await loadRules()
    createOpen.value = false
  } catch (e) {
    pageError.value = e instanceof Error ? e.message : String(e)
  } finally {
    createBusy.value = false
  }
}

async function approve(id: string) {
  actionBusy.value = id
  pageError.value = ''
  try {
    await rulesApi.approveRule(id)
    await loadStats()
    await loadRules()
  } catch (e) {
    pageError.value = e instanceof Error ? e.message : String(e)
  } finally {
    actionBusy.value = null
  }
}

async function reject(id: string) {
  actionBusy.value = id
  pageError.value = ''
  try {
    await rulesApi.rejectRule(id)
    await loadStats()
    await loadRules()
  } catch (e) {
    pageError.value = e instanceof Error ? e.message : String(e)
  } finally {
    actionBusy.value = null
  }
}

function starOn(rule: BehaviorRule, indexZero: number) {
  const p = Math.max(0, Math.min(5, Math.round(Number(rule.priority) || 0)))
  return indexZero < p
}

function confidencePct(c: number) {
  const v = Number(c)
  if (Number.isNaN(v)) return 0
  return v <= 1 ? Math.round(v * 100) : Math.min(100, Math.round(v))
}
</script>

<template>
  <div class="flex h-full min-h-0 bg-[#0d1117] text-[#e6edf3]">
    <!-- 左侧筛选 + 统计 -->
    <aside
      class="flex w-64 flex-shrink-0 flex-col border-r border-[#30363d] bg-[#010409] px-4 py-4"
    >
      <h2 class="text-xs font-semibold uppercase tracking-wide text-[#8b949e]">筛选</h2>
      <label class="mt-3 block text-xs text-[#8b949e]">状态</label>
      <select v-model="statusFilter" class="input mt-1 w-full text-sm">
        <option v-for="o in statusOptions" :key="o.value || 'all'" :value="o.value">
          {{ o.label }}
        </option>
      </select>
      <label class="mt-3 block text-xs text-[#8b949e]">类别</label>
      <select v-model="categoryFilter" class="input mt-1 w-full text-sm">
        <option v-for="o in categoryOptions" :key="o.value || 'all-cat'" :value="o.value">
          {{ o.label }}
        </option>
      </select>

      <h2 class="mt-6 text-xs font-semibold uppercase tracking-wide text-[#8b949e]">统计</h2>
      <div class="mt-3 space-y-2 rounded-lg border border-[#30363d] bg-[#161b22] p-3 text-sm">
        <div class="flex justify-between text-[#c9d1d9]">
          <span class="text-[#8b949e]">总计</span>
          <span class="font-medium text-[#e6edf3]">{{ stats?.total ?? '—' }}</span>
        </div>
        <div class="flex justify-between text-[#c9d1d9]">
          <span class="text-[#8b949e]">待确认</span>
          <span class="text-[#d29922]">{{ stats?.candidate ?? '—' }}</span>
        </div>
        <div class="flex justify-between text-[#c9d1d9]">
          <span class="text-[#8b949e]">已确认</span>
          <span class="text-[#3fb950]">{{ stats?.approved ?? '—' }}</span>
        </div>
        <div class="flex justify-between text-[#c9d1d9]">
          <span class="text-[#8b949e]">已否决</span>
          <span class="text-[#f85149]">{{ stats?.rejected ?? '—' }}</span>
        </div>
      </div>
    </aside>

    <!-- 右侧列表 -->
    <div class="flex min-w-0 flex-1 flex-col px-6 py-4">
      <div class="mb-4 flex flex-wrap items-start justify-between gap-3">
        <div>
          <h1 class="text-xl font-semibold text-[#e6edf3]">行为规范</h1>
          <p class="mt-0.5 text-sm text-[#8b949e]">从源文件抽取、审核并维护 Agent 行为约束</p>
        </div>
        <div class="flex flex-wrap gap-2">
          <button type="button" class="btn-secondary text-sm" @click="openScan">扫描规范源</button>
          <button type="button" class="btn-primary text-sm" @click="openCreate">新建规范</button>
        </div>
      </div>

      <p v-if="pageError" class="mb-3 rounded-lg border border-[#f85149]/40 bg-[#f85149]/10 px-3 py-2 text-sm text-[#ffa198]">
        {{ pageError }}
      </p>

      <div class="min-h-0 flex-1 overflow-auto">
        <div v-if="loading" class="flex h-40 items-center justify-center">
          <div class="h-6 w-6 animate-spin rounded-full border-2 border-[#388bfd] border-t-transparent" />
        </div>
        <div
          v-else-if="rules.length === 0"
          class="flex h-48 flex-col items-center justify-center gap-2 rounded-lg border border-[#30363d] bg-[#161b22] text-[#484f58]"
        >
          <p class="text-sm">暂无规范条目，可尝试「扫描规范源」或「新建规范」</p>
        </div>
        <ul v-else class="space-y-3">
          <li
            v-for="r in rules"
            :key="r.id"
            class="card border-[#30363d] bg-[#161b22] transition-colors hover:border-[#388bfd]/50"
          >
            <div class="flex flex-wrap items-start justify-between gap-3">
              <div class="min-w-0 flex-1">
                <div class="flex flex-wrap items-center gap-2">
                  <h3 class="text-sm font-medium text-[#e6edf3]">{{ r.title }}</h3>
                  <span class="rounded border border-[#30363d] bg-[#0d1117] px-2 py-0.5 text-[10px] text-[#8b949e]">
                    {{ statusLabel(r.status) }}
                  </span>
                  <span v-if="r.category" class="tag bg-[#21262d] text-[#8b949e]">{{ r.category }}</span>
                </div>
                <p class="mt-2 line-clamp-2 text-xs leading-relaxed text-[#8b949e]">
                  {{ r.content }}
                </p>
                <div class="mt-2 flex flex-wrap items-center gap-3 text-[10px] text-[#484f58]">
                  <span v-if="r.sourceFile" class="truncate font-mono">来源：{{ r.sourceFile }}</span>
                  <span v-else-if="r.sourceType" class="font-mono">来源类型：{{ r.sourceType }}</span>
                  <span>范围：{{ r.scope }}</span>
                </div>
                <div class="mt-3 flex flex-wrap items-center gap-4">
                  <div class="flex items-center gap-0.5" title="优先级">
                    <svg
                      v-for="i in 5"
                      :key="`${r.id}-star-${i}`"
                      class="h-3.5 w-3.5"
                      :class="starOn(r, i - 1) ? 'text-[#d29922]' : 'text-[#30363d]'"
                      fill="currentColor"
                      viewBox="0 0 20 20"
                    >
                      <path
                        d="M9.049 2.927c.3-.921 1.603-.921 1.902 0l1.07 3.292a1 1 0 00.95.69h3.462c.969 0 1.371 1.24.588 1.81l-2.8 2.034a1 1 0 00-.364 1.118l1.07 3.292c.3.921-.755 1.688-1.54 1.118l-2.8-2.034a1 1 0 00-1.175 0l-2.8 2.034c-.784.57-1.838-.197-1.539-1.118l1.07-3.292a1 1 0 00-.364-1.118L2.98 8.72c-.783-.57-.38-1.81.588-1.81h3.461a1 1 0 00.951-.69l1.07-3.292z"
                      />
                    </svg>
                  </div>
                  <div class="flex min-w-[120px] flex-1 items-center gap-2 sm:max-w-xs">
                    <span class="whitespace-nowrap text-[10px] text-[#8b949e]">置信度</span>
                    <div class="h-1.5 flex-1 overflow-hidden rounded-full bg-[#21262d]">
                      <div
                        class="h-full rounded-full bg-[#388bfd] transition-all"
                        :style="{ width: `${confidencePct(r.confidence)}%` }"
                      />
                    </div>
                    <span class="w-8 text-right text-[10px] tabular-nums text-[#c9d1d9]">
                      {{ confidencePct(r.confidence) }}%
                    </span>
                  </div>
                </div>
              </div>
              <div
                v-if="isCandidate(r)"
                class="flex flex-shrink-0 flex-wrap gap-2"
              >
                <button
                  type="button"
                  class="rounded-lg border border-[#238636] bg-[#238636]/15 px-2.5 py-1 text-xs text-[#3fb950] transition-colors hover:bg-[#238636]/25 disabled:opacity-50"
                  :disabled="actionBusy === r.id"
                  @click="approve(r.id)"
                >
                  ✓ 确认
                </button>
                <button
                  type="button"
                  class="rounded-lg border border-[#f85149]/50 bg-[#f85149]/10 px-2.5 py-1 text-xs text-[#ffa198] transition-colors hover:bg-[#f85149]/20 disabled:opacity-50"
                  :disabled="actionBusy === r.id"
                  @click="reject(r.id)"
                >
                  ✗ 否决
                </button>
              </div>
            </div>
          </li>
        </ul>
      </div>
    </div>

    <!-- 扫描弹层 -->
    <div
      v-if="scanOpen"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
      @click.self="scanOpen = false"
    >
      <div class="w-full max-w-lg rounded-xl border border-[#30363d] bg-[#161b22] p-5 shadow-xl">
        <h3 class="text-sm font-semibold text-[#e6edf3]">扫描规范源</h3>
        <p class="mt-1 text-xs text-[#8b949e]">每行一个目录或文件路径（将传给后端 scan_rule_sources）</p>
        <textarea
          v-model="scanPathsText"
          class="input mt-3 min-h-[140px] w-full resize-y font-mono text-xs"
          placeholder="/path/to/rules&#10;/another/path"
        />
        <div class="mt-4 flex justify-end gap-2">
          <button type="button" class="btn-secondary text-sm" @click="scanOpen = false">取消</button>
          <button
            type="button"
            class="btn-primary text-sm"
            :disabled="scanBusy"
            @click="runScan"
          >
            {{ scanBusy ? '扫描中…' : '开始扫描' }}
          </button>
        </div>
      </div>
    </div>

    <!-- 新建弹层 -->
    <div
      v-if="createOpen"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
      @click.self="createOpen = false"
    >
      <div class="max-h-[90vh] w-full max-w-lg overflow-y-auto rounded-xl border border-[#30363d] bg-[#161b22] p-5 shadow-xl">
        <h3 class="text-sm font-semibold text-[#e6edf3]">新建规范</h3>
        <label class="mt-3 block text-xs text-[#8b949e]">标题</label>
        <input v-model="newRule.title" class="input mt-1 w-full text-sm" placeholder="简短标题" />
        <label class="mt-3 block text-xs text-[#8b949e]">正文</label>
        <textarea
          v-model="newRule.content"
          class="input mt-1 min-h-[100px] w-full resize-y text-sm"
          placeholder="规范内容"
        />
        <div class="mt-3 grid grid-cols-2 gap-3">
          <div>
            <label class="block text-xs text-[#8b949e]">类别</label>
            <input v-model="newRule.category" class="input mt-1 w-full text-sm" placeholder="可选" />
          </div>
          <div>
            <label class="block text-xs text-[#8b949e]">范围</label>
            <input v-model="newRule.scope" class="input mt-1 w-full text-sm" />
          </div>
        </div>
        <label class="mt-3 block text-xs text-[#8b949e]">优先级 (1–5)</label>
        <input
          v-model.number="newRule.priority"
          type="number"
          min="1"
          max="5"
          class="input mt-1 w-full text-sm"
        />
        <div class="mt-4 flex justify-end gap-2">
          <button type="button" class="btn-secondary text-sm" @click="createOpen = false">取消</button>
          <button
            type="button"
            class="btn-primary text-sm"
            :disabled="createBusy || !newRule.title.trim()"
            @click="submitCreate"
          >
            {{ createBusy ? '创建中…' : '创建' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
