# DBX 内网版升级审查文档（v0.5.76 → v0.5.88）

> 状态：已执行（构建中） ｜ 日期：2026-08-19 ｜ 执行人：Sisyphus

---

## 一、现状对比

| 项目 | 本地（internal-build） | 上游（origin/main） |
|---|---|---|
| App 版本 | **v0.5.76**（已部署内网 08-06） | **v0.5.88** |
| 落后量 | — | **747 commits / 12 个版本**（v0.5.77 ~ v0.5.88） |
| 驱动包 | agents-v0.2.76（内网已部署） | **agents-v0.2.87**（45 驱动） |
| 驱动门槛 | min_app_version=0.5.76 | **min_app_version=0.5.88**（全部） |
| JRE | dbx-jre-21-*.tar.zst | dbx-jre-21-*.tar.zst（不变） |

---

## 二、本次 6 文件改动核查结果

| # | 文件 | 上游 v0.5.88 变化 | 内网改动适用性 |
|---|---|---|---|
| 1 | `crates/dbx-core/src/lib.rs` | 新增 10+ 模块（ai_codebuddy/opencode/qoder/cursor/grok_cli、consul、runtime_config、session_credentials 等）；`R2_CDN_BASE` 常量位置微移（107 行） | ✅ 常量 + 2 处测试断言照改 |
| 2 | `src-tauri/tauri.conf.json` | version 0.5.88；NSIS 新增中英繁语言；pubkey 仍是上游（91 4F...） | ✅ 换回内网 pubkey + `-v2` endpoint |
| 3 | `src-tauri/src/commands/update.rs` | **+92 行**：新增 `UpdateDownloadProgressGate` 进度节流（百分比变化才 emit）；**常量位置/名称不变** | ✅ 2 个常量照改，新增代码不动 |
| 4 | `crates/dbx-core/src/update.rs` | **本次无 diff**（与 v0.5.76 相同结构） | ✅ File 4 原样适用（含测试断言） |
| 5 | `.github/workflows/build-windows.yml` | 上游无此文件（内网独有） | ✅ 从 4fc2dd088 恢复；features 仍 `duckdb-sidecar,mq-admin` |
| 6 | `INTERNAL_BUILD.md` / `DEPLOY_V2_DRIVERS.md` / `deploy/nginx-v2.conf` | 上游无（内网独有） | ✅ 从 4fc2dd088 恢复后更新 |

**新变化点（v0.5.88 特有）**：
- `src-tauri/Cargo.toml` 新增 `dynamodb` feature（默认启用），CI `--no-default-features` 跳过 → 不影响内网构建
- DynamoDB 为纯原生驱动（registry 无 agent 条目），不需要部署驱动包

---

## 三、驱动包更新（agents-v0.2.87）

- **45 个驱动**，全部 `min_app_version=0.5.88` → 必须与 app v0.5.88 同步升级（老 app 不兼容）
- **Neo4j 已回归**：native 0.1.45（v0.2.76 时从发布移除的问题已解决，无需实测连接确认内置）
- 新增原生驱动：DuckDB 0.1.9、RocketMQ 0.1.7、ZooKeeper 0.1.25（原生化）
- 驱动版本全线更新（示例）：oracle 0.1.41→0.1.51、dameng 0.1.45→0.1.53、cassandra 0.1.38→0.1.43、kingbase 0.1.40→0.1.49
- JRE：dbx-jre-21-*.tar.zst（6 平台，保持不变）
- 离线包：dbx-agents-offline-*.zip（6 平台）

**部署命令**（联网机器，部署到 `/dbx-drivers-v2/`）：

```bash
mkdir -p dbx-v2/{drivers,jre,downloads}
gh release download agents-v0.2.87 --repo t8y2/dbx --pattern "agent-registry.json" --dir dbx-v2/
gh release download agents-v0.2.87 --repo t8y2/dbx --pattern "dbx-agent-*.tar.zst" --dir dbx-v2/drivers/
gh release download agents-v0.2.87 --repo t8y2/dbx --pattern "dbx-jre-*.tar.zst" --dir dbx-v2/jre/
gh release download agents-v0.2.87 --repo t8y2/dbx --pattern "dbx-agents-offline-*.zip" --dir dbx-v2/downloads/
```

⚠️ 注意：`agents-latest` 移动 tag 仍指向 06-23 的旧版，必须用固定 tag `agents-v0.2.87`。
⚠️ 无需下载 `*-legacy-placeholder.jar`（空占位，native 存在时永不下载）。

---

## 四、执行记录

1. ✅ `git fetch origin` → 本地 main 同步至 dee9d5276（v0.5.88）
2. ✅ `git push fork main`（网络抖动，重试后成功）
3. ✅ `git checkout -B internal-build main`（重建，避免 747 commits rebase 冲突）
4. ✅ 应用 File 1-5 + 恢复内网独有文件
5. ✅ 本地编译验证（CI 同参数 `--no-default-features --features duckdb-sidecar,mq-admin`）：
   - `cargo check -p dbx-core` ✅
   - `cargo check` (src-tauri) ✅
   - dbx-core 相关单测（lib.rs 2 个 + update.rs 1 个）✅
6. ✅ 提交 `a4d4b545d feat: internal build for v0.5.88 (v2 driver path)`
7. ✅ `git push fork internal-build --force`
8. ✅ 触发 CI：https://github.com/wobs1028/dbx/actions/runs/32248395763

---

## 五、待办（CI 出包后）

| # | 任务 | 说明 |
|---|---|---|
| 1 | 部署驱动 | agents-v0.2.87 全部包到 `/dbx-drivers-v2/`（见第三节） |
| 2 | 部署安装包 | CI 产物 `v0.5.88-internal` 的 .msi/.exe/latest.json → `-v2/releases/latest/` **且老路径也放一份**（老用户自动升级） |
| 3 | 回归测试重点 | 原生化驱动：Neo4j（回归）、DuckDB、RocketMQ、ZooKeeper、DynamoDB（新）；老用户常用：MySQL、PostgreSQL、Oracle、达梦、金仓 |
| 4 | 通知内网用户 | 升级 app 后驱动管理里"一键升级"从 `-v2` 拉新驱动 |

---

## 六、风险与已知问题

| # | 风险 | 影响 | 处理 |
|---|---|---|---|
| 1 | 747 commits / 12 版本功能变化 | 内网用户可能遇到新 bug | 小范围灰度一台再全量 |
| 2 | GitHub 网络抖动 | 推送/拉依赖失败 | 重试；git 依赖（tokio-postgres-gaussdb 等 PR ref）手动 fetch 补缓存 |
| 3 | 本地 src-tauri 测试运行报 `STATUS_ENTRYPOINT_NOT_FOUND` | 仅本机 DLL 环境问题 | 以 cargo check + CI 为准 |
| 4 | 老路径驱动/app 兼容 | 双路径隔离，无冲突 | 老路径 `/dbx-drivers/` 保持不动 |
| 5 | `agents-latest` tag 过期（06-23） | 若误用拉旧驱动 | 固定用 `agents-v0.2.87` |

---

*附：审计时间 2026-08-19，数据来源 = 本地 git（HEAD v0.5.88 internal-build）+ 上游 origin/main（v0.5.88）+ GitHub releases（agents-v0.2.87）+ INTERNAL_BUILD.md v2 手册。*
