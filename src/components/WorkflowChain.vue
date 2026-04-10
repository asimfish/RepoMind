<script setup lang="ts">
import type { WorkflowStep } from '@/types'

defineProps<{
  steps: WorkflowStep[]
}>()
</script>

<template>
  <div class="flex flex-wrap items-center gap-1 overflow-x-auto pb-1">
    <template v-for="(step, i) in steps" :key="`${step.order}-${step.skillName}-${i}`">
      <div
        class="flex max-w-[140px] flex-shrink-0 items-center rounded-md border px-2.5 py-1.5 text-center"
        :class="
          step.isOptional
            ? 'border-dashed border-[#484f58] bg-[#0d1117]/80'
            : 'border-[#30363d] bg-[#21262d]'
        "
      >
        <span class="truncate text-xs font-medium text-[#e6edf3]" :title="step.skillName">
          {{ step.skillName }}
        </span>
      </div>
      <svg
        v-if="i < steps.length - 1"
        class="h-4 w-4 flex-shrink-0 text-[#484f58]"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
      </svg>
    </template>
  </div>
</template>
