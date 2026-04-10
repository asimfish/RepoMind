import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { skillApi } from '@/services/api'
import type { Skill, SkillStats, WorkflowTemplate } from '@/types'

export const useSkillStore = defineStore('skill', () => {
  const skills = ref<Skill[]>([])
  const workflows = ref<WorkflowTemplate[]>([])
  const stats = ref<SkillStats | null>(null)
  const loading = ref(false)
  const scanning = ref(false)

  const skillCount = computed(() => skills.value.length)
  const workflowCount = computed(() => workflows.value.length)

  async function loadSkills(platform?: string, category?: string, search?: string) {
    loading.value = true
    try {
      skills.value = await skillApi.listSkills(platform, category, search)
    } finally {
      loading.value = false
    }
  }

  async function scanSkills() {
    scanning.value = true
    try {
      const result = await skillApi.scanSkills()
      await loadSkills()
      await loadStats()
      return result
    } finally {
      scanning.value = false
    }
  }

  async function loadStats() {
    stats.value = await skillApi.getSkillStats()
  }

  async function loadWorkflows(status?: string) {
    workflows.value = await skillApi.listWorkflows(status)
  }

  async function mineWorkflows(minFrequency?: number) {
    const result = await skillApi.mineWorkflows(minFrequency)
    workflows.value = result
    return result
  }

  async function updateWorkflowStatus(id: string, status: string) {
    await skillApi.updateWorkflowStatus(id, status)
    await loadWorkflows()
  }

  async function exportWorkflow(id: string) {
    return await skillApi.exportWorkflow(id)
  }

  async function collectInvocations() {
    return await skillApi.collectInvocations()
  }

  return {
    skills,
    workflows,
    stats,
    loading,
    scanning,
    skillCount,
    workflowCount,
    loadSkills,
    scanSkills,
    loadStats,
    loadWorkflows,
    mineWorkflows,
    updateWorkflowStatus,
    exportWorkflow,
    collectInvocations,
  }
})
