import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { GitHubUser, Repo, IndexProgress } from '@/types'
import { githubAuthApi, repoApi, indexApi } from '@/services/api'

export const useRepoStore = defineStore('repo', () => {
  const currentUser = ref<GitHubUser | null>(null)
  const githubRepos = ref<Repo[]>([])
  const indexedRepos = ref<Repo[]>([])
  const activeIndexing = ref<Map<string, IndexProgress>>(new Map())
  const isLoadingGithub = ref(false)
  const isAuthenticated = computed(() => currentUser.value !== null)

  const loadCurrentUser = async () => {
    const token = await githubAuthApi.getToken()
    if (token) {
      currentUser.value = await githubAuthApi.getCurrentUser()
    }
  }

  const startOAuth = async () => {
    await githubAuthApi.startOAuth()
    // Rust backend opens browser + starts local callback server
    // Frontend listens for 'oauth-success' event in LoginView
  }

  const logout = async () => {
    await githubAuthApi.clearToken()
    currentUser.value = null
    githubRepos.value = []
  }

  const loadGithubRepos = async () => {
    isLoadingGithub.value = true
    try {
      githubRepos.value = await repoApi.listGitHubRepos()
    } finally {
      isLoadingGithub.value = false
    }
  }

  const loadIndexedRepos = async () => {
    indexedRepos.value = await repoApi.listIndexedRepos()
  }

  const addAndIndexRepo = async (repoFullName: string) => {
    const repo = await repoApi.addRepo(repoFullName)
    indexedRepos.value.push(repo)
    await indexApi.startIndex(repo.id)
    return repo
  }

  const addRepoByUrl = async (url: string) => {
    const repo = await repoApi.addRepoByUrl(url)
    // avoid duplicates
    if (!indexedRepos.value.find(r => r.id === repo.id)) {
      indexedRepos.value.push(repo)
    }
    await indexApi.startIndex(repo.id)
    return repo
  }

  const updateIndexProgress = (progress: IndexProgress) => {
    activeIndexing.value.set(progress.repoId, progress)
    if (progress.percent >= 100) {
      activeIndexing.value.delete(progress.repoId)
      loadIndexedRepos()
    }
  }

  const isIndexing = (repoId: string) => activeIndexing.value.has(repoId)
  const getProgress = (repoId: string) => activeIndexing.value.get(repoId)

  const markStale = (repoId: string) => {
    const repo = indexedRepos.value.find(r => r.id === repoId)
    if (repo && repo.indexStatus === 'indexed') {
      repo.indexStatus = 'stale'
    }
  }

  return {
    currentUser,
    githubRepos,
    indexedRepos,
    activeIndexing,
    isLoadingGithub,
    isAuthenticated,
    loadCurrentUser,
    startOAuth,
    logout,
    loadGithubRepos,
    loadIndexedRepos,
    addAndIndexRepo,
    addRepoByUrl,
    updateIndexProgress,
    isIndexing,
    getProgress,
    markStale,
  }
})
