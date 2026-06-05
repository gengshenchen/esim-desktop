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

```
┌─ 操作栏 ───────────────────────────────────────────────────┐
│  [▶ 一键产测]  [停止]        进度: 8/14  通过: 7  失败: 1   │
└────────────────────────────────────────────────────────────┘

┌─ 模组域测试 ────────────────────────────────────────────────┐
│  失败时排查方向: SIM / 网络 / 协议栈 / 配置解析              │
│                                                              │
│  #  测试项        结果                   耗时    状态        │
│  1  SIM 状态      READY                  120ms   ✓ PASS     │
│  2  网络注册      CREG=1,CGREG=1,RAT=LTE 85ms   ✓ PASS     │
│  3  信号质量      CSQ=18,RSRP=-92        90ms    ✓ PASS     │
│  4  数据业务      UP,IP=10.21.33.8       110ms   ✓ PASS     │
│  5  综合测试      SIM=OK,REG=OK,...      2.5s    ✓ PASS     │
└──────────────────────────────────────────────────────────────┘

┌─ MCU 域测试 ────────────────────────────────────────────────┐
│  失败时排查方向: MCU 固件 / Main UART 链路                    │
│                                                              │
│  #  测试项        结果                   耗时    状态        │
│  6  蓝牙版本      VER=1.0.3              80ms    ✓ PASS     │
│  7  蓝牙 MAC      AA:BB:CC:DD:EE:FF      85ms    ✓ PASS     │
│  8  充电信息      ON=1,ST=CHG,FULL=0     90ms    ✓ PASS     │
│  9  电池电压      3850 mV                75ms    ✓ PASS     │
│  10 LED 测试      [开始][停止]           —       ○ 待人工    │
│  11 FB麦回环      [开始][停止]           —       ○ 待人工    │
│  12 主麦回环      [开始][停止]           —       ○ 待人工    │
│  13 按键测试      [开始] 进度: 2/4       —       ◐ 测试中   │
│  14 电量计校准    —                      —       ○ 待执行    │
└──────────────────────────────────────────────────────────────┘
```

**人工判定项**：LED/麦克风回环执行后弹出确认框 `[PASS] [FAIL]`，由操作员目视/听觉判定。

**按键测试**：

```
┌─ 按键测试 ─────────────────────────────────┐
│                                             │
│   ┌─────┐  ┌─────┐  ┌─────┐  ┌────────┐   │
│   │ PTT │  │VOL+ │  │VOL- │  │ POWER  │   │
│   │  ●  │  │  ●  │  │  ○  │  │   ○    │   │
│   └─────┘  └─────┘  └─────┘  └────────┘   │
│                                             │
│   ● 已检测(绿)  ○ 未检测(灰)               │
│   进度: 2/4    超时倒计时: 25s              │
│                                             │
│   [跳过]  [手动PASS]  [FAIL]                │
└─────────────────────────────────────────────┘
```

异步事件 `+MCUKEY:KEY=PTT,STATE=PRESS` 到达时对应按键变绿。全部检测完自动 PASS。超时未全部完成则 FAIL。

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

- 串口参数（波特率、默认端口）
- 超时参数（每条命令独立配置）
- 判定标准（信号阈值、电池电压范围）
- 产品 Profile 管理
- 按键测试按键列表配置
- 模板管理
- 操作员信息

---

## 7. Rust 后端模块设计

### 7.1 模块划分

```
src-tauri/src/
├── main.rs              # Tauri 应用入口
├── commands.rs          # Tauri IPC 命令注册
├── serial_manager.rs    # 串口管理（扫描/连接/读写）
├── at_protocol.rs       # AT 协议引擎（发送/解析/超时）
├── test_runner.rs       # 产测执行器（自动化流程）
├── config_service.rs    # 配置管理（上传/回读/对比）
├── report.rs            # 报告生成与存储
├── error.rs             # 统一错误类型
└── types.rs             # 共享数据结构
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

// AT 命令
struct ATCommand {
    raw: String,           // "AT+MDSIM?\r\n"
    timeout_ms: u64,       // 2000
}

// AT 响应
struct ATResponse {
    lines: Vec<String>,    // ["+MDSIM:READY"]
    success: bool,         // OK=true, ERROR=false
    error_code: Option<String>,  // "TO", "MODE", etc.
}

// 产测模式状态
enum ProductionState {
    Idle,
    Production,
}

// 测试项状态
enum TestStatus {
    Pending,
    Running,
    Pass,
    Fail { error: String },
    Skipped,
    ManualPending,  // 等待人工判定
}

// 单项测试结果
struct TestResult {
    id: String,
    name: String,
    domain: TestDomain,    // Modem / MCU
    status: TestStatus,
    raw_response: String,
    parsed_data: HashMap<String, String>,
    duration_ms: u64,
    timestamp: String,
}

// 测试报告
struct TestReport {
    id: String,
    imei: String,
    product: String,
    fw_version: String,
    bt_version: String,
    bt_mac: String,
    operator: String,
    timestamp: String,
    overall: OverallResult,  // Pass / Fail
    items: Vec<TestResult>,
}
```

### 7.3 IPC 命令接口

```rust
// 串口管理
#[tauri::command] fn scan_ports() -> Vec<PortInfo>;
#[tauri::command] fn connect(port: String) -> Result<DeviceCapability>;
#[tauri::command] fn disconnect() -> Result<()>;
#[tauri::command] fn get_connection_state() -> ConnectionState;

// AT 命令（底层）
#[tauri::command] fn send_at_command(cmd: String, timeout_ms: u64) -> Result<ATResponse>;

// 产测控制
#[tauri::command] fn enter_production_mode() -> Result<ATResponse>;
#[tauri::command] fn exit_production_mode() -> Result<ATResponse>;
#[tauri::command] fn get_production_state() -> ProductionState;

// 产测执行
#[tauri::command] fn run_single_test(test_id: String) -> Result<TestResult>;
#[tauri::command] fn run_auto_test(items: Vec<String>) -> Result<Vec<TestResult>>;
#[tauri::command] fn manual_judge(test_id: String, pass: bool) -> Result<()>;

// 配置管理
#[tauri::command] fn config_read() -> Result<Vec<String>>;
#[tauri::command] fn config_info() -> Result<ConfigInfo>;
#[tauri::command] fn config_upload(lines: Vec<String>) -> Result<ConfigUploadResult>;
#[tauri::command] fn config_restore_default() -> Result<()>;
#[tauri::command] fn config_clear() -> Result<()>;

// MCU 透传
#[tauri::command] fn mcu_raw_command(cmd: String) -> Result<ATResponse>;

// 报告
#[tauri::command] fn save_report(report: TestReport) -> Result<String>;
#[tauri::command] fn list_reports(filter: ReportFilter) -> Result<Vec<ReportSummary>>;
#[tauri::command] fn export_reports(ids: Vec<String>, format: ExportFormat) -> Result<String>;
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
1. 检查连接状态
2. AT+PROD=1 → 进入产测模式
3. AT+MDINFO? → 获取 IMEI/ICCID/FW 等信息（作为报告标识）
4. 模组域自动测试:
   a. AT+MDSIM? → 判定: READY
   b. AT+MDREG? → 判定: CREG=1 且 CGREG=1
   c. AT+MDSIG? → 判定: CSQ>=10 且 RSRP>=-110
   d. AT+MDDATA? → 判定: STATE=UP 且 IP 非空
   e. AT+MDALL? → 判定: 所有子项 =OK
5. MCU 域自动测试:
   a. AT+MCUBVER? → 判定: 返回非空版本号
   b. AT+MCUMAC? → 判定: MAC 格式合法 (XX:XX:XX:XX:XX:XX)
   c. AT+MCUCHG? → 判定: 返回成功
   d. AT+MCUVBAT? → 判定: 3000 <= MV <= 4300
6. MCU 域人工测试（按产品 Profile 决定是否包含）:
   a. AT+MCULED=1 → 操作员目视确认 → [PASS/FAIL]
   b. AT+MCUFBMIC=1 → 操作员听觉确认 → [PASS/FAIL]
   c. AT+MCUPMIC=1 → 操作员听觉确认 → [PASS/FAIL]
   d. AT+MCUKEY → 等待全部按键上报 → 自动/超时判定
7. MCU 域收尾测试:
   a. AT+MCUGAUGE → 电量计校准
   b. AT+MCUTIME="<当前时间>" → 时间同步
8. AT+PROD=0 → 退出产测模式
9. 汇总结果 → 生成报告 → 保存
```

### 8.2 判定标准（可配置）

```json
{
  "sim": { "expected": "READY" },
  "reg": { "creg": 1, "cgreg": 1 },
  "signal": { "csq_min": 10, "rsrp_min": -110 },
  "battery": { "mv_min": 3000, "mv_max": 4300 },
  "mac": { "regex": "^[0-9A-Fa-f]{2}(:[0-9A-Fa-f]{2}){5}$" },
  "key_test": {
    "keys": ["PTT", "VOL+", "VOL-", "POWER"],
    "timeout_s": 30
  }
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
│       ├── main.rs
│       ├── lib.rs
│       ├── commands.rs          # IPC 命令注册
│       ├── serial_manager.rs    # 串口管理
│       ├── at_protocol.rs       # AT 协议引擎
│       ├── test_runner.rs       # 产测执行器
│       ├── config_service.rs    # 配置管理
│       ├── report.rs            # 报告生成
│       ├── error.rs             # 错误类型
│       └── types.rs             # 共享数据结构
└── src/                         # Vue 前端
    ├── main.ts                  # 入口
    ├── App.vue                  # 根组件
    ├── router/
    │   └── index.ts             # 路由
    ├── views/
    │   ├── ProductionView.vue   # 产测页面
    │   ├── ConfigView.vue       # 配置页面
    │   ├── DebugView.vue        # 调试页面
    │   ├── ReportView.vue       # 报告页面
    │   └── SettingsView.vue     # 设置页面
    ├── components/
    │   ├── layout/
    │   │   ├── AppHeader.vue    # 顶部栏（连接状态）
    │   │   └── AppFooter.vue    # 底部栏（设备信息）
    │   ├── production/
    │   │   ├── TestItemCard.vue # 单项测试卡片
    │   │   ├── KeyTestPanel.vue # 按键测试面板
    │   │   └── ManualJudge.vue  # 人工判定弹窗
    │   ├── config/
    │   │   ├── ConfigEditor.vue # 配置编辑器
    │   │   ├── ConfigDiff.vue   # 对比视图
    │   │   └── TemplateSelector.vue
    │   └── debug/
    │       └── TerminalLog.vue  # 终端日志
    ├── stores/                  # Pinia 状态管理
    │   ├── device.ts            # 设备连接状态
    │   ├── production.ts        # 产测状态
    │   ├── config.ts            # 配置状态
    │   └── report.ts            # 报告数据
    ├── lib/
    │   ├── tauri-serial.ts      # Tauri IPC 封装
    │   ├── at-parser.ts         # AT 响应解析
    │   ├── error-codes.ts       # 错误码映射
    │   └── validators.ts        # 判定标准
    └── assets/
        └── styles/
            └── main.css
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
