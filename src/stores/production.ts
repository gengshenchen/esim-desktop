import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
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

export interface LogEntry {
  time: string
  level: 'info' | 'success' | 'error' | 'warn'
  message: string
}

function createModemItems(): TestItem[] {
  return [
    { id: 'MDSIM', name: 'SIM 状态', domain: 'modem', judgeType: 'auto', status: 'pending', rawResponse: '', parsedData: {}, error: '', durationMs: 0 },
    { id: 'MDREG', name: '网络注册', domain: 'modem', judgeType: 'auto', status: 'pending', rawResponse: '', parsedData: {}, error: '', durationMs: 0 },
    { id: 'MDSIG', name: '信号质量', domain: 'modem', judgeType: 'auto', status: 'pending', rawResponse: '', parsedData: {}, error: '', durationMs: 0 },
    { id: 'MDDATA', name: '数据业务', domain: 'modem', judgeType: 'auto', status: 'pending', rawResponse: '', parsedData: {}, error: '', durationMs: 0 },
    { id: 'MDALL', name: '综合测试', domain: 'modem', judgeType: 'auto', status: 'pending', rawResponse: '', parsedData: {}, error: '', durationMs: 0 },
  ]
}

function createMcuItems(): TestItem[] {
  return [
    { id: 'MCUBVER', name: '蓝牙版本', domain: 'mcu', judgeType: 'auto', status: 'pending', rawResponse: '', parsedData: {}, error: '', durationMs: 0 },
    { id: 'MCUMAC', name: '蓝牙 MAC', domain: 'mcu', judgeType: 'auto', status: 'pending', rawResponse: '', parsedData: {}, error: '', durationMs: 0 },
    { id: 'MCUCHG', name: '充电信息', domain: 'mcu', judgeType: 'auto', status: 'pending', rawResponse: '', parsedData: {}, error: '', durationMs: 0 },
    { id: 'MCUVBAT', name: '电池电压', domain: 'mcu', judgeType: 'auto', status: 'pending', rawResponse: '', parsedData: {}, error: '', durationMs: 0 },
    { id: 'MCULED', name: 'LED 测试', domain: 'mcu', judgeType: 'manual', status: 'pending', rawResponse: '', parsedData: {}, error: '', durationMs: 0 },
    { id: 'MCUFBMIC', name: 'FB 麦回环', domain: 'mcu', judgeType: 'manual', status: 'pending', rawResponse: '', parsedData: {}, error: '', durationMs: 0 },
    { id: 'MCUPMIC', name: '主麦回环', domain: 'mcu', judgeType: 'manual', status: 'pending', rawResponse: '', parsedData: {}, error: '', durationMs: 0 },
    { id: 'MCUKEY', name: '按键测试', domain: 'mcu', judgeType: 'semi_auto', status: 'pending', rawResponse: '', parsedData: {}, error: '', durationMs: 0 },
    { id: 'MCUGAUGE', name: '电量计校准', domain: 'mcu', judgeType: 'auto', status: 'pending', rawResponse: '', parsedData: {}, error: '', durationMs: 0 },
    { id: 'MCUTIME', name: '时间同步', domain: 'mcu', judgeType: 'auto', status: 'pending', rawResponse: '', parsedData: {}, error: '', durationMs: 0 },
    { id: 'MCURST', name: '恢复出厂', domain: 'mcu', judgeType: 'auto', status: 'pending', rawResponse: '', parsedData: {}, error: '', durationMs: 0 },
  ]
}

function now(): string {
  const d = new Date()
  return d.toTimeString().split(' ')[0]
}

export const useProductionStore = defineStore('production', () => {
  const modemItems = ref<TestItem[]>(createModemItems())
  const mcuItems = ref<TestItem[]>(createMcuItems())
  const running = ref(false)
  const logs = ref<LogEntry[]>([])

  function addLog(level: LogEntry['level'], message: string) {
    logs.value.push({ time: now(), level, message })
    if (logs.value.length > 200) logs.value.splice(0, logs.value.length - 200)
  }

  function clearLogs() {
    logs.value = []
  }

  function resetAll() {
    modemItems.value = createModemItems()
    mcuItems.value = createMcuItems()
  }

  function updateItem(id: string, updates: Partial<TestItem>) {
    const all = [...modemItems.value, ...mcuItems.value]
    const item = all.find((i) => i.id === id)
    if (item) Object.assign(item, updates)
  }

  function findItem(id: string): TestItem | undefined {
    return [...modemItems.value, ...mcuItems.value].find(i => i.id === id)
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
    addLog('info', `测试 [${item?.name || testId}] ...`)
    updateItem(testId, { status: 'running', error: '', rawResponse: '' })
    try {
      const result = await invoke<{
        status: string
        raw_response: string
        parsed_data: Record<string, string>
        error: string
        duration_ms: number
      }>('run_single_test', { testId })

      updateItem(testId, {
        status: result.status as TestStatus,
        rawResponse: result.raw_response,
        parsedData: result.parsed_data,
        error: result.error,
        durationMs: result.duration_ms,
      })

      if (result.status === 'pass') {
        addLog('success', `[${item?.name || testId}] PASS (${result.duration_ms}ms) ${result.raw_response}`)
      } else {
        addLog('error', `[${item?.name || testId}] FAIL: ${result.error || result.raw_response}`)
      }
    } catch (e: any) {
      updateItem(testId, { status: 'fail', error: String(e) })
      addLog('error', `[${item?.name || testId}] 异常: ${e}`)
    }
  }

  async function runAutoTest() {
    const device = useDeviceStore()
    running.value = true
    resetAll()
    clearLogs()

    addLog('info', '====== 一键产测开始 ======')
    addLog('info', '进入产测模式 AT+PROD=1 ...')

    try {
      await device.enterProductionMode()
      addLog('success', '已进入产测模式')
    } catch (e: any) {
      addLog('error', `进入产测模式失败: ${e}`)
      running.value = false
      return
    }

    const autoTestIds = [
      ...modemItems.value.filter(i => i.judgeType === 'auto').map(i => i.id),
      ...mcuItems.value.filter(i => i.judgeType === 'auto' && i.id !== 'MCURST').map(i => i.id),
    ]

    let passCount = 0
    let failCount = 0

    for (const testId of autoTestIds) {
      if (!running.value) {
        addLog('warn', '用户手动停止')
        break
      }

      const item = findItem(testId)
      addLog('info', `[${passCount + failCount + 1}/${autoTestIds.length}] 测试 [${item?.name || testId}] ...`)
      updateItem(testId, { status: 'running', error: '', rawResponse: '' })

      try {
        const result = await invoke<{
          id: string
          status: string
          raw_response: string
          parsed_data: Record<string, string>
          error: string
          duration_ms: number
        }>('run_single_test', { testId })

        updateItem(result.id, {
          status: result.status as TestStatus,
          rawResponse: result.raw_response,
          parsedData: result.parsed_data,
          error: result.error,
          durationMs: result.duration_ms,
        })

        if (result.status === 'pass') {
          passCount++
          addLog('success', `[${item?.name || testId}] PASS (${result.duration_ms}ms) ${result.raw_response}`)
        } else {
          failCount++
          addLog('error', `[${item?.name || testId}] FAIL: ${result.error || result.raw_response}`)
        }
      } catch (e: any) {
        failCount++
        updateItem(testId, { status: 'fail', error: String(e) })
        addLog('error', `[${item?.name || testId}] 异常: ${e}`)
      }
    }

    addLog('info', '退出产测模式 AT+PROD=0 ...')
    try {
      await device.exitProductionMode()
      addLog('success', '已退出产测模式')
    } catch (e) {
      device.productionMode = 'idle'
      addLog('warn', `退出产测模式异常: ${e}`)
    }

    const rstItem = findItem('MCURST')
    if (rstItem) {
      addLog('info', `[${passCount + failCount + 1}/${autoTestIds.length + 1}] 测试 [恢复出厂] ...`)
      updateItem('MCURST', { status: 'running', error: '', rawResponse: '' })
      try {
        const result = await invoke<{
          id: string
          status: string
          raw_response: string
          parsed_data: Record<string, string>
          error: string
          duration_ms: number
        }>('run_single_test', { testId: 'MCURST' })

        updateItem('MCURST', {
          status: result.status as TestStatus,
          rawResponse: result.raw_response,
          parsedData: result.parsed_data,
          error: result.error,
          durationMs: result.duration_ms,
        })
        if (result.status === 'pass') {
          passCount++
          addLog('success', `[恢复出厂] PASS (${result.duration_ms}ms)`)
        } else {
          failCount++
          addLog('error', `[恢复出厂] FAIL: ${result.error || result.raw_response}`)
        }
      } catch (e: any) {
        failCount++
        updateItem('MCURST', { status: 'fail', error: String(e) })
        addLog('error', `[恢复出厂] 异常: ${e}`)
      }
    }

    addLog('info', `====== 产测完成: 通过 ${passCount}, 失败 ${failCount} ======`)
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
    resetAll,
    clearLogs,
    addLog,
    updateItem,
    runSingleTest,
    runAutoTest,
    manualJudge,
  }
})
