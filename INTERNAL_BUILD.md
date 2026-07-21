# DBX 内网版编译指南

## 改动文件清单（共 7 个文件）

| # | 文件 | 改动内容 | 目的 |
|---|---|---|---|
| 1 | `crates/dbx-core/src/lib.rs:75` | `R2_CDN_BASE` → 内网地址 | 驱动/JRE/JDBC 插件下载指向内网 |
| 2 | `src-tauri/tauri.conf.json` | 删除 `updater` 节点，`createUpdaterArtifacts` → `false` | 关闭自动更新 |
| 3 | `src-tauri/Cargo.toml:43` | 注释 `tauri-plugin-updater = "2"` | 不编译更新插件 |
| 4 | `src-tauri/src/lib.rs:883` | 注释 `.plugin(tauri_plugin_updater::...)` | 不注册更新插件 |
| 5 | `src-tauri/capabilities/default.json:30` | 删除 `"updater:default"` | 移除更新权限声明 |
| 6 | `src-tauri/src/commands/update.rs` | 删除 `tauri_plugin_updater` 引用，`download_update`/`install_downloaded_update` 改为桩函数返回错误 | 编译通过 |
| 7 | `.github/workflows/build-windows.yml`（新增） | `pnpm tauri build -- --no-default-features --features duckdb-bundled,mq-admin` | CI 自动编译 |

---

## 文件 1：`crates/dbx-core/src/lib.rs`

**只改一行即可让所有驱动/JRE/JDBC 插件下载全部走内网：**

```rust
// 修改前
pub const R2_CDN_BASE: &str = "https://dl.dbxio.com/";
// 修改后（替换为你的内网文件服务器地址）
pub const R2_CDN_BASE: &str = "http://YOUR_SERVER/dbx-drivers/";
```

DBX 下载逻辑：对每个文件构造 CDN URL 和 GitHub URL 两个候选，优先尝试 CDN，GitHub 作为备用（内网不可达时自动跳过，不影响功能）。

---

## 文件 2：`src-tauri/tauri.conf.json`

两处修改：

1. 删除 `plugins` 下的 `"updater": { ... }` 整个块
2. `"createUpdaterArtifacts": false`

---

## 文件 3：`src-tauri/Cargo.toml`

第 43 行注释掉：

```toml
# tauri-plugin-updater = "2"
```

---

## 文件 4：`src-tauri/src/lib.rs`

第 883 行注释掉：

```rust
// .plugin(tauri_plugin_updater::Builder::new().build())  // disabled for internal build
```

---

## 文件 5：`src-tauri/capabilities/default.json`

删除第 30 行：

```
"updater:default",
```

---

## 文件 6：`src-tauri/src/commands/update.rs`

三处修改：

1. 删除 `use tauri_plugin_updater::{Update, UpdaterExt};` 导入
2. `PendingUpdate` 枚举去掉 `Update` 引用：`Ready { bytes: Vec<u8> }`
3. `download_update` 和 `install_downloaded_update` 改为返回 `Err("Auto-update is disabled in this build.")`

---

## 文件 7：`.github/workflows/build-windows.yml`

```yaml
name: Build Windows MSI
on: workflow_dispatch
jobs:
  build:
    runs-on: windows-2022
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
      - uses: pnpm/action-setup@v4
      - run: pnpm install --no-frozen-lockfile
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: x86_64-pc-windows-msvc
      - run: pnpm tauri build -- --no-default-features --features duckdb-bundled,mq-admin
        env:
          CI: "true"
      - uses: actions/upload-artifact@v4
        with:
          name: dbx-windows-x64
          path: |
            src-tauri/target/release/bundle/msi/*.msi
            src-tauri/target/release/bundle/nsis/*.exe
```

> `--no-default-features --features duckdb-bundled,mq-admin` 跳过 `sqlite-sqlcipher` 特性（该特性依赖 OpenSSL 从源码编译，Windows 上需要完整 Perl，费时且内网用不到加密 SQLite）。

---

## 本地编译命令

```powershell
pnpm install --no-frozen-lockfile
pnpm tauri build -- --no-default-features --features duckdb-bundled,mq-admin
```

---

## 内网文件服务器搭建

`R2_CDN_BASE` 指向的目录结构：

```
http://YOUR_SERVER/dbx-drivers/
├── agents/
│   ├── agent-registry.json          ← 从 GitHub agents-latest release 下载，无需修改
│   ├── drivers/
│   │   └── dbx-agent-*.jar / *.exe  ← 全部 41+ 驱动 JAR + 原生二进制
│   └── jre/
│       └── dbx-jre-*.tar.gz         ← 各平台 JRE 归档
├── releases/
│   └── latest/
│       ├── latest.json              ← 从主版本 release 下载
│       └── dbx-jdbc-plugin-latest.zip
└── downloads/                       ← 离线 ZIP 包（可选，用于手动导入）
    └── dbx-agents-offline-*.zip
```

### 获取文件

```bash
# agent 驱动和 JRE（全部从 agents-latest release 下载）
gh release download agents-latest --repo t8y2/dbx --dir ./agents/
# 归类
mkdir -p agents/drivers agents/jre
mv agents/dbx-agent-*.jar agents/drivers/
mv agents/dbx-agent-*-* agents/drivers/
mv agents/dbx-jre-*.tar.gz agents/jre/

# latest.json 和 JDBC 插件
mkdir -p releases/latest
curl -L -o releases/latest/latest.json "https://github.com/t8y2/dbx/releases/latest/download/latest.json"
curl -L -o releases/latest/dbx-jdbc-plugin-latest.zip "https://github.com/t8y2/dbx/releases/latest/download/dbx-jdbc-plugin-latest.zip"

# 离线包移到 downloads/（可选）
mkdir -p downloads
mv agents/dbx-agents-offline-*.zip downloads/
```

### Nginx 配置

```nginx
server {
    listen 80;
    root /path/to/dbx_files;
    add_header Accept-Ranges bytes;
    gzip off;

    location / {
        autoindex on;    # 调试用，上线可去掉
        try_files $uri =404;
    }
}
```

> DBX 使用 HTTP Range 断点续传下载大文件，`Accept-Ranges bytes` 必须开启。
> DBX 请求头 `Accept-Encoding: identity` 表示不接收压缩响应，`gzip off` 避免 nginx 强行压缩驱动 JAR。

### agent-registry.json 无需修改

DBX 下载逻辑：从 registry JSON 中的 GitHub URL 提取文件名，拼接 `R2_CDN_BASE + 文件名` 作为 CDN URL 优先尝试下载。只要内网服务器上文件名与 GitHub 一致，`agent-registry.json` 不需要任何改动。

---

## 新版本迁移工作流

### 仓库结构

```
t8y2/dbx (上游)
    │
    ├── git fetch upstream
    ↓
wobs1028/dbx
    ├── main          ← 永远追平上游 main
    └── internal-build ← 内网改动（7 个文件），基于 main 之上
```

**不要每次重新 fork。** `main` 分支保持纯净追上游，`internal-build` 分支放你的定制改动。

### 首次设置（只做一次）

在 fork 仓库中添加上游 remote：

```bash
git remote add upstream https://github.com/t8y2/dbx.git
git fetch upstream
```

### 每次新版本迁移步骤

```bash
# === 1. 同步上游最新代码到 main ===
git fetch upstream
git checkout main
git merge upstream/main
git push origin main

# === 2. 将内网改动 rebase 到最新 main ===
git checkout internal-build
git rebase main
# 如果有冲突，解决后 git add + git rebase --continue
git push origin internal-build --force

# === 3. 触发 GitHub Actions 编译 ===
# 浏览器打开 https://github.com/wobs1028/dbx/actions/workflows/build-windows.yml
# 点击 Run workflow → 选择 internal-build 分支

# === 4. 更新内网文件服务器内的驱动文件 ===
# 到文件服务器上执行"获取文件"脚本（见下方）
# === 5. 分发新 .msi ===
```

> **为什么用 rebase**：`internal-build` 分支上只有你的 7 个文件改动，rebase 后历史线干净，很少会遇到冲突——除非新版本恰好改了你动过的同一行。
