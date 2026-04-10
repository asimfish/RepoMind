<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useSkillStore } from '@/stores/skill'
import type { WorkflowStep, WorkflowTemplate } from '@/types'
import WorkflowChain from '@/components/WorkflowChain.vue'

const skillStore = useSkillStore()
const mining = ref(false)
const collecting = ref(false)
const exportBusy = ref<string | null>(null)

const chainHint = computed(() => {
  const n = skillStore.stats?.totalChains ?? 0
  return `从 ${n} 条调用链记录中提炼模板（先「采集调用链」再「重新挖掘」）`
})

function stepsOf(wf: WorkflowTemplate): WorkflowStep[] {
  const raw = wf.steps
  if (!Array.isArray(raw)) return []
  return raw.map((s, i) => ({
    order: typeof s.order === 'number' ? s.order : i,
    skillName: s.skillName ?? (s as { skill_name?: string }).skill_name ?? '?',
    skillId: s.skillId ?? (s as { skill_id?: string }).skill_id,
    isOptional: Boolean(s.isOptional ?? (s as { is_optional?: boolean }).is_optional),
    avgPosition: Number(s.avgPosition ?? (s as { avg_position?: number }).avg_position ?? i),
    coOccurrenceRatio: Number(
      s.coOccurrenceRatio ?? (s as { co_occurrence_ratio?: number }).co_occurrence_ratio ?? 1,
    ),
  }))
}

const statusLabel: Record<string, string> = {
  discovered: '待确认',
  confirmed: '已确认',
  exported: '已导出',
  dismissed: '已忽略',
}

function normStatus(s: string) {
  return String(s).toLowerCase().replace(/-/g, '_')
}

onMounted(async () => {
  await Promise.all([skillStore.loadStats(), skillStore.loadWorkflows()])
})

async function onMine() {
  mining.value = true
  try {
    await skillStore.mineWorkflows()
    await skillStore.loadStats()
  } finally {
    mining.value = false
  }
}

async function onCollect() {
  collecting.value = true
  try {
    await skillStore.collectInvocations()
    await skillStore.loadStats()
  } finally {
    collecting.value = false
  }
}

async function confirmWf(id: string) {
  await skillStore.updateWorkflowStatus(id, 'confirmed')
  await skillStore.loadStats()
}

async function dismissWf(id: string) {
  await skillStore.updateWorkflowStatus(id, 'dismissed')
  await skillStore.loadStats()
}

async function exportWf(id: string) {
  exportBusy.value = id
  try {
    const path = await skillStore.exportWorkflow(id)
    await skillStore.updateWorkflowStatus(id, 'exported')
    await skillStore.loadStats()
    // eslint-disable-next-line no-alert
    alert(`已导出到：\n${path}`)
  } finally {
    exportBusy.value = null
  }
}
</script>

<template>
  <div class="flex h-full flex-col px-6 py-4">
    <div class="mb-4 flex flex-wrap items-center justify-between gap-3">
      <div>
        <h1 class="text-xl font-semibold text-[#e6edf3]">Workflow 模板</h1>
        <p class="mt-0.5 text-sm text-[#8b949e]">{{ chainHint }}</p>
      </div>
      <div class="flex flex-wrap gap-2">
        <button
          class="btn-secondary text-sm"
          :disabled="collecting"
          @click="onCollect"
        >
          {{ collecting ? '采集中…' : '采集调用链' }}
        </button>
        <button
          class="btn-primary text-sm"
          :disabled="mining"
          @click="onMine"
        >
          {{ mining ? '挖掘中…' : '重新挖掘' }}
        </button>
      </div>
    </div>

    <div class="flex-1 space-y-3 overflow-auto">
      <div
        v-if="skillStore.workflows.length === 0"
        class="flex h-48 flex-col items-center justify-center gap-2 rounded-lg border border-[#30363d] bg-[#161b22] text-[#484f58]"
      >
        <p class="text-sm">暂无 Workflow，请先采集调用链并执行挖掘</p>
      </div>

      <div
        v-for="wf in skillStore.workflows"
        :key="wf.id"
        class="card space-y-3"
      >
        <div class="flex flex-wrap items-start justify-between gap-2">
          <div class="min-w-0 flex-1">
            <h2 class="text-sm font-medium text-[#e6edf3]">{{ wf.name }}</h2>
            <p class="mt-1 text-xs text-[#8b949e]">{{ wf.description }}</p>
          </div>
          <span class="tag flex-shrink-0 bg-[#21262d] text-[#8b949e]">
            {{ statusLabel[normStatus(String(wf.status))] ?? wf.status }}
          </span>
        </div>

        <div class="flex flex-wrap gap-3 text-xs text-[#8b949e]">
          <span>频率 <span class="text-[#e6edf3]">{{ wf.frequency }}</span></span>
          <span
            >置信度
            <span class="text-[#e6edf3]">{{ (wf.confidence * 100).toFixed(0) }}%</span></span
          >
          <span v-if="wf.category">分类 <span class="text-[#e6edf3]">{{ wf.category }}</span></span>
        </div>

        <WorkflowChain :steps="stepsOf(wf)" />

        <div class="flex flex-wrap gap-2 border-t border-[#30363d] pt-3">
          <button
            class="rounded-md border border-[#238636]/50 bg-[#238636]/15 px-3 py-1.5 text-xs font-medium text-[#3fb950] transition-colors hover:bg-[#238636]/25 disabled:opacity-50"
            :disabled="normStatus(String(wf.status)) === 'confirmed'"
            @click="confirmWf(wf.id)"
          >
            确认
          </button>
          <button
            class="rounded-md border border-[#30363d] bg-[#21262d] px-3 py-1.5 text-xs text-[#8b949e] transition-colors hover:border-[#f85149]/40 hover:text-[#f85149] disabled:opacity-50"
            :disabled="normStatus(String(wf.status)) === 'dismissed'"
            @click="dismissWf(wf.id)"
          >
            忽略
          </button>
          <button
            class="rounded-md border border-[#388bfd]/40 bg-[#388bfd]/10 px-3 py-1.5 text-xs font-medium text-[#388bfd] transition-colors hover:bg-[#388bfd]/20 disabled:opacity-50"
            :disabled="exportBusy === wf.id"
            @click="exportWf(wf.id)"
          >
            {{ exportBusy === wf.id ? '导出中…' : '导出 SKILL.md' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
