import { invoke } from '@tauri-apps/api/core'
import type { GitHubUser, Repo, AppSettings, IndexStatus, SearchResult, GraphNode, GraphEdge, ImpactResult } from '@/types'

export const githubAuthApi = {
  startOAuth: () => invoke<void>('start_github_oauth'),
  getToken: () => invoke<string | null>('get_github_token'),
  clearToken: () => invoke<void>('clear_github_token'),
  getCurrentUser: () => invoke<GitHubUser>('get_github_user'),
}

export const repoApi = {
  listGitHubRepos: (page = 1) => invoke<Repo[]>('list_github_repos', { page }),
  listIndexedRepos: () => invoke<Repo[]>('list_indexed_repos'),
  addRepo: (repoFullName: string) => invoke<Repo>('add_repo', { repoFullName }),
  addRepoByUrl: (url: string) => invoke<Repo>('add_repo_by_url', { url }),
  removeRepo: (repoId: string) => invoke<void>('remove_repo', { repoId }),
  getRepo: (repoId: string) => invoke<Repo>('get_repo', { repoId }),
}

export const indexApi = {
  startIndex: (repoId: string) => invoke<void>('start_index', { repoId }),
  cancelIndex: (repoId: string) => invoke<void>('cancel_index', { repoId }),
  getIndexStatus: (repoId: string) => invoke<IndexStatus>('get_index_status', { repoId }),
}

export const searchApi = {
  search: (repoId: string, query: string) =>
    invoke<SearchResult[]>('search', { repoId, query }),
  getContext: (repoId: string, symbol: string) =>
    invoke<GraphNode[]>('get_context', { repoId, symbol }),
  getImpact: (repoId: string, symbol: string) =>
    invoke<ImpactResult>('get_impact', { repoId, symbol }),
  getGraph: (repoId: string, limit?: number) =>
    invoke<{ nodes: GraphNode[]; edges: GraphEdge[] }>('get_graph', { repoId, limit }),
  getSummary: (repoId: string, symbol: string) =>
    invoke<string>('get_ai_summary', { repoId, symbol }),
}

export const settingsApi = {
  getSettings: () => invoke<AppSettings>('get_settings'),
  updateSettings: (settings: Partial<AppSettings>) =>
    invoke<void>('update_settings', { settings }),
  validateClaudeKey: (apiKey: string) =>
    invoke<boolean>('validate_claude_key', { apiKey }),
  getMcpStatus: () => invoke<{ installed: boolean; path: string }>('get_mcp_status'),
}
