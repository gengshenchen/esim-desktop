<script setup lang="ts">
import { ref, computed, nextTick, watch, onMounted, onUnmounted } from 'vue'
import {
  NSpace, NButton, NTag, NAlert, NScrollbar, NModal, NDivider,
  useDialog,
} from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import { useDeviceStore } from '@/stores/device'
import { useProductionStore } from '@/stores/production'
import type { TestItem } from '@/stores/production'

const device = useDeviceStore()
const production = useProductionStore()
const dialog = useDialog()
const logScrollRef = ref<InstanceType<typeof NScrollbar> | null>(null)

const logState = ref<'closed' | 'expanded' | 'collapsed'>('closed')

// 手动测试弹框状态
const manualModal = ref(false)
const manualItem = ref<TestItem | null>(null)
const manualBusy = ref(false)
let manualStartTime = 0

// 按键测试弹框状态
const showKeyTest = ref(false)
const keyTestElapsed = ref(0)
let keyTestElapsedTimer: ReturnType<typeof setInterval> | null = null

const MANUAL_HINTS: Record<string, string> = {
  MCULED: '请观察设备 LED 是否正常亮起',
  MCUFBMIC: '请确认 FB 麦克风回环是否有声音',
  MCUPMIC: '请确认主麦克风回环是否有声音',
}

const STOP_CMDS: Record<string, string> = {
  MCULED: 'AT+MCULED=0',
  MCUFBMIC: 'AT+MCUFBMIC=0',
  MCUPMIC: 'AT+MCUPMIC=0',
}

onMounted(() => {
  production.reloadIfDirty()
})

onUnmounted(() => {
  if (keyTestElapsedTimer) {
    clearInterval(keyTestElapsedTimer)
    keyTestElapsedTimer = null
  }
})

const allItems = computed(() => {
  const items = [...production.modemItems, ...production.mcuItems]
  const order: Record<string, number> = { auto: 0, semi_auto: 1, manual: 2 }
  return items
    .filter(i => i.id !== 'MCURST')
    .sort((a, b) => (order[a.judgeType] ?? 9) - (order[b.judgeType] ?? 9))
    .concat(items.filter(i => i.id === 'MCURST'))
})
const passCount = computed(() => allItems.value.filter(i => i.status === 'pass').length)
const failCount = computed(() => allItems.value.filter(i => i.status === 'fail').length)
const latestLog = computed(() => production.logs.length > 0 ? production.logs[production.logs.length - 1] : null)
const busy = computed(() => production.running || production.keyTestActive)

watch(() => production.logs.length, () => {
  if (logState.value === 'expanded') {
    nextTick(() => {
      logScrollRef.value?.scrollTo({ top: 99999 })
    })
  }
})

function openLog() {
  logState.value = 'expanded'
  nextTick(() => {
    logScrollRef.value?.scrollTo({ top: 99999 })
  })
}

function closeLog() {
  logState.value = 'closed'
  production.clearLogs()
}

function toggleCollapse() {
  if (logState.value === 'expanded') {
    logState.value = 'collapsed'
  } else if (logState.value === 'collapsed') {
    logState.value = 'expanded'
    nextTick(() => {
      logScrollRef.value?.scrollTo({ top: 99999 })
    })
  }
}

function statusType(s: string): 'success' | 'error' | 'warning' | 'info' | 'default' {
  if (s === 'pass') return 'success'
  if (s === 'fail') return 'error'
  if (s === 'running') return 'warning'
  if (s === 'manual_pending') return 'info'
  return 'default'
}

function statusLabel(s: string): string {
  const map: Record<string, string> = {
    pending: '待执行', running: '执行中', pass: 'PASS', fail: 'FAIL',
    skipped: '跳过', manual_pending: '待人工',
  }
  return map[s] || s
}

function domainLabel(item: TestItem): string {
  return item.domain === 'modem' ? '模组' : 'MCU'
}

function domainType(item: TestItem): 'info' | 'warning' {
  return item.domain === 'modem' ? 'info' : 'warning'
}

function displayData(item: TestItem): string {
  if (item.error) return item.error
  if (item.rawResponse) return item.rawResponse
  return ''
}

function logColor(level: string): string {
  if (level === 'success') return '#52c41a'
  if (level === 'error') return '#ff4d4f'
  if (level === 'warn') return '#faad14'
  return '#d9d9d9'
}

// --- 统一测试入口 ---

function handleTest(item: TestItem) {
  if (item.judgeType === 'auto') {
    if (item.id === 'MCURST') {
      dialog.warning({
        title: '确认恢复出厂',
        content: '恢复出厂将清除 MCU 所有用户数据，确定执行？',
        positiveText: '确认',
        negativeText: '取消',
        onPositiveClick: () => production.runSingleTest(item.id),
      })
    } else {
      production.runSingleTest(item.id)
    }
  } else if (item.judgeType === 'manual') {
    openManualTest(item)
  } else if (item.judgeType === 'semi_auto') {
    openKeyTest()
  }
}

// --- 手动测试弹框 ---

async function openManualTest(item: TestItem) {
  manualItem.value = item
  manualBusy.value = true
  manualModal.value = true
  manualStartTime = Date.now()

  production.addLog('info', `[${item.name}] 开始手动测试`)
  await production.runSingleTest(item.id)
  manualBusy.value = false
}

async function judgeManualTest(pass: boolean) {
  const item = manualItem.value
  if (!item) return
  const durationMs = Date.now() - manualStartTime

  const stopCmd = STOP_CMDS[item.id]
  if (stopCmd) {
    try {
      production.addLog('info', `TX: ${stopCmd}`)
      const resp = await invoke<{ lines: string[]; success: boolean }>('send_at_command', { cmd: stopCmd, timeoutMs: 3000 })
      production.addLog('info', `RX: ${resp.lines?.join(' ') || 'OK'}`)
    } catch (e: any) {
      production.addLog('warn', `停止命令失败: ${e}`)
    }
  }

  production.updateItem(item.id, { status: pass ? 'pass' : 'fail', durationMs })
  production.addLog(pass ? 'success' : 'error', `[${item.name}] 人工判定: ${pass ? 'PASS' : 'FAIL'} (${(durationMs / 1000).toFixed(1)}s)`)
  manualModal.value = false
  manualItem.value = null
}

async function cancelManualTest() {
  const item = manualItem.value
  if (!item) return

  const stopCmd = STOP_CMDS[item.id]
  if (stopCmd) {
    try {
      await invoke('send_at_command', { cmd: stopCmd, timeoutMs: 3000 })
      production.addLog('info', `[${item.name}] 已取消`)
    } catch { /* ignore */ }
  }

  production.updateItem(item.id, { status: 'pending', error: '' })
  manualModal.value = false
  manualItem.value = null
}

// --- 按键测试 ---

const keyTestPassedCount = computed(() => production.keyInfos.filter(k => k.state === 'passed').length)
const keyTestRemaining = computed(() => {
  const { timeoutS } = production.getKeyTestParams()
  const remaining = Math.max(0, timeoutS - keyTestElapsed.value)
  return remaining
})

function keyStateBorder(state: string): string {
  if (state === 'passed') return '#18a058'
  if (state === 'pressed') return '#2080f0'
  if (state === 'stuck') return '#d03050'
  return '#ddd'
}

function keyStateBg(state: string): string {
  if (state === 'passed') return '#f0fdf4'
  if (state === 'pressed') return '#eff6ff'
  if (state === 'stuck') return '#fef2f2'
  return '#fff'
}

function keyStateIcon(state: string): string {
  if (state === 'passed') return '✓'
  if (state === 'pressed') return '↓'
  if (state === 'stuck') return '✕'
  return '○'
}

function keyStateLabel(state: string): string {
  if (state === 'passed') return '通过'
  if (state === 'pressed') return '已按下'
  if (state === 'stuck') return '卡住'
  return '待测'
}

async function openKeyTest() {
  if (busy.value) return

  if (device.productionMode !== 'production') {
    production.addLog('info', '进入产测模式...')
    try {
      await device.enterProductionMode()
      production.addLog('success', '已进入产测模式')
    } catch (e: any) {
      production.addLog('error', `进入产测模式失败: ${e}`)
      return
    }
  }

  production.updateItem('MCUKEY', { status: 'running', error: '', rawResponse: '' })
  production.addLog('info', '[按键测试] 发送 AT+MCUKEY=1')

  try {
    const resp = await invoke<{ lines: string[]; success: boolean }>('send_at_command', { cmd: 'AT+MCUKEY=1', timeoutMs: 5000 })
    if (!resp.success) {
      production.updateItem('MCUKEY', { status: 'fail', error: 'AT+MCUKEY=1 失败' })
      production.addLog('error', '[按键测试] AT+MCUKEY=1 失败')
      return
    }
  } catch (e: any) {
    production.updateItem('MCUKEY', { status: 'fail', error: String(e) })
    production.addLog('error', `[按键测试] AT+MCUKEY=1 异常: ${e}`)
    return
  }

  production.initKeyInfos()
  await production.startKeyTestLoop()
  showKeyTest.value = true

  keyTestElapsed.value = 0
  keyTestElapsedTimer = setInterval(() => {
    keyTestElapsed.value++
  }, 1000)
}

async function stopKeyTest() {
  if (keyTestElapsedTimer) {
    clearInterval(keyTestElapsedTimer)
    keyTestElapsedTimer = null
  }
  const incomplete = production.keyInfos
    .filter(k => k.state !== 'passed')
    .map(k => k.label)
    .join(', ')
  await production.finishKeyTest(false, `手动停止，未通过: ${incomplete}`)
  showKeyTest.value = false
}

watch(() => production.keyTestActive, (active) => {
  if (!active && keyTestElapsedTimer) {
    clearInterval(keyTestElapsedTimer)
    keyTestElapsedTimer = null
  }
  if (!active && showKeyTest.value) {
    setTimeout(() => {
      showKeyTest.value = false
    }, 2000)
  }
})
</script>

<template>
  <div style="display: flex; flex-direction: column; height: 100%;">
    <NAlert v-if="!device.connected" type="warning" style="margin-bottom: 16px;">
      请先连接设备
    </NAlert>

    <template v-if="device.connected">
      <!-- 操作栏 -->
      <div style="display: flex; align-items: center; gap: 12px; margin-bottom: 8px; flex-shrink: 0;">
        <NButton
          type="primary"
          size="small"
          @click="production.runAutoTest()"
          :disabled="busy"
          :loading="production.running"
        >
          一键产测
        </NButton>
        <NButton
          v-if="production.running"
          type="error"
          size="small"
          @click="production.running = false"
        >
          停止
        </NButton>
        <NButton size="small" @click="production.resetAll()" :disabled="busy">
          重置
        </NButton>
        <span style="font-size: 13px;">
          通过: <span style="color: #18a058; font-weight: bold;">{{ passCount }}</span> |
          失败: <span style="color: #d03050; font-weight: bold;">{{ failCount }}</span> |
          总计: {{ allItems.length }}
        </span>
        <div style="flex: 1;" />
        <NButton
          v-if="logState === 'closed'"
          size="small"
          @click="openLog"
        >
          日志
        </NButton>
        <NButton
          v-else
          size="small"
          type="error"
          quaternary
          @click="closeLog"
        >
          关闭日志
        </NButton>
      </div>

      <!-- 产测表格 -->
      <div style="flex: 1; overflow: auto; min-height: 0;">
        <table style="width: 100%; border-collapse: collapse; font-size: 13px;">
          <thead>
            <tr style="text-align: left; border-bottom: 1px solid #e0e0e0; position: sticky; top: 0; background: #fff; z-index: 1;">
              <th style="padding: 6px 8px; width: 32px;">#</th>
              <th style="padding: 6px 8px; width: 50px;">域</th>
              <th style="padding: 6px 8px; width: 90px;">测试项</th>
              <th style="padding: 6px 8px;">结果</th>
              <th style="padding: 6px 8px; width: 60px;">耗时</th>
              <th style="padding: 6px 8px; width: 70px;">状态</th>
              <th style="padding: 6px 8px; width: 80px;">操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(item, idx) in allItems" :key="item.id" style="border-bottom: 1px solid #f5f5f5;">
              <td style="padding: 5px 8px;">{{ idx + 1 }}</td>
              <td style="padding: 5px 8px;">
                <NTag :type="domainType(item)" size="small" :bordered="false">{{ domainLabel(item) }}</NTag>
              </td>
              <td style="padding: 5px 8px;">{{ item.name }}</td>
              <td style="padding: 5px 8px; font-family: monospace; font-size: 12px; max-width: 280px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                {{ displayData(item) }}
              </td>
              <td style="padding: 5px 8px; font-size: 12px;">{{ item.durationMs ? `${item.durationMs}ms` : '' }}</td>
              <td style="padding: 5px 8px;">
                <NTag :type="statusType(item.status)" size="small">{{ statusLabel(item.status) }}</NTag>
              </td>
              <td style="padding: 5px 8px;">
                <NButton
                  size="tiny"
                  @click="handleTest(item)"
                  :disabled="busy || item.status === 'running'"
                >
                  测试
                </NButton>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- 日志面板 -->
      <div v-if="logState !== 'closed'" style="flex-shrink: 0; margin-top: 8px;">
        <div
          v-if="logState === 'collapsed'"
          style="display: flex; align-items: center; background: #1a1a1a; border-radius: 4px; padding: 4px 12px; cursor: pointer; gap: 8px;"
          @click="toggleCollapse"
        >
          <span style="color: #888; font-size: 11px; flex-shrink: 0;">▶</span>
          <span
            v-if="latestLog"
            style="font-family: monospace; font-size: 12px; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
            :style="{ color: logColor(latestLog.level) }"
          >
            [{{ latestLog.time }}] {{ latestLog.message }}
          </span>
          <span v-else style="color: #666; font-family: monospace; font-size: 12px; flex: 1;">暂无日志</span>
        </div>

        <template v-if="logState === 'expanded'">
          <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 4px;">
            <span
              style="font-size: 12px; color: #666; font-weight: 500; cursor: pointer;"
              @click="toggleCollapse"
            >
              ▼ 运行日志
            </span>
            <NButton size="tiny" quaternary @click="production.clearLogs()">清除</NButton>
          </div>
          <NScrollbar ref="logScrollRef" style="height: 180px; background: #1a1a1a; border-radius: 4px; padding: 8px;">
            <div class="log-selectable" style="font-family: 'Cascadia Code', 'Fira Code', monospace; font-size: 12px; line-height: 1.5;">
              <div v-if="production.logs.length === 0" style="color: #666;">暂无日志</div>
              <div v-for="(entry, idx) in production.logs" :key="idx" :style="{ color: logColor(entry.level) }">
                <span style="color: #555;">[{{ entry.time }}]</span>
                {{ entry.message }}
              </div>
            </div>
          </NScrollbar>
        </template>
      </div>

      <!-- 手动测试弹框 -->
      <NModal
        v-model:show="manualModal"
        preset="card"
        :title="manualItem?.name || '手动测试'"
        style="width: 400px;"
        :mask-closable="false"
        :closable="false"
      >
        <div style="text-align: center; padding: 16px 0;">
          <div style="font-size: 15px; color: #333; margin-bottom: 24px;">
            {{ MANUAL_HINTS[manualItem?.id || ''] || '请观察设备状态' }}
          </div>
          <NSpace justify="center" :size="16">
            <NButton
              type="success" size="large" style="width: 120px;"
              :disabled="manualBusy"
              @click="judgeManualTest(true)"
            >
              PASS
            </NButton>
            <NButton
              type="error" size="large" style="width: 120px;"
              :disabled="manualBusy"
              @click="judgeManualTest(false)"
            >
              FAIL
            </NButton>
          </NSpace>
          <div style="margin-top: 16px;">
            <NButton quaternary size="small" @click="cancelManualTest">取消</NButton>
          </div>
        </div>
      </NModal>

      <!-- 按键测试弹框 -->
      <NModal
        v-model:show="showKeyTest"
        preset="card"
        title="按键测试"
        style="width: 450px;"
        :mask-closable="false"
        :closable="false"
      >
        <div style="text-align: center;">
          <NSpace justify="center" style="margin: 20px 0;" :size="16">
            <div
              v-for="key in production.keyInfos"
              :key="key.name"
              style="width: 90px; height: 90px; border: 2px solid #ddd; border-radius: 8px; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 4px; transition: all 0.3s;"
              :style="{ borderColor: keyStateBorder(key.state), background: keyStateBg(key.state) }"
              :class="{ 'key-pressed-pulse': key.state === 'pressed' }"
            >
              <div style="font-size: 22px; font-weight: bold;">{{ keyStateIcon(key.state) }}</div>
              <div style="font-size: 12px; font-weight: bold;">{{ key.label }}</div>
              <div style="font-size: 11px; color: #888;">{{ keyStateLabel(key.state) }}</div>
            </div>
          </NSpace>
          <div style="color: #666; font-size: 13px; margin-bottom: 8px;">
            进度: {{ keyTestPassedCount }}/{{ production.keyInfos.length }}
            <span style="margin-left: 12px; color: #999;">剩余: {{ keyTestRemaining }}s</span>
          </div>
          <div style="color: #999; font-size: 12px; margin-bottom: 4px;">
            {{ production.keyTestActive ? '请依次按下并释放所有按键...' : (keyTestPassedCount === production.keyInfos.length ? '全部通过!' : '测试已结束') }}
          </div>
          <NDivider />
          <NSpace justify="center" :size="16">
            <NButton
              v-if="production.keyTestActive"
              type="error" size="large" style="width: 120px;"
              @click="stopKeyTest"
            >
              停止
            </NButton>
            <NButton
              v-if="!production.keyTestActive"
              quaternary size="small"
              @click="showKeyTest = false"
            >
              关闭
            </NButton>
          </NSpace>
        </div>
      </NModal>
    </template>
  </div>
</template>

<style scoped>
@keyframes key-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.6; }
}
.key-pressed-pulse {
  animation: key-pulse 1.2s ease-in-out infinite;
}
</style>
