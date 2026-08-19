# DBX v0.5.76 内网驱动部署操作手册（v2 路径）

> 配套分支：`internal-build`（已推送，commit `31d27a7a3`）
> 目标：在 nginx 上新增 `/dbx-drivers-v2/` 路径，供新版 app（v0.5.76）使用新驱动格式。

---

## 一、总体结构

```
nginx 挂载两个路径：
  /dbx-drivers/    ← 老版本 app 用（现有，保持不动）
  /dbx-drivers-v2/ ← 新版本 app 用（本次全新部署）
```

**原理**：新 app 编译时 `R2_CDN_BASE = http://25.75.3.1/dbx-drivers-v2/`。
驱动下载流程：**先试内网 -v2，失败回退 GitHub 官方**。所以 -v2 只需部署内网实际要用的文件。

---

## 二、目录结构（照抄老路径，内容换新）

```
/dbx-drivers-v2/
├── agents/
│   ├── agent-registry.json        ← 新清单（agents-v0.2.76）
│   ├── drivers/
│   │   ├── dbx-agent-*.tar.zst    ← 新格式驱动包（替换老 .jar 裸文件）
│   │   └── *-legacy-placeholder.jar ← registry 引用的占位文件（必须存在，见下）
│   └── jre/
│       └── dbx-jre-21-*.tar.zst   ← 新格式 JRE（替换老 .tar.gz）
├── downloads/
│   └── dbx-agents-offline-*.zip   ← 新离线包（可选，给完全离线用户）
└── releases/
    └── latest/
        ├── DBX_0.5.76_x64-setup.exe   ← CI 产物
        ├── DBX_0.5.76_x64_en-US.msi   ← CI 产物
        └── latest.json                ← CI 产物
```

---

## 三、需要下载的文件清单

### 3.1 一键下载命令（完整版，含所有平台）

在联网机器上执行：

```bash
# 1. 新 registry + JRE
mkdir -p dbx-v2/{drivers,jre}
gh release download agents-v0.2.76 --repo t8y2/dbx \
  --pattern "agent-registry.json" --dir dbx-v2/

# 2. 全部驱动包（.tar.zst，所有平台）+ 占位 jar
gh release download agents-v0.2.76 --repo t8y2/dbx \
  --pattern "dbx-agent-*.tar.zst" --dir dbx-v2/drivers/
gh release download agents-v0.2.76 --repo t8y2/dbx \
  --pattern "*-legacy-placeholder.jar" --dir dbx-v2/drivers/

# 3. JRE（所有平台）
gh release download agents-v0.2.76 --repo t8y2/dbx \
  --pattern "dbx-jre-*.tar.zst" --dir dbx-v2/jre/

# 4. 离线包（可选）
gh release download agents-v0.2.76 --repo t8y2/dbx \
  --pattern "dbx-agents-offline-*.zip" --dir dbx-v2/downloads/
```

### 3.2 最小集（仅 Windows x64，推荐内网）

**必须**：
- `agent-registry.json`（1 个，~20KB）
- 全部 `*.tar.zst` 驱动包中的 **jar 型驱动**（37 个，平台无关，一个包适用所有平台）
- **windows-x64 的 native 驱动**（9 个）：oracle、xugu、kingbase、vastbase、duckdb、rabbitmq、cassandra、tdengine
- 所有 `*-legacy-placeholder.jar`（9 个，registry 引用但 size=0，占位即可）
- `dbx-jre-21-windows-x64.tar.zst`（1 个）

> ⚠️ **重要**：registry 中每个驱动条目引用的 URL 文件名**必须**在服务器上存在。
> native 驱动若不部署某个平台，该平台客户端会回退 GitHub 下载（内网无外网则失败）。
> 建议至少部署 windows-x64 全套 + windows-aarch64（如有 ARM 设备）。

### 3.3 jar 型驱动完整清单（37 个，平台无关）

| 驱动 | 文件 | 驱动 | 文件 |
|---|---|---|---|
| access | dbx-agent-access-0.1.37.tar.zst | mongodb | dbx-agent-mongodb-0.1.42.tar.zst |
| bigquery | dbx-agent-bigquery-0.1.41.tar.zst | oceanbase-oracle | dbx-agent-oceanbase-oracle-0.1.36.tar.zst |
| dameng | dbx-agent-dameng-0.1.45.tar.zst | oscar | dbx-agent-oscar-0.1.15.tar.zst |
| databend | dbx-agent-databend-0.1.28.tar.zst | rocketmq | dbx-agent-rocketmq-0.1.2.tar.zst |
| databricks | dbx-agent-databricks-0.1.35.tar.zst | saphana | dbx-agent-saphana-0.1.35.tar.zst |
| db2 | dbx-agent-db2-0.1.40.tar.zst | snowflake | dbx-agent-snowflake-0.1.40.tar.zst |
| etcd | dbx-agent-etcd-0.1.28.tar.zst | spark | dbx-agent-spark-0.1.12.tar.zst |
| exasol | dbx-agent-exasol-0.1.35.tar.zst | sqlserver-legacy | dbx-agent-sqlserver-legacy-0.1.11.tar.zst |
| firebird | dbx-agent-firebird-0.1.35.tar.zst | sundb | dbx-agent-sundb-0.1.40.tar.zst |
| gbase8a | dbx-agent-gbase8a-0.1.35.tar.zst | teradata | dbx-agent-teradata-0.1.35.tar.zst |
| gbase8s | dbx-agent-gbase8s-0.1.29.tar.zst | trino | dbx-agent-trino-0.1.40.tar.zst |
| goldendb | dbx-agent-goldendb-0.1.40.tar.zst | uxdb | dbx-agent-uxdb-0.1.7.tar.zst |
| h2 | dbx-agent-h2-0.1.40.tar.zst | vertica | dbx-agent-vertica-0.1.35.tar.zst |
| h2-legacy | dbx-agent-h2-legacy-0.1.10.tar.zst | yashandb | dbx-agent-yashandb-0.1.36.tar.zst |
| highgo | dbx-agent-highgo-0.1.38.tar.zst | zookeeper | dbx-agent-zookeeper-0.1.18.tar.zst |
| hive | dbx-agent-hive-0.1.40.tar.zst | | |
| informix | dbx-agent-informix-0.1.41.tar.zst | | |
| iotdb | dbx-agent-iotdb-0.1.28.tar.zst | | |
| iris | dbx-agent-iris-0.1.31.tar.zst | | |
| kafka | dbx-agent-kafka-0.1.6.tar.zst | | |
| kylin | dbx-agent-kylin-0.1.40.tar.zst | | |

### 3.4 native 驱动（按平台，9 个驱动 × 平台数）

| 驱动 | windows-x64 文件 | 版本 |
|---|---|---|
| oracle | dbx-agent-oracle-0.1.41-windows-x64.tar.zst | 0.1.41 |
| xugu | dbx-agent-xugu-0.1.29-windows-x64.tar.zst | 0.1.29 |
| kingbase | dbx-agent-kingbase-0.1.43-windows-x64.tar.zst | 0.1.43 |
| vastbase | dbx-agent-vastbase-0.1.39-windows-x64.tar.zst | 0.1.39 |
| duckdb | dbx-agent-duckdb-0.1.3-windows-x64.tar.zst | 0.1.3 |
| rabbitmq | dbx-agent-rabbitmq-0.1.1-windows-x64.tar.zst | 0.1.1 |
| cassandra | dbx-agent-cassandra-0.1.38-windows-x64.tar.zst | 0.1.38 |
| tdengine | dbx-agent-tdengine-0.1.40-windows-x64.tar.zst | 0.1.40 |

（每个驱动还有 linux-x64 / linux-aarch64 / macos-x64 / macos-aarch64 / windows-aarch64 版本，按需下载）

### 3.5 占位 jar（9 个，registry 引用，size=0）

```
dbx-agent-cassandra-legacy-placeholder.jar
dbx-agent-duckdb-legacy-placeholder.jar
dbx-agent-kingbase-legacy-placeholder.jar
dbx-agent-oracle-legacy-placeholder.jar
dbx-agent-rabbitmq-legacy-placeholder.jar
dbx-agent-tdengine-legacy-placeholder.jar
dbx-agent-vastbase-legacy-placeholder.jar
dbx-agent-xugu-legacy-placeholder.jar
```

> 这些是**空占位**（registry 里 `size: 0`、`sha256: ""`）。老 app 才会用 jar 回退；
> 新 app 用 native。**但 registry 的 URL 指向它，文件必须存在**，否则下载 404 会报错。

### 3.6 JRE

```
dbx-jre-21-windows-x64.tar.zst   （内网 Windows 最小集只需这一个）
（其他平台按需：linux-x64/aarch64、macos-x64/aarch64、windows-aarch64）
```

---

## 四、部署步骤（服务器上执行）

```bash
# 假设 nginx root = /var/www/html/dbx_files（与老路径同根）
cd /var/www/html/dbx_files

# 1. 创建 v2 目录结构（照抄老路径）
mkdir -p dbx-drivers-v2/agents/drivers dbx-drivers-v2/agents/jre \
         dbx-drivers-v2/downloads dbx-drivers-v2/releases/latest

# 2. 上传下载好的文件
#    agent-registry.json → dbx-drivers-v2/agents/
#    *.tar.zst 驱动      → dbx-drivers-v2/agents/drivers/
#    *-legacy-placeholder.jar → dbx-drivers-v2/agents/drivers/
#    dbx-jre-*.tar.zst   → dbx-drivers-v2/agents/jre/
#    dbx-agents-offline-*.zip → dbx-drivers-v2/downloads/（可选）

# 3. 验证 registry 引用的文件都能访问
#    用脚本比对 registry 中所有 URL 文件名是否存在于服务器目录
```

---

## 五、nginx 配置（照抄 dbx-drivers 即可）

老路径配置不用动，新增一个 location 或直接复用同一个 server 块（因为两个路径同 root）：

```nginx
server {
    listen 80;
    root /var/www/html/dbx_files;   # 与老路径同一个 root
    add_header Accept-Ranges bytes;
    gzip off;
    location / {
        autoindex on;
        try_files $uri =404;
    }
}
```

> 因为 `/dbx-drivers/` 和 `/dbx-drivers-v2/` 是**同一 root 下的两个子目录**，
> nginx 配置**无需改动**！只要把文件放进 `dbx_files/dbx-drivers-v2/` 目录即可自动可访问。

---

## 六、发布流程（CI 完成后）

1. GitHub Actions → `build-windows.yml` → Run workflow → branch `internal-build`
2. CI 完成（~40 分钟）后，从 Release `v0.5.76-internal` 下载：
   - `DBX_0.5.76_x64_en-US.msi`
   - `DBX_0.5.76_x64-setup.exe`
   - `latest.json`
3. 复制到 `dbx-drivers-v2/releases/latest/`
4. **老路径更新**：把 `latest.json` 和安装包也放一份到老路径 `dbx-drivers/releases/latest/`
   （这样老用户检查更新 → 看到 v0.5.76 → 自动升级 → 变新 app → 走 v2 拉新驱动）
5. 通知内网用户升级

---

## 七、验证清单

- [ ] `http://25.75.3.1/dbx-drivers-v2/agents/agent-registry.json` 可访问
- [ ] registry 中所有驱动 URL 对应的文件存在（可用脚本核对）
- [ ] `http://25.75.3.1/dbx-drivers-v2/releases/latest/latest.json` 可访问（CI 后）
- [ ] 新 app v0.5.76 驱动管理 → 一键安装 → 从 v2 下载成功
- [ ] 重点回归：Neo4j、TDengine、Cassandra、Vastbase、RabbitMQ（原生驱动迁移）
- [ ] 老 app v0.5.68 用户仍可正常用老路径驱动（不受影响）
