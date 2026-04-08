# Changelog

## [0.2.0] - 2026-04-09

### Added
- 通过仓库链接直接添加仓库（支持 HTTPS / SSH / owner/repo 格式）
- 内置首次配置引导，App 内完成 GitHub OAuth App 注册，无需手动编辑配置文件
- GitHub Device Flow 登录（无需回调服务器，无需 Client Secret）
- 版本管理脚本（`pnpm version:patch` / `pnpm version:minor`）

### Fixed
- 修复仓库列表点击无反应的问题（事件处理器从表达式改为函数）
- 修复 `cargo` 命令找不到（`.zshrc` 自动添加 Rust 环境）
- 修复多 binary 导致 `pnpm tauri build` 失败（添加 `default-run`）
- add_repo 后自动持久化 state.json

---

## [0.1.0] - 2026-04-08

### Added
- Tauri 2.x + Vue 3 + TypeScript 项目初始化
- GitHub OAuth 本地回调服务器（Port 7890）
- 仓库管理（列表、克隆、索引）
- gitnexus analyze 知识图谱索引集成
- FSEvents 增量监听（git commit 后自动标记 stale）
- MCP Server（5 个工具：list_repos / search / context / impact / cypher）
- 系统托盘 + Cmd⇧R Spotlight 搜索面板
- Sigma.js 知识图谱 WebGL 可视化
- Claude AI 中文代码摘要（haiku-4-5）
- BM25 + nomic-embed-text 语义向量 RRF 融合搜索
- Impact 影响分析（直接/间接影响展示）
- 状态持久化（state.json 原子写入）
- 安全加固（OAuth token 不再注入 URL，grep 改用固定字符串匹配）
