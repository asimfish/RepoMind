// Repository types
export interface Repo {
  id: string
  name: string
  fullName: string        // owner/repo
  description: string | null
  language: string | null
  stars: number
  isPrivate: boolean
  cloneUrl: string
  htmlUrl: string
  updatedAt: string
  localPath?: string      // 本地存储路径（已克隆时）
  indexStatus: IndexStatus
  lastIndexedAt?: string
  lastCommit?: string     // 最后索引时的 commit hash
}

export type IndexStatus = 'not_indexed' | 'indexing' | 'indexed' | 'stale' | 'error'

// GitHub user
export interface GitHubUser {
  login: string
  name: string | null
  avatarUrl: string
  email: string | null
}

// Index progress
export interface IndexProgress {
  repoId: string
  phase: string
  percent: number
  message: string
}

// Search result
export interface SearchResult {
  symbol: string
  file: string
  line: number
  snippet: string
  type: 'function' | 'class' | 'method' | 'variable' | 'interface' | 'enum'
  score: number
  repoName: string
}

// Knowledge graph node
export interface GraphNode {
  id: string
  label: string
  type: 'file' | 'function' | 'class' | 'community' | 'process'
  file?: string
  line?: number
  community?: string
}

// Knowledge graph edge
export interface GraphEdge {
  id: string
  source: string
  target: string
  type: 'calls' | 'imports' | 'extends' | 'implements' | 'member_of'
  confidence: number
}

// Impact analysis
export interface ImpactResult {
  symbol: string
  directlyAffected: ImpactNode[]
  indirectlyAffected: ImpactNode[]
  processes: string[]
}

export interface ImpactNode {
  symbol: string
  file: string
  confidence: number
  depth: number
}

// App settings
export interface AppSettings {
  githubToken?: string
  indexStoragePath: string
  claudeApiKey?: string
  mcpEnabled: boolean
  autoIndexOnCommit: boolean
  searchLanguage: 'en' | 'zh'
}
