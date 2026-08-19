# DBX Internal Build Guide（v2 标准手册）

> **最后更新**：2026-08-19（v0.5.76 → v0.5.88 升级后，跨 12 版本）
> **当前基线**：上游 v0.5.88 ｜ 内网已发布 v0.5.76（本次 v0.5.88 构建中）｜ 驱动 agents-v0.2.87
> **架构**：双路径（老 `/dbx-drivers/` 冻结 + 新 `/dbx-drivers-v2/`）

---

## 0. 架构总览（先读这节）

```
上游 t8y2/dbx (origin) ── git fetch ──► 本地 main（永远跟踪上游）
                                          │
                              git checkout -B internal-build main
                                          │  + 6 文件内网改动
                                          ▼
                                  fork wobs1028/dbx (fork)
                                          │  push --force
                                          ▼
                              GitHub Actions (build-windows.yml)
                                          │  出包 v0.5.76-internal
                                          ▼
                              nginx 双路径分发
   ┌──────────────────────────────┴──────────────────────────────┐
   ▼                                                              ▼
/dbx-drivers/  (老，冻结)                             /dbx-drivers-v2/  (新，主路径)
   老 app v0.5.68- 用户用                              新 app v0.5.76+ 用户用
   旧 .jar 驱动 + tar.gz JRE                           新 .tar.zst 驱动 + tar.zst JRE
```

### 关键机制（决定一切的前提）

1. **驱动下载路径由 `R2_CDN_BASE` 编译期常量决定**（`crates/dbx-core/src/lib.rs:96`）。
   老 app 二进制里是 `http://25.75.3.1/dbx-drivers/`，永远不变；新版编译时改为 `-v2`。
2. **下载顺序**：先试内网 `R2_CDN_BASE`，404/失败再回退 GitHub 官方。
   所以 `-v2` 目录**无需部署所有平台**，缺文件时会自动回退 GitHub（内网无外网则失败）。
3. **驱动格式**：v0.2.58 起从裸 `.jar`/二进制改为 **`.tar.zst` 压缩包** + `agent-registry.json` 清单。
   老 app（无 zstd 依赖）无法解压新格式 → **驱动包与 app 必须同步升级**。
4. **`min_app_version` 字段**：新驱动要求 app ≥ 对应版本。目前是软校验（函数定义存在但安装路径未强制调用），未来可能启用。
5. **registry 中 native 驱动引用的 `*-legacy-placeholder.jar` 是空占位**（size=0）：
   native 存在时**永远不会被下载**，**无需在服务器创建**。

---

## 1. 快速开始（每次版本升级的标准流程）

```bash
# 1. 同步上游
git fetch origin
git checkout main && git merge origin/main      # 快进到最新上游
git push fork main

# 2. 重建 internal-build（强烈推荐，避免 461 commits 级冲突）
git checkout -B internal-build main
# 然后手动重新应用下方「第 2 节」的 6 文件改动

# 3. 本地快速验证（可选但推荐，防白跑 CI）
#    只验证改动的 crate 能编译，用与 CI 相同的 feature：
cd crates && cargo check -p dbx-core --no-default-features --features duckdb-sidecar,mq-admin
cd ../src-tauri && cargo check --no-default-features --features duckdb-sidecar,mq-admin

# 4. 提交并推送
git add -A && git commit -m "feat: internal build for vX.X.XX"
git push fork internal-build --force

# 5. 触发 CI
gh workflow run build-windows.yml --repo wobs1028/dbx --ref internal-build
# 监控：gh run watch --repo wobs1028/dbx
```

---

## 2. 6 个文件改动（每次都要重新应用）

### File 1: `crates/dbx-core/src/lib.rs`

```rust
// 第 96 行：
pub const R2_CDN_BASE: &str = "http://25.75.3.1/dbx-drivers-v2/";
```

⚠️ 同步修改同文件 `#[cfg(test)]` 里的两处断言（第 ~196、~209 行）：
`"http://25.75.3.1/dbx-drivers-v2/..."`（替换 `https://dl.dbxio.com/...`）。

### File 2: `src-tauri/tauri.conf.json`

```json
"updater": {
  "pubkey": "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEJFNkYxMEJCQTA1NTUzNjQKUldSa1UxV2d1eEJ2dnJZWVZ4NlREbWErUGxnR0cwbGhCM1Z2UW5QeXMzcU45UHBUSHdZekFSV0MK",
  "endpoints": ["http://25.75.3.1/dbx-drivers-v2/releases/latest/latest.json"],
  "dangerousInsecureTransportProtocol": true
}
```

- pubkey：内网签名公钥（Key ID `BE6F10BBA0555364`），**不要动**，除非换密钥。
- `createUpdaterArtifacts` 必须 `true`（上游默认已有）。

### File 3: `src-tauri/src/commands/update.rs`

```rust
const OFFICIAL_UPDATE_ENDPOINTS: [&str; 2] = [
    "http://25.75.3.1/dbx-drivers-v2/releases/latest/latest.json",
    "https://github.com/t8y2/dbx/releases/latest/download/latest.json",  // GitHub 保留为 fallback
];
const R2_LATEST_RELEASE_DOWNLOAD_PREFIX: &str = "http://25.75.3.1/dbx-drivers-v2/releases/latest/";
```

### File 4: `crates/dbx-core/src/update.rs`

在常量区（`RELEASE_URL_PREFIX` 后）添加：

```rust
fn internal_release_url_prefix() -> String {
    format!("{}releases/latest/", crate::R2_CDN_BASE)
}

const DEFAULT_R2_CDN_BASE: &str = "https://dl.dbxio.com/";

fn is_default_cdn() -> bool {
    crate::R2_CDN_BASE == DEFAULT_R2_CDN_BASE
}
```

`fetch_latest_release` 中把 GitHub 元数据拉取包上判断：

```rust
if is_default_cdn() {
    if let Ok(github) = fetch_github_release_metadata(&client, &release.version).await {
        release.github = Some(github);
    }
}
```

`build_update_info` 中 release_url fallback：

```rust
.unwrap_or_else(|| {
    if is_default_cdn() {
        format!("{RELEASE_URL_PREFIX}{latest_version}")
    } else {
        internal_release_url_prefix()
    }
});
```

⚠️ 同步修改测试断言（`update_check_candidates_follow_selected_source`，~537/545 行）：
`"http://25.75.3.1/dbx-drivers-v2/releases/latest/latest.json"`。

### File 5: `.github/workflows/build-windows.yml`

内网独有文件（上游没有），每次升级**从上一个 internal-build commit 恢复**，改动两处：
1. 构建参数 feature 名随上游变化（当前 v0.5.76 用 `--features duckdb-sidecar,mq-admin`，`--no-default-features`）
2. latest.json 里所有 `dbx-drivers-v2` 链接保持（版本号是变量，无需改）

### File 6: `INTERNAL_BUILD.md`

本文件。每次升级后更新版本号、日期、pitfalls。

---

## 3. 驱动更新（agents release）

### 3.1 当前驱动基线

- 上游 agents release：**agents-v0.2.87**（每次升级先看 `gh release list`）
- 驱动清单：`agent-registry.json`（唯一入口，app 只认它）
- 格式：`.tar.zst`（native 分平台包 / jar 型平台无关包）
- 当前 registry 共 45 个驱动，全部 `min_app_version=0.5.88`（含 neo4j 原生 0.1.45，v0.2.76 的移除问题已恢复）

### 3.2 下载命令（联网机器）

```bash
mkdir -p dbx-v2/{drivers,jre,downloads}

gh release download agents-latest --repo t8y2/dbx --pattern "agent-registry.json" --dir dbx-v2/
gh release download agents-latest --repo t8y2/dbx --pattern "dbx-agent-*.tar.zst" --dir dbx-v2/drivers/
gh release download agents-latest --repo t8y2/dbx --pattern "dbx-jre-*.tar.zst" --dir dbx-v2/jre/
gh release download agents-latest --repo t8y2/dbx --pattern "dbx-agents-offline-*.zip" --dir dbx-v2/downloads/
```

> ⚠️ `agents-latest` 是移动 tag；若要固定版本用具体 tag（如 `agents-v0.2.76`）。
> ⚠️ 无需下载 `*-legacy-placeholder.jar`（空占位，永远不会被下载，见第 0 节机制 5）。

### 3.3 部署到服务器

```
nginx root（如 /root/nginx_test/html/dbx_files）/dbx-drivers-v2/
├── agents/
│   ├── agent-registry.json
│   ├── drivers/dbx-agent-*.tar.zst
│   └── jre/dbx-jre-*.tar.zst
├── downloads/dbx-agents-offline-*.zip     （可选，离线用户用）
└── releases/latest/                       （CI 产物，见第 4 节）
```

### 3.4 清理策略（双路径下的优势）

- **老路径 `/dbx-drivers/` 永远不动、不清理**（老 app 可能长期存在）。
- 新路径每次升级直接覆盖/新增，无兼容负担。
- 新路径缺某平台文件时 app 自动回退 GitHub（内网无外网则需补全）。

---

## 4. CI 与发布

### 4.1 触发

```bash
gh workflow run build-windows.yml --repo wobs1028/dbx --ref internal-build
# 查看：https://github.com/wobs1028/dbx/actions/workflows/build-windows.yml
```

### 4.2 产物（~40 分钟后）

Release `vX.X.XX-internal` 包含：
- `DBX_X.X.XX_x64-setup.exe`
- `DBX_X.X.XX_x64_en-US.msi`
- `latest.json`

### 4.3 部署

```bash
# 下载产物
gh release download vX.X.XX-internal --repo wobs1028/dbx --pattern "*.msi" --pattern "*.exe" --pattern "latest.json"

# 部署到新路径 releases/latest/
cp * /var/www/html/dbx_files/dbx-drivers-v2/releases/latest/

# ★ 关键：老路径也要放一份（老用户检查更新走老路径 latest.json）
#   老用户升级到新 app 后 → 变新 app → 自动走 -v2 拉新驱动
cp DBX_X.X.XX_*.exe DBX_X.X.XX_*.msi latest.json /var/www/html/dbx_files/dbx-drivers/releases/latest/
```

---

## 5. 版本升级要点速查（本次 v0.5.76→v0.5.88 学到的）

| 关注点 | 说明 |
|---|---|
| 上游 feature 名会变 | v0.5.67 用 `duckdb-bundled`，v0.5.76 改 `duckdb-sidecar`，v0.5.88 新增 `dynamodb`（CI 不用，`--no-default-features` 跳过）；每次确认 `src-tauri/Cargo.toml` 与 `dbx-core/Cargo.toml` |
| 上游会重构代码 | 本次 `commands/update.rs` 加了进度节流（`UpdateDownloadProgressGate`），但常量位置不变；`dbx-core/src/update.rs` **本次无变化**（File 4 原样适用）；每次 diff 核对 |
| 上游 pubkey 会换 | 每次升级必须把 `tauri.conf.json` 的 pubkey 换回内网 `BE6F10BBA0555364` |
| 原生驱动迁移 | 上游持续把 Java agent → 原生 Go/Rust（本次新增：Neo4j 回归原生、DuckDB→0.1.9、RocketMQ/ZooKeeper 原生化） |
| DynamoDB 原生驱动 | v0.5.88 新增 DynamoDB 原生驱动（`dynamodb` feature），registry 无对应 agent 条目（纯内置），CI `--no-default-features` 不影响 |
| 驱动版本全线更新 | agents-v0.2.87：45 驱动，oracle 0.1.51 / dameng 0.1.53 / cassandra 0.1.43 / duckdb 0.1.9 等 |
| 新增/移除驱动 | registry 驱动列表会变化，部署时以 `agent-registry.json` 引用为准 |
| 网络抖动 | GitHub 直连间歇性失败（中国网络环境）；cargo 拉 git 依赖（tokio-postgres-gaussdb 等）失败时，手动 `git fetch origin refs/pull/N/head` 到 `~/.cargo/git/db/` 补缓存后重试 |
| 本地 src-tauri 测试运行 | 本机可能报 `STATUS_ENTRYPOINT_NOT_FOUND`（DLL 环境问题），以 `cargo check` + dbx-core 测试为准，CI（windows-2022）不受影响 |

---

## 6. nginx 配置

老路径与新路径在同一 root 下，**nginx 无需为 v2 加配置**：

```nginx
server {
    listen 80;
    root /path/to/dbx_files;      # 与老路径同一 root
    add_header Accept-Ranges bytes;
    gzip off;
    location / {
        autoindex on;
        try_files $uri =404;
    }
}
```

参考文件：`deploy/nginx-v2.conf`。

---

## 7. 签名密钥（一次性，已配置）

- 私钥 → GitHub Secret `TAURI_SIGNING_PRIVATE_KEY`（raw `key.key` 内容，已 base64，勿二次编码）
- 密码 → `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` = `dbx-internal`
- 公钥 → `tauri.conf.json` `updater.pubkey`（`BE6F10BBA0555364`）
- ⚠️ 换密钥 = 所有已装客户端重装，非必要不换。

---

## 8. 常见 Pitfalls

1. **老 app 无法装新驱动**：老 app 无 zstd，解不了 `.tar.zst`。双路径方案下老用户继续用老路径，不受影响。
2. **placeholder jar 404**：`*-legacy-placeholder.jar` 是空占位，native 存在时不下载，**无需部署**。
3. **编译报 perl/OpenSSL**：`sqlite-sqlcipher` 需要 OpenSSL+Perl，CI 用 `--no-default-features --features duckdb-sidecar,mq-admin` 跳过（内网不需要）。
4. **CI 卡队列**：GitHub Actions 免费额度用尽，稍后再试或手动触发。
5. **rebase 冲突爆炸**：跨多版本升级时直接 `git checkout -B internal-build main` 重建 + 重应用 6 文件，别硬 rebase。
6. **测试断言编译失败**：改了 `R2_CDN_BASE` 后，lib.rs 和 update.rs 里的测试硬编码 URL 必须同步改。
7. **老用户收不到升级**：老路径 `releases/latest/latest.json` 必须更新指向新版，否则老用户永远停在旧版。
8. **驱动装不上但 registry 正常**：确认服务器文件名与 registry URL 完全一致（含版本号），nginx `autoindex on` 时可直接浏览核对。

---

## 9. 仓库状态速查

| 项 | 值 |
|---|---|
| 上游 remote | `origin` = `https://github.com/t8y2/dbx.git` |
| fork remote | `fork` = `https://github.com/wobs1028/dbx.git` |
| 上游分支 | `main`（本地同步到最新） |
| 内网分支 | `internal-build`（6 文件改动，force push） |
| 内网文件服务器 | `http://25.75.3.1/`，root 含 `dbx-drivers/`（老）与 `dbx-drivers-v2/`（新） |
| CI 入口 | `wobs1028/dbx` → Actions → `build-windows.yml` → Run workflow → `internal-build` |
| 当前基线 | 上游 v0.5.88 / agents-v0.2.87 / 内网 v0.5.88 构建中 |
| 当前 internal-build commit | `a4d4b545d feat: internal build for v0.5.88 (v2 driver path)` |

---

## 10. 下次升级的完整步骤（照着做即可）

```bash
# ① 同步上游
git fetch origin && git checkout main && git merge origin/main && git push fork main

# ② 记录新版本号
UPSTREAM_VER=$(git log -1 --format=%s origin/main | grep -oP 'v[\d.]+' || echo "check manually")
echo "Upstream: $UPSTREAM_VER"

# ③ 重建 internal-build 分支
git checkout -B internal-build main

# ④ 应用第 2 节 6 文件改动（重点核对：feature 名、update.rs 结构、pubkey、测试断言）
#    注意：同时恢复 INTERNAL_BUILD.md、DEPLOY_V2_DRIVERS.md、deploy/nginx-v2.conf（上游没有）：
#    git checkout <上一内网commit> -- INTERNAL_BUILD.md DEPLOY_V2_DRIVERS.md deploy/nginx-v2.conf

# ⑤ 本地快速编译验证（用 CI 同参数）
cd crates && cargo check -p dbx-core --no-default-features --features duckdb-sidecar,mq-admin
cd ../src-tauri && cargo check --no-default-features --features duckdb-sidecar,mq-admin

# ⑥ 提交推送
git add -A && git commit -m "feat: internal build for $UPSTREAM_VER" && git push fork internal-build --force

# ⑦ 触发 CI 并等待
gh workflow run build-windows.yml --repo wobs1028/dbx --ref internal-build
gh run watch --repo wobs1028/dbx

# ⑧ 部署驱动（第 3 节）+ 部署安装包（第 4.3 节，含老路径一份）
# ⑨ 更新本文件版本号（第 9 节）+ 驱动下载命令里的 agents tag
# ⑩ 通知内网用户升级
```

---

*本手册为内网构建唯一真相源（Single Source of Truth）。UPGRADE_REVIEW_v0.5.76.md 为 v0.5.76 升级的历史审查记录，UPGRADE_REVIEW_v0.5.88.md 为 v0.5.88 升级审查记录，DEPLOY_V2_DRIVERS.md 为驱动部署的详细清单，均已被本手册覆盖，可作参考保留。*
