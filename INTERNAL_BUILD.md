# DBX 内网版编译指南

## 改动文件清单（共 7 个文件）

| # | 文件 | 改动内容 | 目的 |
|---|---|---|---|
| 1 | `crates/dbx-core/src/lib.rs:75` | `R2_CDN_BASE` → 内网地址 | 驱动/JRE/JDBC 插件下载指向内网 |
| 2 | `src-tauri/tauri.conf.json` | 填入新 pubkey，`createUpdaterArtifacts: true`，添加 updater 节点（内网端点） | 启用自动更新 |
| 3 | `src-tauri/Cargo.toml:43` | 保持 `tauri-plugin-updater = "2"` | 编译更新插件 |
| 4 | `src-tauri/src/lib.rs:883` | 保持 `.plugin(tauri_plugin_updater::...)` | 注册更新插件 |
| 5 | `src-tauri/capabilities/default.json:30` | 保持 `"updater:default"` | 更新权限声明 |
| 6 | `src-tauri/src/commands/update.rs` | `OFFICIAL_UPDATE_ENDPOINTS[0]` 和 `R2_LATEST_RELEASE_DOWNLOAD_PREFIX` → 内网 | 让"下载更新"也走内网（编译时字符串常量） |
| 7 | `.github/workflows/build-windows.yml`（新增） | CI 编译 + 签名 + 自动创建 Release + 生成 latest.json | CI 自动编译并签名 |

> 注意：文件 2-5 是从**上游原版恢复**，而非禁用。文件 2 替换为**自己的公钥**，文件 6 把端点常量指向内网（GitHub 保留为 fallback）。

---

## 前置准备：生成签名密钥对

自动更新需要 Tauri 对安装包进行 minisign 签名，客户端用公钥验签。**必须使用 Tauri 官方 CLI 生成密钥**（`minisign` 命令行生成的密钥格式与 Tauri 不完全兼容，会报 `Missing encoded key in secret key`）。

**推荐用空密码**（CI 友好，避免密码 secret 配置）：

```powershell
# 项目根目录（已经 pnpm install 过）
pnpm tauri signer generate -w .\key.key -p ""
```

两次回车确认空密码。

生成两个文件：
- `key.key` — 私钥文件（**本身就是 base64 一行**，由 Tauri CLI 写入）
- `key.key.pub` — 公钥文件

**公钥**：`key.key.pub` 的内容直接写入 `tauri.conf.json` 的 `updater.pubkey` 字段。

**私钥（用于 GitHub Secret）**：`key.key` 文件的**原始内容**直接粘贴到 `TAURI_SIGNING_PRIVATE_KEY`，**不要再 base64 编码**。Tauri 内部会做一次 base64 解码——如果再编码一次，会变成双重 base64，触发 `Missing encoded key in secret key` 错误。

> 密钥只需生成一次，之后每次版本迁移复用即可。如丢失需重新生成，会导致所有已安装客户端必须重装。

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

## 文件 2-5：启用自动更新（签名）

除文件 2 需替换公钥外，其余文件与上游保持一致即可。

### 文件 2：`src-tauri/tauri.conf.json`

在 `plugins` 下添加 `updater` 节点，`createUpdaterArtifacts` 改为 `true`，**必须**加 `dangerousInsecureTransportProtocol`：

```json
"plugins": {
    "deep-link": { ... },
    "updater": {
      "pubkey": "dHJ1c3RlZCBjb21tZW50Oi...",
      "endpoints": ["http://YOUR_SERVER/dbx-drivers/releases/latest/latest.json"],
      "dangerousInsecureTransportProtocol": true
    }
}
```

- `pubkey`：上一步生成的 Tauri 格式公钥（base64 一行）
- `endpoints`：内网 nginx 上的 `latest.json`
- `dangerousInsecureTransportProtocol: true`：**必需**。Tauri updater 默认拒绝 HTTP 端点（仅允许 HTTPS），启用此选项后允许通过 HTTP 获取更新清单。重启应用即生效。

### 文件 3-5

| 文件 | 状态 |
|---|---|
| `src-tauri/Cargo.toml` | 保持 `tauri-plugin-updater = "2"`（不要注释） |
| `src-tauri/src/lib.rs` | 保持 `.plugin(tauri_plugin_updater::Builder::new().build())` |
| `src-tauri/capabilities/default.json` | 保持 `"updater:default"` |

### 文件 6：`src-tauri/src/commands/update.rs`

DBX 的"检查更新"（`check_for_updates`）走 `dbx_core::update::fetch_latest_release` → `race_download`，**已自动用内网**（因为 `R2_CDN_BASE` 已改）。

但"下载更新"（`download_update`）用的是本文件里的 `OFFICIAL_UPDATE_ENDPOINTS` 和 `R2_LATEST_RELEASE_DOWNLOAD_PREFIX` 常量，**这些是编译时嵌入二进制的字符串**，不会跟随 `tauri.conf.json` 的 `endpoints`。必须改为内网：

```rust
const OFFICIAL_UPDATE_ENDPOINTS: [&str; 2] = [
    "http://YOUR_SERVER/dbx-drivers/releases/latest/latest.json",  // 内网主
    "https://github.com/t8y2/dbx/releases/latest/download/latest.json", // 公网 fallback
];
const R2_LATEST_RELEASE_DOWNLOAD_PREFIX: &str = "http://YOUR_SERVER/dbx-drivers/releases/latest/";
```

GitHub URL 保留为 fallback——内网服务器挂了时 DBX 仍能去 GitHub 找最新版本（公网不通时静默失败，不影响功能）。

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

      - name: Build Tauri app (with signing)
        shell: bash
        run: pnpm tauri build -- --no-default-features --features duckdb-bundled,mq-admin
        env:
          CI: true
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}

      - name: Generate latest.json for internal update
        shell: bash
        run: |
          VERSION=$(node -p "require('./package.json').version")
          MSI=$(ls target/release/bundle/msi/*.msi | head -1)
          MSI_NAME=$(basename "$MSI")
          EXE=$(ls target/release/bundle/nsis/*.exe | head -1)
          EXE_NAME=$(basename "$EXE")
          SIG="${MSI}.sig"
          SIGNATURE=$( [ -f "$SIG" ] && cat "$SIG" || echo "" )
          NOW=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
          cat > target/release/bundle/latest.json << EOF
          {
            "version": "${VERSION}",
            "notes": "内网版本更新。",
            "pub_date": "${NOW}",
            "platforms": {
              "windows-x86_64": {
                "signature": "${SIGNATURE}",
                "url": "http://YOUR_SERVER/dbx-drivers/releases/latest/${EXE_NAME}"
              },
              "windows-x86_64-nsis": {
                "signature": "${SIGNATURE}",
                "url": "http://YOUR_SERVER/dbx-drivers/releases/latest/${EXE_NAME}"
              },
              "windows-x86_64-msi": {
                "signature": "${SIGNATURE}",
                "url": "http://YOUR_SERVER/dbx-drivers/releases/latest/${MSI_NAME}"
              }
            }
          }
          EOF

      - name: Read version
        id: version
        shell: bash
        run: echo "value=$(node -p "require('./package.json').version")" >> $GITHUB_OUTPUT

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          tag_name: v${{ steps.version.outputs.value }}-internal
          name: "DBX 内网版 v${{ steps.version.outputs.value }}"
          body: |
            ## 部署到内网 nginx

            将以下文件放入 `http://YOUR_SERVER/dbx-drivers/releases/latest/`：
            - `.msi` / `.exe` 安装包
            - `latest.json`
          files: |
            target/release/bundle/msi/*.msi
            target/release/bundle/nsis/*.exe
            target/release/bundle/latest.json
```

> `--no-default-features --features duckdb-bundled,mq-admin` 跳过 `sqlite-sqlcipher`（依赖 OpenSSL 源码编译，Windows 上需要 Perl，内网用不到加密 SQLite）。

---

## GitHub Secrets 配置（只做一次）

到仓库 Settings → Secrets and variables → Actions → Repository secrets，添加以下 secrets：

| Name | Value | 说明 |
|---|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | 整个 `key.key` 文件的内容（一行 base64，由 `pnpm tauri signer generate` 输出） | **不要**再 base64 编码一次；不要注释行单独贴 |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 留空 / 不创建此 secret | 在 CI 模式下 tauri-cli 会自动用空字符串解密 |

**为什么**：Tauri 内部流程是 `base64_decode(secret) → SecretKeyBox::from_string → into_secret_key(password)`。所以 secret 必须是**已 base64 编码**的私钥文件整体；如果是明文（`untrusted comment: minisign ...` 两行文本）会报 `Missing encoded key in secret key`。

**为什么用空密码**：`pnpm tauri signer generate -p ""` 生成的是"密码为空字符串"的加密私钥，CI 模式下 tauri-cli 传 `Some("")` 给 minisign，正好匹配。如果生成时设了具体密码，则需把密码填到此 secret。

---

## 本地编译命令

**CMD（推荐用空密码密钥）：**

```cmd
set TAURI_SIGNING_PRIVATE_KEY=（粘贴 key.key 文件完整内容）
pnpm tauri build -- --no-default-features --features duckdb-bundled,mq-admin
```

**PowerShell：**

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY=(Get-Content .\key.key -Raw)
pnpm tauri build -- --no-frozen-lockfile -- --no-default-features --features duckdb-bundled,mq-admin
```

> 不设 `TAURI_SIGNING_PRIVATE_KEY` 环境变量会报 `failed to decode secret key: Missing encoded key in secret key`（通常是 secret 内容格式不对，或 base64 编码后又编码了一次）。

---

## 内网文件服务器搭建

### 目录结构

```
http://YOUR_SERVER/dbx-drivers/
├── agents/
│   ├── agent-registry.json          ← 从 GitHub agents-latest release 下载，无需修改
│   ├── drivers/
│   │   └── dbx-agent-*.jar / *.exe  ← 全部驱动 JAR + 原生二进制
│   └── jre/
│       └── dbx-jre-*.tar.gz         ← 各平台 JRE 归档
├── releases/
│   └── latest/
│       ├── DBX_0.5.62_x64_zh-CN.msi  ← 内网版安装包（CI 编译后生成）
│       ├── DBX_0.5.62_x64-setup.exe  ← 内网版安装包（CI 编译后生成）
│       ├── latest.json               ← 自动更新清单（CI 编译后生成，URL指向内网）
│       └── dbx-jdbc-plugin-latest.zip ← 从主版本 release 下载
└── downloads/                        ← 离线 ZIP 包（可选，用于手动导入）
    └── dbx-agents-offline-*.zip
```

### 获取驱动文件

```bash
# agent 驱动和 JRE（全部从 agents-latest release 下载）
gh release download agents-latest --repo t8y2/dbx --dir ./agents/
mkdir -p agents/drivers agents/jre
mv agents/dbx-agent-*.jar agents/drivers/
mv agents/dbx-agent-*-* agents/drivers/
mv agents/dbx-jre-*.tar.gz agents/jre/

# JDBC 插件
mkdir -p releases/latest
curl -L -o releases/latest/dbx-jdbc-plugin-latest.zip "https://github.com/t8y2/dbx/releases/latest/download/dbx-jdbc-plugin-latest.zip"

# 离线包移到 downloads/
mkdir -p downloads
mv agents/dbx-agents-offline-*.zip downloads/
```

### 获取内网版安装包（每次 CI 编译完成后）

1. 打开 https://github.com/wobs1028/dbx/releases → 找到 `vX.X.XX-internal`
2. 下载 `.msi`、`.exe`、`latest.json`
3. 放入 nginx 的 `releases/latest/` 目录

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

**不要每次重新 fork。** `main` 分支保持纯净追上游，`internal-build` 分支放定制改动。

### 首次设置（只做一次）

```bash
# 添加上游 remote + GitHub Secrets（参考上方"前置准备"和"GitHub Secrets 配置"两节）
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
# https://github.com/wobs1028/dbx/actions/workflows/build-windows.yml
# → Run workflow → 选 internal-build 分支

# === 4. 编译成功后在 Releases 页面下载 .msi + .exe + latest.json ===
# 放入 nginx 的 releases/latest/ 目录替换旧文件

# === 5. 更新 nginx 上的驱动文件 ===
# 到文件服务器上重新下载 agents-latest 的驱动文件（参考上方"获取驱动文件"）
```

> 第 4 步完成后，所有内网用户启动 DBX 时会自动检测到新版本并弹窗提示更新。
