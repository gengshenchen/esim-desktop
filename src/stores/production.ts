import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useDeviceStore } from './device'

export type TestStatus = 'pending' | 'running' | 'pass' | 'fail' | 'skipped' | 'manual_pending'

export interface TestItem {
  id: string
  name: string
  domain: 'modem' | 'mcu'
  judgeType: 'auto' | 'manual' | 'semi_auto'
  status: TestStatus
  rawResponse: string
  parsedData: Record<string, string>
  error: string
  durationMs: number
}

export interface TestItemConfig {
  id: string
  enabled: boolean
  retries: number
  timeout_ms: number
  params: Record<string, any>
}

export interface LogEntry {
  time: string
  level: 'info' | 'success' | 'error' | 'warn'
  message: string
}

export type KeyState = 'untested' | 'pressed' | 'passed' | 'stuck'

export interface KeyInfo {
  name: string
  label: string
  state: KeyState
  pressedAt: number
}

const ALL_MODEM_ITEMS: Omit<TestItem, 'status' | 'rawResponse' | 'parsedData' | 'error' | 'durationMs'>[] = [
  { id: 'MDSIM', name: 'SIM 状态', domain: 'modem', judgeType: 'auto' },
  { id: 'MDREG', name: '网络注册', domain: 'modem', judgeType: 'auto' },
  { id: 'MDSIG', name: '信号质量', domain: 'modem', judgeType: 'auto' },
  { id: 'MDDATA', name: '数据业务', domain: 'modem', judgeType: 'auto' },
  { id: 'MDALL', name: '综合测试', domain: 'modem', judgeType: 'auto' },
]

const ALL_MCU_ITEMS: Omit<TestItem, 'status' | 'rawResponse' | 'parsedData' | 'error' | 'durationMs'>[] = [
  { id: 'MCUBVER', name: '蓝牙版本', domain: 'mcu', judgeType: 'auto' },
  { id: 'MCUMAC', name: '蓝牙 MAC', domain: 'mcu', judgeType: 'auto' },
  { id: 'MCUCHG', name: '充电信息', domain: 'mcu', judgeType: 'auto' },
  { id: 'MCUVBAT', name: '电池电压', domain: 'mcu', judgeType: 'auto' },
  { id: 'MCULED', name: 'LED 测试', domain: 'mcu', judgeType: 'manual' },
  { id: 'MCUFBMIC', name: 'FB 麦回环', domain: 'mcu', judgeType: 'manual' },
  { id: 'MCUPMIC', name: '主麦回环', domain: 'mcu', judgeType: 'manual' },
  { id: 'MCUKEY', name: '按键测试', domain: 'mcu', judgeType: 'semi_auto' },
  { id: 'MCUGAUGE', name: '电量计校准', domain: 'mcu', judgeType: 'auto' },
  { id: 'MCUTIME', name: '时间同步', domain: 'mcu', judgeType: 'auto' },
  { id: 'MCURST', name: '恢复出厂', domain: 'mcu', judgeType: 'auto' },
]

function toTestItem(def: typeof ALL_MODEM_ITEMS[0]): TestItem {
  return { ...def, status: 'pending', rawResponse: '', parsedData: {}, error: '', durationMs: 0 }
}

function now(): string {
  const d = new Date()
  return d.toTimeString().split(' ')[0]
}

function isoNow(): string {
  const d = new Date()
  const pad = (n: number) => n.toString().padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}

function genReportId(): string {
  const d = new Date()
  const pad = (n: number) => n.toString().padStart(2, '0')
  return `rpt_${d.getFullYear()}${pad(d.getMonth() + 1)}${pad(d.getDate())}_${pad(d.getHours())}${pad(d.getMinutes())}${pad(d.getSeconds())}`
}

export const useProductionStore = defineStore('production', () => {
  const modemItems = ref<TestItem[]>([])
  const mcuItems = ref<TestItem[]>([])
  const running = ref(false)
  const logs = ref<LogEntry[]>([])
  const testItemConfigs = ref<TestItemConfig[]>([])
  const reportSaved = ref(false)
  const currentReportId = ref('')
  let lastReportSnapshot = ''
  let savePending = false

  // Key test state
  const keyTestActive = ref(false)
  const keyInfos = ref<KeyInfo[]>([])
  let keyTestUnlisten: (() => void) | null = null
  let stuckTimer: ReturnType<typeof setInterval> | null = null
  let keyTestStartTime = 0

  const configDirty = ref(false)

  const enabledIds = computed(() => {
    const cfgMap = new Map(testItemConfigs.value.map(c => [c.id, c]))
    const allIds = [...ALL_MODEM_ITEMS, ...ALL_MCU_ITEMS].map(i => i.id)
    return allIds.filter(id => {
      const cfg = cfgMap.get(id)
      return !cfg || cfg.enabled
    })
  })

  function markConfigDirty() {
    configDirty.value = true
  }

  async function loadTestItemConfigs() {
    try {
      const settings = await invoke<{ test_items: TestItemConfig[] }>('cmd_load_settings')
      testItemConfigs.value = settings.test_items || []
    } catch {
      testItemConfigs.value = []
    }

    const cfgMap = new Map(testItemConfigs.value.map(c => [c.id, c]))

    modemItems.value = ALL_MODEM_ITEMS
      .filter(def => { const c = cfgMap.get(def.id); return !c || c.enabled })
      .map(toTestItem)

    mcuItems.value = ALL_MCU_ITEMS
      .filter(def => { const c = cfgMap.get(def.id); return !c || c.enabled })
      .map(toTestItem)

    reportSaved.value = false
    currentReportId.value = ''
    lastReportSnapshot = ''
    configDirty.value = false
  }

  function reloadIfDirty() {
    if (configDirty.value) {
      loadTestItemConfigs()
    }
  }

  function addLog(level: LogEntry['level'], message: string) {
    logs.value.push({ time: now(), level, message })
    if (logs.value.length > 200) logs.value.splice(0, logs.value.length - 200)
  }

  function clearLogs() {
    logs.value = []
  }

  function resetAll() {
    const cfgMap = new Map(testItemConfigs.value.map(c => [c.id, c]))
    modemItems.value = ALL_MODEM_ITEMS
      .filter(def => { const c = cfgMap.get(def.id); return !c || c.enabled })
      .map(toTestItem)
    mcuItems.value = ALL_MCU_ITEMS
      .filter(def => { const c = cfgMap.get(def.id); return !c || c.enabled })
      .map(toTestItem)
    reportSaved.value = false
    currentReportId.value = ''
    lastReportSnapshot = ''
  }

  function updateItem(id: string, updates: Partial<TestItem>) {
    const all = [...modemItems.value, ...mcuItems.value]
    const item = all.find((i) => i.id === id)
    if (item) {
      Object.assign(item, updates)
      checkAutoSaveReport()
    }
  }

  function findItem(id: string): TestItem | undefined {
    return [...modemItems.value, ...mcuItems.value].find(i => i.id === id)
  }

  function checkAutoSaveReport() {
    if (savePending) return
    const allItems = [...modemItems.value, ...mcuItems.value]
    const nonRstItems = allItems.filter(i => i.id !== 'MCURST')
    const rstItem = allItems.find(i => i.id === 'MCURST')

    const testedItems = allItems.filter(i => i.status === 'pass' || i.status === 'fail')
    if (testedItems.length === 0) return

    const nonRstDone = nonRstItems.every(i => i.status === 'pass' || i.status === 'fail' || i.status === 'skipped')
    if (!nonRstDone) return

    if (rstItem && rstItem.status === 'pending') {
      savePending = true
      runFactoryReset().finally(() => { savePending = false })
      return
    }

    const allDone = allItems.every(i => i.status === 'pass' || i.status === 'fail' || i.status === 'skipped')
    if (!allDone) return

    const snapshot = testedItems.map(i => `${i.id}:${i.status}`).join(',')
    if (snapshot === lastReportSnapshot) return
    lastReportSnapshot = snapshot

    const passCount = testedItems.filter(i => i.status === 'pass').length
    const failCount = testedItems.filter(i => i.status === 'fail').length
    savePending = true
    saveReport(passCount, failCount).finally(() => { savePending = false })
  }

  async function runFactoryReset() {
    const device = useDeviceStore()

    addLog('info', `测试 [恢复出厂]`)
    updateItem('MCURST', { status: 'running', error: '', rawResponse: '' })
    try {
      const result = await invoke<SingleTestResult>('run_single_test', { testId: 'MCURST' })

      addLog('info', `TX: ${result.command}`)
      addLog('info', `RX: ${result.raw_response || '(空)'}`)

      updateItem('MCURST', {
        status: result.status as TestStatus,
        rawResponse: result.raw_response,
        parsedData: result.parsed_data,
        error: result.error,
        durationMs: result.duration_ms,
      })
      if (result.status === 'pass') {
        addLog('success', `[恢复出厂] PASS (${result.duration_ms}ms)`)
      } else {
        addLog('error', `[恢复出厂] FAIL: ${result.error}`)
      }
    } catch (e: any) {
      updateItem('MCURST', { status: 'fail', error: String(e) })
      addLog('error', `[恢复出厂] 异常: ${e}`)
    }

    if (device.productionMode === 'production') {
      addLog('info', '退出产测模式 AT+PROD=0 ...')
      try {
        await device.exitProductionMode()
        addLog('success', '已退出产测模式')
      } catch (e) {
        device.productionMode = 'idle'
        addLog('warn', `退出产测模式异常: ${e}`)
      }
    }
  }

  async function queryDeviceInfo() {
    const device = useDeviceStore()
    try {
      const data = await invoke<Record<string, string>>('query_device_info')
      addLog('info', `设备信息: ${JSON.stringify(data)}`)
      device.deviceInfo.imei = data.IMEI || data.imei || ''
      device.deviceInfo.iccid = data.ICCID || data.iccid || ''
      if (!device.deviceInfo.fwVersion) {
        device.deviceInfo.fwVersion = data.VER || data.FW || data.ver || data.VERSION || ''
      }
    } catch (e) {
      addLog('warn', `查询设备信息失败: ${e}`)
    }
  }

  async function saveReport(passCount: number, failCount: number) {
    const device = useDeviceStore()
    const allItems = [...modemItems.value, ...mcuItems.value]
    const testedItems = allItems.filter(i => i.status === 'pass' || i.status === 'fail')
    const overall = failCount === 0 && passCount > 0 ? 'PASS' : 'FAIL'
    const totalDuration = testedItems.reduce((sum, i) => sum + (i.durationMs || 0), 0)

    if (!currentReportId.value) {
      currentReportId.value = genReportId()
    }

    if (!device.deviceInfo.imei) {
      await queryDeviceInfo()
    }

    let operator = ''
    try {
      const settings = await invoke<{ operator: string }>('cmd_load_settings')
      operator = settings.operator || ''
    } catch { /* ignore */ }

    const btItem = findItem('MCUBVER')
    const macItem = findItem('MCUMAC')

    const report = {
      id: currentReportId.value,
      timestamp: isoNow(),
      operator,
      device: {
        product: device.capability?.product || 'UNKNOWN',
        imei: device.deviceInfo.imei || '',
        iccid: device.deviceInfo.iccid || '',
        fw_version: device.deviceInfo.fwVersion || '',
        bt_version: btItem?.parsedData?.VER || btItem?.rawResponse || '',
        bt_mac: macItem?.parsedData?.MAC || macItem?.rawResponse || '',
      },
      overall,
      duration_ms: totalDuration,
      items: testedItems.map(item => ({
        id: item.id,
        name: item.name,
        domain: item.domain,
        status: item.status,
        data: item.parsedData,
        raw: item.rawResponse,
        duration_ms: item.durationMs,
      })),
    }

    const isUpdate = reportSaved.value
    try {
      await invoke<string>('cmd_save_report', { reportData: report })
      reportSaved.value = true
      addLog('success', isUpdate
        ? `报告已更新: ${report.id} (${overall}, ${passCount}pass/${failCount}fail)`
        : `报告已保存: ${report.id}`)
    } catch (e) {
      addLog('error', `报告保存失败: ${e}`)
    }
  }

  // --- Key test state machine ---

  function getKeyTestParams() {
    const cfgMap = new Map(testItemConfigs.value.map(c => [c.id, c]))
    const keyCfg = cfgMap.get('MCUKEY')
    const timeoutS = keyCfg?.params?.timeout_s ?? 30
    const keyTimeoutS = keyCfg?.params?.key_timeout_s ?? 10
    return { timeoutS, keyTimeoutS }
  }

  function initKeyInfos() {
    const device = useDeviceStore()
    const keys = device.productConfig?.key_test?.keys || []
    if (keys.length > 0) {
      keyInfos.value = keys.map(k => ({
        name: k.name,
        label: k.label,
        state: 'untested' as KeyState,
        pressedAt: 0,
      }))
    } else {
      keyInfos.value = [
        { name: 'MULTI_FUN', label: '多功能键', state: 'untested', pressedAt: 0 },
        { name: 'VOL_UP', label: '音量+', state: 'untested', pressedAt: 0 },
        { name: 'VOL_DOWN', label: '音量-/关机键', state: 'untested', pressedAt: 0 },
      ]
    }
  }

  function handleKeyEvent(keyName: string, state: string) {
    const info = keyInfos.value.find(k => k.name === keyName)
    if (!info) {
      addLog('warn', `收到未配置的按键: ${keyName}`)
      return
    }

    if (state === 'PRESS') {
      if (info.state === 'untested') {
        info.state = 'pressed'
        info.pressedAt = Date.now()
        addLog('info', `按键检测: ${info.label} 按下`)
      } else if (info.state === 'pressed') {
        info.pressedAt = Date.now()
      }
    } else if (state === 'RELEASE') {
      if (info.state === 'pressed') {
        info.state = 'passed'
        addLog('info', `按键检测: ${info.label} 释放 - 通过`)
      } else if (info.state === 'untested') {
        addLog('warn', `按键检测: ${info.label} 释放但未按下，忽略`)
      }
    }
  }

  function checkKeyTestCompletion(): { done: boolean; passed: boolean; reason: string } {
    const { timeoutS, keyTimeoutS } = getKeyTestParams()
    const nowMs = Date.now()

    // Check STUCK
    for (const k of keyInfos.value) {
      if (k.state === 'pressed' && (nowMs - k.pressedAt) > keyTimeoutS * 1000) {
        k.state = 'stuck'
        addLog('warn', `按键检测: ${k.label} 卡住 (超过${keyTimeoutS}s无释放)`)
      }
    }

    const allTerminal = keyInfos.value.every(k => k.state === 'passed' || k.state === 'stuck')
    const allPassed = keyInfos.value.every(k => k.state === 'passed')
    const hasStuck = keyInfos.value.some(k => k.state === 'stuck')

    if (allPassed) {
      return { done: true, passed: true, reason: '' }
    }

    if (allTerminal && hasStuck) {
      const stuckNames = keyInfos.value.filter(k => k.state === 'stuck').map(k => k.label).join(', ')
      return { done: true, passed: false, reason: `${stuckNames} 卡住` }
    }

    // Total timeout
    if ((nowMs - keyTestStartTime) > timeoutS * 1000) {
      const incomplete = keyInfos.value
        .filter(k => k.state !== 'passed')
        .map(k => k.label)
        .join(', ')
      return { done: true, passed: false, reason: `超时，未通过: ${incomplete}` }
    }

    return { done: false, passed: false, reason: '' }
  }

  async function finishKeyTest(passed: boolean, reason: string) {
    if (!keyTestActive.value) return
    keyTestActive.value = false

    if (keyTestUnlisten) {
      keyTestUnlisten()
      keyTestUnlisten = null
    }
    if (stuckTimer) {
      clearInterval(stuckTimer)
      stuckTimer = null
    }

    try {
      await invoke('send_at_command', { cmd: 'AT+MCUKEY=0', timeoutMs: 3000 })
    } catch {
      // MCU 60s auto-exit
    }

    const durationMs = Date.now() - keyTestStartTime
    const parsedData: Record<string, string> = {}
    for (const k of keyInfos.value) parsedData[k.name] = k.state
    const rawResponse = keyInfos.value.map(k => `${k.name}:${k.state}`).join(' ')

    updateItem('MCUKEY', {
      status: passed ? 'pass' : 'fail',
      durationMs,
      parsedData,
      rawResponse,
      error: reason,
    })

    addLog(
      passed ? 'success' : 'error',
      `[按键测试] ${passed ? 'PASS' : 'FAIL: ' + reason} (${(durationMs / 1000).toFixed(1)}s)`,
    )
  }

  async function startKeyTestLoop() {
    keyTestStartTime = Date.now()
    keyTestActive.value = true

    try {
      keyTestUnlisten = (await listen<{ key: string; state: string }>('key:event', (event) => {
        if (!keyTestActive.value) return
        handleKeyEvent(event.payload.key, event.payload.state)
      })) as unknown as () => void
    } catch {
      // event not available
    }

    stuckTimer = setInterval(() => {
      if (!keyTestActive.value) return
      const result = checkKeyTestCompletion()
      if (result.done) {
        finishKeyTest(result.passed, result.reason)
      }
    }, 500)
  }

  function stopKeyTestCleanup() {
    keyTestActive.value = false
    if (keyTestUnlisten) {
      keyTestUnlisten()
      keyTestUnlisten = null
    }
    if (stuckTimer) {
      clearInterval(stuckTimer)
      stuckTimer = null
    }
  }

  interface SingleTestResult {
    id: string
    command: string
    status: string
    raw_response: string
    parsed_data: Record<string, string>
    error: string
    duration_ms: number
  }

  async function runSingleTest(testId: string) {
    const device = useDeviceStore()
    if (device.productionMode !== 'production') {
      addLog('info', '进入产测模式...')
      try {
        await device.enterProductionMode()
        addLog('success', '已进入产测模式')
      } catch (e: any) {
        addLog('error', `进入产测模式失败: ${e}`)
        updateItem(testId, { status: 'fail', error: `进入产测模式失败: ${e}` })
        return
      }
    }

    const item = findItem(testId)
    updateItem(testId, { status: 'running', error: '', rawResponse: '' })
    try {
      const result = await invoke<SingleTestResult>('run_single_test', { testId })

      addLog('info', `TX: ${result.command}`)
      addLog('info', `RX: ${result.raw_response || '(空)'}`)

      updateItem(testId, {
        status: result.status as TestStatus,
        rawResponse: result.raw_response,
        parsedData: result.parsed_data,
        error: result.error,
        durationMs: result.duration_ms,
      })

      if (result.status === 'pass') {
        addLog('success', `[${item?.name || testId}] PASS (${result.duration_ms}ms)`)
      } else if (result.status === 'manual_pending') {
        addLog('info', `[${item?.name || testId}] 等待人工判定`)
      } else {
        addLog('error', `[${item?.name || testId}] FAIL: ${result.error}`)
      }
    } catch (e: any) {
      updateItem(testId, { status: 'fail', error: String(e) })
      addLog('error', `[${item?.name || testId}] 异常: ${e}`)
    }
  }

  async function runAutoTest() {
    const device = useDeviceStore()
    const startTime = Date.now()
    running.value = true
    reportSaved.value = false
    resetAll()
    clearLogs()

    addLog('info', '====== 一键产测开始 ======')

    await loadTestItemConfigs()

    addLog('info', '进入产测模式 AT+PROD=1 ...')

    try {
      await device.enterProductionMode()
      addLog('success', '已进入产测模式')
    } catch (e: any) {
      addLog('error', `进入产测模式失败: ${e}`)
      running.value = false
      return
    }

    await queryDeviceInfo()

    const autoItems = [...modemItems.value, ...mcuItems.value]
      .filter(i => i.judgeType === 'auto' && i.id !== 'MCURST')

    const testIds = autoItems.map(i => i.id)

    for (const item of autoItems) {
      updateItem(item.id, { status: 'running', error: '', rawResponse: '' })
    }

    let passCount = 0
    let failCount = 0

    const unlisten = await listen<SingleTestResult>('test:result', (event) => {
      const result = event.payload
      const item = findItem(result.id)

      addLog('info', `TX: ${result.command}`)
      addLog('info', `RX: ${result.raw_response || '(空)'}`)

      updateItem(result.id, {
        status: result.status as TestStatus,
        rawResponse: result.raw_response,
        parsedData: result.parsed_data,
        error: result.error,
        durationMs: result.duration_ms,
      })

      if (result.status === 'pass') {
        passCount++
        addLog('success', `[${item?.name || result.id}] PASS (${result.duration_ms}ms)`)
      } else {
        failCount++
        addLog('error', `[${item?.name || result.id}] FAIL: ${result.error}`)
      }
    })

    try {
      await invoke('run_auto_test', { testIds })
    } catch (e: any) {
      addLog('error', `批量测试异常: ${e}`)
    }

    ;(unlisten as unknown as () => void)()

    const totalDuration = Date.now() - startTime
    addLog('info', `====== 自动测试完成: 通过 ${passCount}, 失败 ${failCount}, 耗时 ${(totalDuration / 1000).toFixed(1)}s ======`)

    const manualItems = [...modemItems.value, ...mcuItems.value].filter(
      i => (i.judgeType === 'manual' || i.judgeType === 'semi_auto') && i.status === 'pending'
    )
    const rstItem = findItem('MCURST')

    if (manualItems.length > 0) {
      addLog('info', `还有 ${manualItems.length} 项手动测试待完成，全部完成后自动执行恢复出厂并保存报告`)
    } else if (rstItem) {
      // No manual items — MCURST + exit will be triggered by checkAutoSaveReport
    } else {
      // No manual items and no MCURST — exit production and save now
      addLog('info', '退出产测模式 AT+PROD=0 ...')
      try {
        await device.exitProductionMode()
        addLog('success', '已退出产测模式')
      } catch (e) {
        device.productionMode = 'idle'
        addLog('warn', `退出产测模式异常: ${e}`)
      }
      await saveReport(passCount, failCount)
    }

    running.value = false
  }

  async function manualJudge(testId: string, pass: boolean) {
    const item = findItem(testId)
    updateItem(testId, { status: pass ? 'pass' : 'fail' })
    addLog(pass ? 'success' : 'error', `[${item?.name || testId}] 人工判定: ${pass ? 'PASS' : 'FAIL'}`)
    try {
      await invoke('manual_judge', { testId, pass })
    } catch (e) {
      console.error('manual_judge failed:', e)
    }
  }

  return {
    modemItems,
    mcuItems,
    running,
    logs,
    testItemConfigs,
    keyTestActive,
    keyInfos,
    enabledIds,
    resetAll,
    clearLogs,
    addLog,
    updateItem,
    findItem,
    configDirty,
    markConfigDirty,
    reloadIfDirty,
    loadTestItemConfigs,
    runSingleTest,
    runAutoTest,
    manualJudge,
    queryDeviceInfo,
    initKeyInfos,
    startKeyTestLoop,
    finishKeyTest,
    stopKeyTestCleanup,
    getKeyTestParams,
  }
})
