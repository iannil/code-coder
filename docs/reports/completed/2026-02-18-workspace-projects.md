# Workspace Projects 功能实现

**日期**: 2026-02-18
**状态**: 已完成

## 概述

为 Web UI 的 Workspace 添加了 Projects 管理功能，使用户可以：
- 管理多个项目
- 在创建 Session 时选择项目、Agent 和 Model

## 实现内容

### Phase 1: 后端 API 扩展

#### 1.1 目录浏览 API
- **新建**: `packages/ccode/src/api/server/handlers/directory.ts`
- 实现 `GET /api/directories?path=` 端点
- 支持列出指定目录下的子目录
- 自动展开 `~` 为用户主目录

#### 1.2 项目 API
- **新建**: `packages/ccode/src/api/server/handlers/project.ts`
- 实现以下端点：
  - `GET /api/projects` - 列出所有项目
  - `GET /api/projects/:id` - 获取项目详情
  - `POST /api/projects` - 创建新项目
  - `PATCH /api/projects/:id` - 更新项目
  - `DELETE /api/projects/:id` - 删除项目
  - `GET /api/projects/:id/sessions` - 获取项目的所有会话

#### 1.3 路由注册
- **修改**: `packages/ccode/src/api/server/router.ts`
- 添加目录和项目相关路由

#### 1.4 Session 创建 API 增强
- **修改**: `packages/ccode/src/api/server/handlers/session.ts`
- 扩展 `createSession` 支持新参数：`projectID`, `directory`, `agent`, `model`

### Phase 2: 前端状态管理

#### 2.1 类型定义
- **修改**: `packages/web/src/lib/types.ts`
- 添加类型：
  - `ProjectInfo` - 项目信息
  - `ProjectCreateInput` - 创建项目输入
  - `DirectoryEntry` - 目录条目
  - `DirectoryListResponse` - 目录列表响应
- 扩展 `SessionCreateInput` 添加 `projectID`, `directory`, `agent`, `model` 字段

#### 2.2 API 客户端
- **修改**: `packages/web/src/lib/api.ts`
- 添加方法：
  - `listDirectories()` - 列出目录
  - `listProjects()` - 列出项目
  - `getProject()` - 获取项目
  - `createProject()` - 创建项目
  - `updateProject()` - 更新项目
  - `deleteProject()` - 删除项目
  - `getProjectSessions()` - 获取项目会话

#### 2.3 项目 Store
- **新建**: `packages/web/src/stores/project.ts`
- 使用 Zustand + Immer 实现
- 提供 hooks：
  - `useProjects()` - 获取所有项目
  - `useProject(id)` - 获取单个项目
  - `useSelectedProject()` - 获取选中的项目
  - `useProjectsLoading()` - 加载状态
  - `useProjectStore()` - 完整 store

#### 2.4 Store 导出
- **修改**: `packages/web/src/stores/index.ts`
- 添加 project store 导出

### Phase 3: 前端 UI 组件

#### 3.1 目录选择器
- **新建**: `packages/web/src/components/shared/DirectoryPicker.tsx`
- 功能：
  - 显示面包屑导航
  - 列出子目录
  - 支持选择当前目录或子目录
  - 支持返回上级目录

#### 3.2 项目页面
- **新建**: `packages/web/src/pages/Projects.tsx`
- 功能：
  - 项目列表（卡片式展示）
  - 新建项目对话框
  - 使用 DirectoryPicker 选择目录
  - 删除项目确认
  - 导航到项目会话

#### 3.3 路由配置
- **修改**: `packages/web/src/router.ts`
- 添加 `/projects` 路由

#### 3.4 导航更新
- **修改**: `packages/web/src/App.tsx`
- 在 Workspace 分组添加 Projects 导航项

#### 3.5 Session 创建增强
- **修改**: `packages/web/src/components/session/SessionCreate.tsx`
- 添加项目选择器
- 添加模型选择器
- 改进 Agent 选择器

#### 3.6 Session Store 类型更新
- **修改**: `packages/web/src/stores/session.ts`
- 扩展 `createSession` 方法参数类型

## 验证

### 类型检查
```bash
cd packages/web && bun run tsc --noEmit
# 无错误输出
```

### 后端 API 测试
```bash
# 测试目录浏览
curl "http://localhost:4400/api/directories?path=/Users"

# 测试项目列表
curl http://localhost:4400/api/projects

# 创建项目
curl -X POST http://localhost:4400/api/projects \
  -H "Content-Type: application/json" \
  -d '{"directory":"/path/to/project","name":"My Project"}'
```

### 前端测试
1. 访问 `/projects` 页面
2. 点击 "New Project" 按钮
3. 使用目录选择器选择项目目录
4. 创建项目后在列表中查看
5. 创建 Session 时测试项目/Agent/Model 选择器

## 文件清单

| 操作 | 文件路径 |
|------|----------|
| 新建 | `packages/ccode/src/api/server/handlers/directory.ts` |
| 新建 | `packages/ccode/src/api/server/handlers/project.ts` |
| 修改 | `packages/ccode/src/api/server/router.ts` |
| 修改 | `packages/ccode/src/api/server/handlers/session.ts` |
| 新建 | `packages/web/src/stores/project.ts` |
| 修改 | `packages/web/src/stores/index.ts` |
| 修改 | `packages/web/src/stores/session.ts` |
| 修改 | `packages/web/src/lib/types.ts` |
| 修改 | `packages/web/src/lib/api.ts` |
| 新建 | `packages/web/src/components/shared/DirectoryPicker.tsx` |
| 新建 | `packages/web/src/pages/Projects.tsx` |
| 修改 | `packages/web/src/components/session/SessionCreate.tsx` |
| 修改 | `packages/web/src/App.tsx` |
| 修改 | `packages/web/src/router.ts` |

## 后续优化建议

1. **按项目分组显示 Sessions** - 在 Sessions 页面添加项目筛选
2. **项目图标上传** - 支持自定义项目图标
3. **项目设置** - 添加项目详情编辑页面
4. **搜索功能** - 在项目列表添加搜索
