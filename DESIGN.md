# E02T 跨平台桌面端产测工具 — 设计文档

## 1. 概述

### 1.1 项目目标

基于 `usb_cdc_config_and_production_design.md` 协议，开发一款跨平台 PC 桌面端产测与配置工具，通过 USB CDC 串口（ttyACM0 / COMx）与 4G 模组通信，支持：

1. 设备配置文件（`U:/config.ini`）的推送、回读、校验与恢复
2. 4G 模组自身产测（SIM/信号/网络/数据业务）
3. MCU 产测桥接（蓝牙/LED/充电/电池/麦克风/按键/电量计/时间同步）
4. 测试报告生成与追溯

### 1.2 适用产品

| 产品 | 配置功能 | 产测功能 | MCU 桥接 |
|------|---------|---------|---------|
| E02T (feature/shanli) | 基础支持 | 重点支持 | 支持 |
| 4GCAM | 重点支持 | 基础支持 | 可选 |
| 其他版本 | 重点支持 | 可选 | 可选 |

工具连接设备后通过 `AT+CAP?` 自动识别产品能力，动态适配界面。

---

## 2. 技术栈

| 层 | 选型 | 说明 |
|----|------|------|
| 框架 | Tauri 2 | 轻量跨平台桌面框架（~15MB 安装包） |
| 前端 | Vue 3 + TypeScript | 响应式 UI |
| UI 组件库 | Naive UI | 企业级 Vue 3 组件库 |
| 后端 | Rust | Tauri 后端，负责串口通信和业务逻辑 |
| 串口 | serialport (Rust crate) | 跨平台串口通信 |
| 构建 | Vite | 前端构建工具 |
| 打包 | tauri-bundler | Windows (.msi) / Linux (.deb/.AppImage) |

### 2.1 选型理由

- **Tauri vs Electron**：内存占用 30MB vs 200MB+，打包体积 15MB vs 100MB+，产线 PC 配置通常不高
- **Rust 串口**：`serialport` crate 在 Windows/Linux 上稳定可靠，支持异步读写
- **Vue 3 + Naive UI**：上手快，中文文档完善，表格/表单/通知组件齐全

---

## 3. 系统架构

```
┌─────────────────────────────────────────────────────────┐
│                     Vue 3 Frontend                       │
│                                                          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐  │
│  │ 连接面板  │  │ 配置面板  │  │ 产测面板  │  │ 报告   │  │
│  └─────┬────┘  └─────┬────┘  └─────┬────┘  └───┬────┘  │
│        └─────────────┴──────────────┴────────────┘       │
│                         │ Tauri IPC (invoke/event)        │
├─────────────────────────┼────────────────────────────────┤
│                    Rust Backend                           │
│                                                          │
│  ┌────────────────┐  ┌─────────────────┐  ┌───────────┐ │
│  │ serial_manager │  │ at_protocol     │  │ commands  │ │
│  │ - scan_ports   │  │ - send_cmd      │  │ (IPC层)   │ │
│  │ - connect      │  │ - parse_resp    │  └───────────┘ │
│  │ - disconnect   │  │ - async_events  │                 │
│  │ - async_read   │  │ - timeouts      │                 │
│  └────────┬───────┘  └────────┬────────┘                │
│           └───────────────────┘                          │
│                       │                                  │
│            USB CDC (ttyACM0 / COMx)                      │
└───────────────────────┼──────────────────────────────────┘
                        │
                   ┌────┴────┐        ┌─────────┐
                   │ 4G 模组  │──UART──│  MCU    │
                   └─────────┘        └─────────┘
```

---

## 4. 通信协议

### 4.1 协议格式

采用 AT 风格文本协议，每条命令以 `\r\n` 结尾。

**发送格式**：
```
AT+<CMD>\r\n
AT+<CMD>?\r\n
AT+<CMD>=<args>\r\n
```

**返回格式**：
```
成功: +<CMD>: <result>\r\n 后接 OK\r\n
失败: +<CMD>: ERR,<code>\r\n 后接 ERROR\r\n
```

### 4.2 命令总表

#### 通用命令（始终可用，不依赖产测模式）

| 命令 | 功能 | 超时 |
|------|------|------|
| `AT+CAP?` | 查询设备能力 | 2s |
| `AT+VER?` | 查询固件版本 | 2s |
| `AT+DEV?` | 查询设备信息 | 2s |
| `AT+PROD=1` | 进入产测模式 | 5s |
| `AT+PROD=0` | 退出产测模式 | 5s |

#### 配置命令（需进入产测模式）

| 命令 | 功能 | 超时 |
|------|------|------|
| `AT+CFGINFO?` | 查询配置文件信息 | 3s |
| `AT+CFGSTART` | 开始配置上传会话 | 3s |
| `AT+CFGSET="<line>"` | 写入一行配置 | 3s |
| `AT+CFGSAVE` | 结束上传并保存 | 5s |
| `AT+CFGREAD?` | 回读配置内容 | 5s |
| `AT+CFGDEF` | 恢复默认配置 | 3s |

#### 模组自测命令（需进入产测模式）

| 命令 | 功能 | 超时 |
|------|------|------|
| `AT+MDINFO?` | 模组基础信息 | 3s |
| `AT+MDSIM?` | SIM 状态 | 2s |
| `AT+MDREG?` | 网络注册状态 | 2s |
| `AT+MDSIG?` | 信号质量 | 2s |
| `AT+MDDATA?` | 数据业务状态 | 2s |
| `AT+MDDNS="<host>"` | DNS 测试 | 10s |
| `AT+MDPING="<ip>",<count>` | Ping 测试 | 20s |
| `AT+MDALL?` | 综合网络测试 | 30s |

#### MCU 测试入口命令（需进入产测模式）

| 命令 | 功能 | 超时 | 判定方式 |
|------|------|------|---------|
| `AT+MCUBVER?` | 蓝牙版本 | 2s | 自动 |
| `AT+MCUMAC?` | 蓝牙 MAC | 2s | 自动 |
| `AT+MCUCHG?` | 充电信息 | 2s | 自动 |
| `AT+MCUVBAT?` | 电池电压 | 2s | 自动 |
| `AT+MCULED=<0/1>` | LED 跑马灯 | 3s | 人工 |
| `AT+MCUFBMIC=<0/1>` | FB 麦回环 | 3s | 人工 |
| `AT+MCUPMIC=<0/1>` | 主麦回环 | 3s | 人工 |
| `AT+MCUKEY` | 按键测试 | 3s | 半自动 |
| `AT+MCUGAUGE` | 电量计校准 | 5s | 自动 |
| `AT+MCUTIME="<datetime>"` | 时间同步 | 2s | 自动 |
| `AT+MCURST` | 恢复出厂 | 5s | 自动 |
| `AT+MCURAW="<cmd>"` | MCU 透传 | 5s | 调试用 |

### 4.3 错误码映射

| 错误码 | 含义 | 归属 |
|--------|------|------|
| `MODE` | 未进入产测模式 | 通用 |
| `UNSUP` | 当前产品不支持该命令 | 通用 |
| `ARG` | 参数错误 | 通用 |
| `FS` | 文件系统错误 | 配置域 |
| `FMT` | 配置格式错误 | 配置域 |
| `IO` | 文件/UART 读写错误 | 配置域/MCU域 |
| `BUSY` | 已有配置任务进行中 | 配置域 |
| `TO` | MCU 超时未响应 | MCU域 |
| `FAIL` | MCU 执行失败 | MCU域 |

### 4.4 异步事件

按键测试模式下，MCU 通过模组异步上报按键事件：

```
+MCUKEY:KEY=PTT,STATE=PRESS
+MCUKEY:KEY=VOL+,STATE=PRESS
```

前端通过 Tauri Event 监听异步上报，实时更新按键状态。

---

## 5. 状态机设计

### 5.1 产测模式状态机

```
IDLE (未进入产测)
  │
  ├─ AT+CAP? / AT+VER? / AT+DEV? → 始终可用
  ├─ 其他命令 → 返回 ERR,MODE
  │
  ↓ AT+PROD=1
  │
ENTERING (进入中，等待模组切换 + MCU 应答)
  │
  ├─ 成功 → PRODUCTION
  └─ 失败(ERR,TO 等) → IDLE
  │
PRODUCTION (已进入产测)
  │
  ├─ 所有配置/模组/MCU 命令可用
  │
  ↓ AT+PROD=0
  │
EXITING (退出中，等待模组切换 + MCU 应答)
  │
  ├─ 成功 → IDLE
  └─ 失败 → 仍处于 PRODUCTION（提示重试）
```

前端行为：
- 状态栏始终显示当前模式（IDLE / PRODUCTION）
- 用户操作产测/配置命令前，前端检查状态，若 IDLE 则提示"请先进入产测模式"
- 收到 `ERR,MODE` 错误时，弹出引导提示

### 5.2 配置上传状态机

```
IDLE
  │
  ↓ AT+CFGSTART (成功)
  │
RECEIVING
  │
  ├─ AT+CFGSET="..." (可重复)
  │   └─ 更新行计数 LINE=N
  │
  ↓ AT+CFGSAVE (成功)
  │
DONE → 自动回读 AT+CFGREAD? → 对比校验
```

异常处理：
- 单包超时（3-5s）→ 重试 1 次，仍失败则中止
- 整体上传超时（60s）→ 中止并提示
- CFGSAVE 返回 ERR,FMT → 提示格式错误，需重新上传
- USB 断线 → 清理状态，提示重连

### 5.3 MCU 命令状态机

```
IDLE
  │
  ↓ 发送 MCU 命令
  │
WAIT_RESPONSE
  │
  ├─ 收到 +CMD:... + OK → 解析结果 → IDLE
  ├─ 收到 +CMD:ERR,xx + ERROR → 记录错误 → IDLE
  └─ 超时 → ERR,TO → IDLE
```

约束：同一时刻只允许一个 MCU 命令在执行，避免 UART 响应交叉。

---

## 6. 前端界面设计

### 6.1 整体布局

```
┌──────────────────────────────────────────────────────────────┐
│  [Logo] E02T 产测工具 v1.0     [COM3 ▾][连接] 状态: PRODUCTION│
├──────────────────────────────────────────────────────────────┤
│  [产测]  [配置]  [调试]  [报告]  [设置]                        │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│                       (页面内容区)                             │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│  设备: E02T | IMEI: 86123... | FW: v1.2.3 | BT: 1.0.3       │
└──────────────────────────────────────────────────────────────┘
```

### 6.2 产测页面

所有测试项（自动/手动/半自动）统一使用单个 **[测试]** 按钮，按 `judgeType` 分派不同行为：

- **auto**：直接执行 AT 命令并自动判定（MCURST 需二次确认弹窗）
- **manual**：点击后弹出手动测试弹框，自动发送启动命令，操作员判定后发送停止命令
- **semi_auto**：点击后弹出按键测试弹框

```
┌─ 操作栏 ───────────────────────────────────────────────────┐
│  [▶ 一键产测]  [停止]  [重置]   通过: 7  失败: 1  总计: 14  │
│                                               [日志/关闭日志]│
└────────────────────────────────────────────────────────────┘

┌─ 统一测试表格 ──────────────────────────────────────────────┐
│  #  域    测试项      结果                 耗时   状态  操作  │
│  1  模组  SIM 状态    READY                120ms  PASS [测试] │
│  2  模组  网络注册    CREG=1,CGREG=1       85ms   PASS [测试] │
│  3  模组  信号质量    CSQ=18,RSRP=-92      90ms   PASS [测试] │
│  ...                                                         │
│  10 MCU   LED 测试    —                    —      待执行[测试]│
│  11 MCU   FB麦回环    —                    —      待执行[测试]│
│  12 MCU   主麦回环    —                    —      待执行[测试]│
│  13 MCU   按键测试    —                    —      待执行[测试]│
│  ...                                                         │
└──────────────────────────────────────────────────────────────┘
```

**手动测试弹框**（LED / FB麦回环 / 主麦回环）：

```
┌─ LED 测试 ────────────────────────────────┐
│                                            │
│     请观察设备 LED 是否正常亮起             │
│                                            │
│         [  PASS  ]    [  FAIL  ]           │
│                                            │
│                 [取消]                      │
└────────────────────────────────────────────┘
```

弹框打开时自动发送启动命令（如 `AT+MCULED=1`），点击 PASS/FAIL 时自动发送停止命令（如 `AT+MCULED=0`）。取消则恢复待执行状态。

**按键测试弹框**：

```
┌─ 按键测试 ─────────────────────────────────┐
│                                             │
│   ┌─────┐  ┌─────┐  ┌─────┐  ┌────────┐   │
│   │ PTT │  │VOL+ │  │VOL- │  │ POWER  │   │
│   │  ●  │  │  ●  │  │  ○  │  │   ○    │   │
│   └─────┘  └─────┘  └─────┘  └────────┘   │
│                                             │
│   ● 已检测(绿)  ○ 未检测(灰)               │
│   进度: 2/4                                 │
│                                             │
│         [  PASS  ]    [  FAIL  ]            │
│                  [停止]                      │
└─────────────────────────────────────────────┘
```

异步事件 `+MCUKEY:KEY=PTT,STATE=PRESS` 到达时对应按键变绿。全部检测完自动 PASS。超时未全部完成则 FAIL。操作员也可随时手动判定 PASS/FAIL。

**三态日志面板**：

日志面板默认关闭，有三种状态：
- **关闭**（默认）：不显示日志区域
- **展开**：显示完整日志滚动区域（180px 高度）
- **折叠**：仅显示最新一条日志的单行预览

日志内容包含 TX/RX 详情（发送指令和接收响应），支持文本选择和复制。一键产测不会自动打开日志。

**测试数据持久化**：

产测页面的测试结果数据保存在 Pinia store 中，切换到其他页面再返回时数据不会丢失。仅在设置页面修改了配置（`configDirty` 标志）后，回到产测页面才会重新加载配置并清空数据。

### 6.3 配置页面

```
┌─ 配置管理 ──────────────────────────────────────────────────┐
│                                                              │
│  来源: (●) 手动编辑  ( ) 从模板  ( ) 从文件导入              │
│                                                              │
│  模板: [默认配置 ▾]  [加载]  [另存为模板...]                  │
│                                                              │
│  ┌─ 编辑区 ────────────────────────────────────────────┐     │
│  │ 1 │ version=1                                       │     │
│  │ 2 │ server=192.168.1.100                            │     │
│  │ 3 │ port=9000                                       │     │
│  │ 4 │ # APN 配置                                      │     │
│  │ 5 │ apn=cmnet                                       │     │
│  │ 6 │ volume=8                                        │     │
│  │ 7 │                                                 │     │
│  └──────────────────────────────────────────────────────┘     │
│                                                              │
│  [推送到设备]  [从设备回读]  [恢复默认]  [清空设备配置]        │
│                                                              │
│  ┌─ 推送进度 ──────────────────────────────────────────┐     │
│  │  状态: 完成  ████████████ 6/6 行                     │     │
│  │  回读校验: ✓ 与推送内容一致                           │     │
│  └──────────────────────────────────────────────────────┘     │
│                                                              │
│  ┌─ 对比视图（推送 vs 回读）──────────────────────────┐       │
│  │  推送内容             │  回读内容                   │       │
│  │  version=1            │  version=1            ✓    │       │
│  │  server=192.168.1.100 │  server=192.168.1.100 ✓    │       │
│  │  port=9000            │  port=9000            ✓    │       │
│  │  apn=cmnet            │  apn=cmnet            ✓    │       │
│  │  volume=8             │  volume=8             ✓    │       │
│  └───────────────────────┴────────────────────────────┘       │
└──────────────────────────────────────────────────────────────┘
```

**操作说明**：

| 操作 | 协议流程 | 说明 |
|------|---------|------|
| 推送到设备 | `CFGSTART` → 逐行 `CFGSET` → `CFGSAVE` → `CFGREAD?` 回读校验 | 自动对比推送与回读内容 |
| 从设备回读 | `CFGREAD?` → 显示到编辑区 | 查看设备当前配置 |
| 恢复默认 | `AT+CFGDEF` | 删除配置恢复程序默认 |
| 清空设备配置 | `CFGSTART` → `CFGSAVE`（不发 CFGSET） | 写入空文件 |

### 6.4 调试页面

```
┌─ 调试终端 ──────────────────────────────────────────────────┐
│                                                              │
│  模式: (●) AT 命令   ( ) MCU 透传                            │
│                                                              │
│  ┌─ 日志 ──────────────────────────────────────────────┐     │
│  │ [14:30:01] TX > AT+CAP?                              │     │
│  │ [14:30:01] RX < +CAP: PRODUCT=E02T,CONFIG=1,...      │     │
│  │ [14:30:01] RX < OK                                   │     │
│  │ [14:30:05] TX > AT+MDSIM?                            │     │
│  │ [14:30:05] RX < +MDSIM:READY                         │     │
│  │ [14:30:05] RX < OK                                   │     │
│  │ [14:30:10] TX > AT+MCURAW="AT+BVER?"                 │     │
│  │ [14:30:10] RX < +MCURAW:+BVER:VER=1.0.3              │     │
│  │ [14:30:10] RX < +MCURAW:OK                           │     │
│  │ [14:30:10] RX < OK                                   │     │
│  └──────────────────────────────────────────────────────┘     │
│                                                              │
│  输入: [AT+MDSIG?                    ] [发送] [清屏]         │
│                                                              │
│  MCU透传模式说明: 输入 MCU 命令(如 AT+BVER?)，               │
│  自动包装为 AT+MCURAW="AT+BVER?" 发送                        │
└──────────────────────────────────────────────────────────────┘
```

### 6.5 报告页面

```
┌─ 测试报告 ──────────────────────────────────────────────────┐
│                                                              │
│  筛选: [日期范围 ▾] [结果 ▾] [产品 ▾]  [搜索IMEI/MAC...]    │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │ #  时间          IMEI             产品  结果  操作      │  │
│  │ 1  06-02 14:30   861234567890123  E02T  PASS  [详情]   │  │
│  │ 2  06-02 14:25   861234567890124  E02T  FAIL  [详情]   │  │
│  │ 3  06-02 14:20   861234567890125  E02T  PASS  [详情]   │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  [导出 CSV]  [导出 PDF]   统计: 今日 15 台, PASS 13, FAIL 2  │
└──────────────────────────────────────────────────────────────┘
```

### 6.6 设置页面

设置修改后自动保存（500ms 防抖），无需手动点击保存按钮。

**基础设置**：
- 操作员姓名
- 串口波特率
- 数据目录

**测试项配置**（per-item）：

每个测试项可独立配置：

| 配置项 | 说明 |
|--------|------|
| 启用/禁用 | 控制该项是否出现在产测页面 |
| 重试次数 | 失败后额外重试次数（0=只执行1次，1=最多2次） |
| 超时(ms) | 单次执行超时时间 |

部分测试项有额外参数：

| 测试项 | 参数 | 说明 |
|--------|------|------|
| MDSIG (信号质量) | csq_min, rssi_min, rsrp_min | CSQ 0-31 阈值、RSSI dBm 阈值、RSRP 阈值 |
| MDALL (综合测试) | ping_enabled, ping_host, ping_count | 是否测试 Ping、Ping 地址、次数 |
| MCUVBAT (电池电压) | mv_min, mv_max | 电压合格范围 (mV) |
| MCUKEY (按键测试) | keys, timeout_s | 待测按键列表、超时秒数 |

可通过"恢复默认"将所有测试项配置重置为系统默认值。

**配置与产测联动**：

设置修改后通过 `configDirty` 标志通知产测页面。切换到产测页面时检查该标志，若配置已变更则重新加载配置并清空上次测试数据，确保配置立即生效（如关闭 Ping 后产测不再测试 Ping）。

---

## 7. Rust 后端模块设计

### 7.1 模块划分

```
src-tauri/src/
├── main.rs              # Tauri 应用入口
├── lib.rs               # 模块声明和 Tauri 命令注册
├── commands.rs          # Tauri IPC 命令实现
├── serial_manager.rs    # 串口管理（扫描/连接/读写/事件监听）
├── at_protocol.rs       # AT 协议引擎（命令映射/解析/判定）
├── report.rs            # 报告 JSON 读写与 CSV 导出
├── settings.rs          # 设置与模板持久化
└── types.rs             # 共享数据结构（含 default_test_items）
```

### 7.2 核心数据结构

```rust
// 设备能力
struct DeviceCapability {
    product: String,       // "E02T" / "4GCAM"
    config: bool,          // CONFIG=1
    production: bool,      // PRODUCTION=1
    mcu: bool,             // MCU=1
}

// AT 响应
struct ATResponse {
    lines: Vec<String>,    // ["+MDSIM:READY"]
    success: bool,         // OK=true, ERROR=false
    error_code: Option<String>,  // "TO", "MODE", etc.
}

// 单项测试结果
struct TestResult {
    id: String,
    command: String,       // 实际发送的 AT 命令（用于日志）
    status: String,        // "pass" / "fail" / "manual_pending"
    raw_response: String,
    parsed_data: HashMap<String, String>,
    error: String,
    duration_ms: u64,
}

// 测试项配置（per-item）
struct TestItemConfig {
    id: String,
    enabled: bool,
    retries: u32,          // 额外重试次数（0=1次, 1=最多2次）
    timeout_ms: u64,
    params: HashMap<String, serde_json::Value>,
}

// 应用设置
struct AppSettings {
    operator: String,
    baud_rate: u32,
    data_dir: String,
    test_items: Vec<TestItemConfig>,
}

// 测试报告
struct TestReport {
    id: String,
    timestamp: String,
    operator: String,
    device: DeviceReportInfo,
    overall: String,       // "PASS" / "FAIL"
    duration_ms: u64,
    items: Vec<TestReportItem>,
}

struct DeviceReportInfo {
    product: String,
    imei: String,
    iccid: String,
    fw_version: String,
    bt_version: String,
    bt_mac: String,
}
```

### 7.3 IPC 命令接口

```rust
// 串口管理
#[tauri::command] fn scan_ports() -> Vec<String>;
#[tauri::command] fn connect(port: String) -> Result<DeviceCapability>;
#[tauri::command] fn disconnect() -> Result<()>;

// AT 命令（底层）
#[tauri::command] fn send_at_command(cmd: String, timeout_ms: u64) -> Result<ATResponse>;

// 产测控制
#[tauri::command] fn enter_production_mode() -> Result<ATResponse>;
#[tauri::command] fn exit_production_mode() -> Result<ATResponse>;

// 产测执行（读取 AppSettings 获取 per-item config）
#[tauri::command] fn run_single_test(test_id: String) -> Result<TestResult>;
#[tauri::command] fn run_auto_test() -> Result<Vec<TestResult>>;
#[tauri::command] fn manual_judge(test_id: String, pass: bool) -> Result<()>;
#[tauri::command] fn query_device_info() -> Result<HashMap<String, String>>;

// 配置管理
#[tauri::command] fn config_read() -> Result<Vec<String>>;
#[tauri::command] fn config_info() -> Result<ConfigInfo>;
#[tauri::command] fn config_upload(lines: Vec<String>) -> Result<ConfigUploadResult>;
#[tauri::command] fn config_restore_default() -> Result<()>;
#[tauri::command] fn config_clear() -> Result<()>;

// 报告
#[tauri::command] fn cmd_save_report(report_data: TestReport) -> Result<String>;
#[tauri::command] fn cmd_list_reports(filter: ReportFilter) -> Result<Vec<ReportSummary>>;
#[tauri::command] fn cmd_get_report(report_id: String) -> Result<TestReport>;
#[tauri::command] fn cmd_delete_report(report_id: String) -> Result<()>;
#[tauri::command] fn cmd_export_csv(filter: ReportFilter) -> Result<String>;

// 设置
#[tauri::command] fn cmd_load_settings() -> AppSettings;
#[tauri::command] fn cmd_save_settings(settings_data: AppSettings) -> Result<()>;
#[tauri::command] fn cmd_get_default_test_items() -> Vec<TestItemConfig>;
#[tauri::command] fn cmd_get_data_dir() -> String;

// 模板
#[tauri::command] fn cmd_list_templates() -> Result<Vec<ConfigTemplate>>;
#[tauri::command] fn cmd_save_template(template: ConfigTemplate) -> Result<()>;
#[tauri::command] fn cmd_delete_template(name: String) -> Result<()>;
```

### 7.4 异步事件推送

Rust 后端通过 Tauri Event 向前端推送：

| 事件名 | 数据 | 用途 |
|--------|------|------|
| `serial:data` | `{ line: String }` | 原始串口收发（调试页面） |
| `serial:disconnected` | `{}` | USB 断线通知 |
| `test:progress` | `{ id, status, data }` | 测试进度更新 |
| `key:event` | `{ key, state }` | 按键测试异步上报 |
| `config:progress` | `{ line, total }` | 配置上传进度 |

---

## 8. 产测执行流程

### 8.1 一键自动产测

```
1. 加载最新测试项配置（从设置读取 enabled/retries/timeout/params）
2. AT+PROD=1 → 进入产测模式（最多重试 3 次）
3. AT+MDINFO? → 获取 IMEI/ICCID/FW 等信息（作为报告标识）
4. 按顺序执行所有已启用的自动测试项（judgeType=auto，MCURST 除外）:
   每项按配置的 retries 重试（retries=0 执行 1 次, retries=1 最多 2 次），
   使用配置的 timeout_ms 和 params 进行判定。
   失败不跳过后续项，继续执行。
5. AT+PROD=0 → 退出产测模式
6. 执行 MCURST（恢复出厂）— 放在退出产测模式之后
7. 汇总自动测试结果
8. 若有手动/半自动测试项未完成，提示等待
9. 全部已启用项完成后自动保存报告
```

**报告自动保存**：当所有已启用测试项均达到最终状态（pass/fail/skipped）时自动触发报告保存，覆盖"自动测试完成但手动测试未做"的场景 — 手动项完成后也会触发保存。

### 8.2 判定标准（per-item 可配置）

每个测试项的判定参数通过设置页面独立配置，存储在 `AppSettings.test_items[].params` 中。

```rust
struct TestItemConfig {
    id: String,
    enabled: bool,
    retries: u32,       // 额外重试次数（0=1次, 1=2次）
    timeout_ms: u64,
    params: HashMap<String, serde_json::Value>,
}
```

**信号质量判定（MDSIG）— 双尺度 CSQ**：

设备返回的 CSQ 值有两种尺度：
- **0-31 尺度**（正值）：对比 `csq_min`（默认 10）
- **dBm 尺度**（负值，如 -63）：对比 `rssi_min`（默认 -90）

```rust
let csq_ok = if csq >= 0 { csq >= csq_min } else { csq >= rssi_min };
```

**综合测试判定（MDALL）**：

当 `ping_enabled=false` 时，跳过 PING 和 DNS 子项判定。

**各项默认参数**：

```json
{
  "MDSIG": { "csq_min": 10, "rssi_min": -90, "rsrp_min": -110 },
  "MDALL": { "ping_enabled": true, "ping_host": "8.8.8.8", "ping_count": 3 },
  "MCUVBAT": { "mv_min": 3000, "mv_max": 4500 },
  "MCUKEY": { "keys": ["PTT", "VOL+", "VOL-", "POWER"], "timeout_s": 30 }
}
```

---

## 9. 配置管理流程

### 9.1 推送配置

```
用户编辑/选择配置内容
  ↓
前端校验（非空行必须含 =）
  ↓
AT+CFGSTART
  ↓ 成功
逐行发送 AT+CFGSET="<line>"
  ↓ 每行成功（+CFGSET: LINE=N）→ 更新进度条
  ↓ 某行失败 → 中止，提示错误
AT+CFGSAVE
  ↓ 成功
AT+CFGREAD? → 回读内容
  ↓
自动对比推送内容 vs 回读内容
  ↓ 一致 → ✓ 推送成功
  ↓ 不一致 → ⚠ 警告，显示 diff
```

### 9.2 清空配置

```
AT+CFGSTART → AT+CFGSAVE（不发任何 CFGSET）
```

### 9.3 异常处理

| 异常场景 | 处理方式 |
|---------|---------|
| CFGSTART 后 USB 断线 | 清理前端状态，提示重连 |
| CFGSET 超时 | 重试 1 次，仍失败则中止 |
| CFGSAVE 返回 ERR,FMT | 提示格式错误，检查配置内容 |
| 整体上传超时（60s） | 中止上传，提示重试 |
| CFGREAD 内容与推送不一致 | 高亮差异行，提示可能写入异常 |

---

## 10. 产品 Profile 系统

### 10.1 Profile 结构

```json
{
  "E02T": {
    "product": "E02T",
    "description": "E02T 耳机 POC 产品",
    "capabilities": { "config": true, "production": true, "mcu": true },
    "auto_test_items": [
      "MDSIM", "MDREG", "MDSIG", "MDDATA", "MDALL",
      "MCUBVER", "MCUMAC", "MCUCHG", "MCUVBAT"
    ],
    "manual_test_items": ["MCULED", "MCUFBMIC", "MCUPMIC", "MCUKEY"],
    "final_items": ["MCUGAUGE", "MCUTIME"],
    "dangerous_items": ["MCURST"],
    "key_list": ["PTT", "VOL+", "VOL-", "POWER"],
    "thresholds": {
      "csq_min": 10,
      "rsrp_min": -110,
      "battery_mv_min": 3000,
      "battery_mv_max": 4300,
      "key_test_timeout_s": 30
    }
  },
  "4GCAM": {
    "product": "4GCAM",
    "description": "4G 摄像头产品",
    "capabilities": { "config": true, "production": false, "mcu": false },
    "auto_test_items": ["MDSIM", "MDREG", "MDSIG", "MDDATA"],
    "manual_test_items": [],
    "final_items": [],
    "dangerous_items": [],
    "key_list": [],
    "thresholds": {
      "csq_min": 10,
      "rsrp_min": -110
    }
  }
}
```

### 10.2 动态适配

连接设备后：
1. 发送 `AT+CAP?` 获取 `PRODUCT=xxx`
2. 查找本地 Profile，未找到则使用通用 Profile
3. 根据 Profile 决定：
   - 显示哪些页面（配置/产测）
   - 产测包含哪些测试项
   - 使用哪套判定标准

---

## 11. 数据存储

### 11.1 目录结构

```
~/.esim-production-tool/
├── config.json              # 工具全局配置
├── profiles/
│   ├── E02T.json            # E02T 产品 Profile
│   └── 4GCAM.json           # 4GCAM 产品 Profile
├── templates/
│   ├── default.ini          # 默认配置模板
│   └── factory.ini          # 产线配置模板
├── reports/
│   ├── 2026-06-01/
│   │   ├── 861234567890123_PASS_143045.json
│   │   └── 861234567890124_FAIL_143112.json
│   └── 2026-06-02/
│       └── ...
└── logs/
    └── serial_2026-06-02.log  # 串口通信日志（调试用）
```

### 11.2 报告 JSON 结构

```json
{
  "id": "rpt_20260602_143045_001",
  "timestamp": "2026-06-02T14:30:45",
  "operator": "张三",
  "device": {
    "product": "E02T",
    "imei": "861234567890123",
    "iccid": "89860012345678901234",
    "fw_version": "EC800MCN_LE_R06A08",
    "bt_version": "1.0.3",
    "bt_mac": "AA:BB:CC:DD:EE:FF"
  },
  "overall": "PASS",
  "duration_ms": 15230,
  "items": [
    {
      "id": "MDSIM",
      "name": "SIM 状态",
      "domain": "modem",
      "status": "pass",
      "data": { "status": "READY" },
      "raw": "+MDSIM:READY",
      "duration_ms": 120
    },
    {
      "id": "MCUVBAT",
      "name": "电池电压",
      "domain": "mcu",
      "status": "pass",
      "data": { "mv": 3850 },
      "raw": "+MCUVBAT:MV=3850",
      "duration_ms": 85
    }
  ]
}
```

---

## 12. 项目结构

```
desktop/
├── DESIGN.md                    # 本文档
├── package.json                 # 前端依赖
├── vite.config.ts               # Vite 构建配置
├── tsconfig.json                # TypeScript 配置
├── index.html                   # 入口 HTML
├── src-tauri/                   # Rust 后端
│   ├── Cargo.toml
│   ├── tauri.conf.json          # Tauri 配置
│   ├── icons/                   # 应用图标
│   └── src/
│       ├── main.rs              # Tauri 入口
│       ├── lib.rs               # 模块声明 + 命令注册
│       ├── commands.rs          # IPC 命令实现
│       ├── serial_manager.rs    # 串口管理
│       ├── at_protocol.rs       # AT 协议引擎
│       ├── report.rs            # 报告读写与 CSV 导出
│       ├── settings.rs          # 设置与模板持久化
│       └── types.rs             # 共享数据结构
└── src/                         # Vue 前端
    ├── main.ts                  # 入口
    ├── App.vue                  # 根组件（右键菜单禁用、全局样式）
    ├── router/
    │   └── index.ts             # 路由（5 个页面）
    ├── views/
    │   ├── ProductionView.vue   # 产测页面（统一表格 + 弹框）
    │   ├── ConfigView.vue       # 配置页面
    │   ├── DebugView.vue        # 调试页面
    │   ├── ReportView.vue       # 报告页面
    │   └── SettingsView.vue     # 设置页面（per-item 配置 + 自动保存）
    ├── components/
    │   └── layout/
    │       ├── AppHeader.vue    # 顶部栏（连接状态）
    │       └── AppFooter.vue    # 底部栏（设备信息）
    └── stores/                  # Pinia 状态管理
        ├── device.ts            # 设备连接状态
        ├── production.ts        # 产测状态（测试项 + 日志 + configDirty）
        └── report.ts            # 报告数据
```

---

## 13. 开发计划

| 阶段 | 内容 | 预估工时 |
|------|------|---------|
| **P0 基础骨架** | Tauri+Vue 项目初始化、串口扫描/连接/断开、AT+CAP? 能力发现、调试页面（手动收发 AT 命令） | 3-4 天 |
| **P1 产测核心** | 产测模式控制、模组自测命令、MCU 命令、一键自动测试、PASS/FAIL 判定 | 4-5 天 |
| **P2 配置功能** | 配置编辑器、模板管理、推送/回读/校验/恢复/清空 | 2-3 天 |
| **P3 报告系统** | 报告生成/存储/查询/导出 CSV | 2-3 天 |
| **P4 人工交互** | LED/麦克风人工判定、按键测试异步上报、确认弹窗 | 2 天 |
| **P5 打包发布** | Windows .msi + Linux .AppImage 打包、图标、安装测试 | 1-2 天 |
| **P6 产线增强** | 条码扫描集成、批量统计、操作员登录（按需） | 按需 |

---

## 14. 注意事项

### 14.1 安全性

- `AT+MCURST`（恢复出厂）需二次确认弹窗，放在产测流程最后
- 正式产品固件建议通过 `AT+CAP?` 返回 `PRODUCTION=0` 关闭产测入口
- 报告中不存储完整 ICCID（脱敏处理）

### 14.2 可靠性

- USB 断线时全局通知，清理所有进行中状态
- MCU 命令串行执行，禁止并发发送
- 配置上传使用 tmp→bak→ini 三级机制（设备侧），上位机侧做回读校验

### 14.3 可维护性

- 新增测试项只需在 Profile JSON 中添加，无需修改代码
- 错误码映射表集中管理
- 超时参数可在设置页面调整，无需重新编译
