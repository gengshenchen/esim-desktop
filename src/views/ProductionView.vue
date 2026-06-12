<script setup lang="ts">
import { ref, computed, nextTick, watch, onMounted, onUnmounted } from 'vue'
import {
  NSpace, NButton, NScrollbar, NModal, NDivider,
  useDialog,
} from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow, LogicalSize, availableMonitors } from '@tauri-apps/api/window'
import { useDeviceStore } from '@/stores/device'
import { useProductionStore } from '@/stores/production'
import { useReportStore } from '@/stores/report'
import type { TestItem } from '@/stores/production'

const device = useDeviceStore()
const production = useProductionStore()
const reportStore = useReportStore()
const dialog = useDialog()
const logScrollRef = ref<InstanceType<typeof NScrollbar> | null>(null)

const logExpanded = ref(false)

const manualModal = ref(false)
const manualItem = ref<TestItem | null>(null)
const manualBusy = ref(false)
let manualStartTime = 0

const showKeyTest = ref(false)
const keyTestElapsed = ref(0)
let keyTestElapsedTimer: ReturnType<typeof setInterval> | null = null

const resultOverlay = ref<'pass' | 'fail' | null>(null)
let resultOverlayTimer: ReturnType<typeof setTimeout> | null = null
const keepProductionMode = ref(false)

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
  reportStore.loadReports()
})

onUnmounted(() => {
  if (keyTestElapsedTimer) {
    clearInterval(keyTestElapsedTimer)
    keyTestElapsedTimer = null
  }
  if (resultOverlayTimer) {
    clearTimeout(resultOverlayTimer)
    resultOverlayTimer = null
  }
})

const modemItems = computed(() => {
  const items = production.modemItems.filter(i => i.id !== 'MCURST')
  return [...items.filter(i => i.judgeType === 'auto'), ...items.filter(i => i.judgeType !== 'auto')]
})
const mcuItemsNormal = computed(() => {
  const items = production.mcuItems.filter(i => i.id !== 'MCURST')
  const sorted = [...items.filter(i => i.judgeType === 'auto'), ...items.filter(i => i.judgeType !== 'auto')]
  const rst = production.mcuItems.find(i => i.id === 'MCURST')
  if (rst) sorted.push(rst)
  return sorted
})
const allItems = computed(() => [...production.modemItems, ...production.mcuItems])
const passCount = computed(() => allItems.value.filter(i => i.status === 'pass').length)
const failCount = computed(() => allItems.value.filter(i => i.status === 'fail').length)
const totalCount = computed(() => allItems.value.length)
const testedCount = computed(() => passCount.value + failCount.value)
const latestLog = computed(() => production.logs.length > 0 ? production.logs[production.logs.length - 1] : null)
const busy = computed(() => production.running || production.keyTestActive)

const progressPercent = computed(() => {
  if (totalCount.value === 0) return 0
  return Math.round((testedCount.value / totalCount.value) * 100)
})

// Today stats from report store
const todayStats = computed(() => {
  const today = new Date().toISOString().slice(0, 10)
  const todayReports = reportStore.reports.filter(r => r.timestamp.startsWith(today))
  return {
    total: todayReports.length,
    pass: todayReports.filter(r => r.overall === 'PASS').length,
    fail: todayReports.filter(r => r.overall === 'FAIL').length,
  }
})

const todayYield = computed(() => {
  if (todayStats.value.total === 0) return '-'
  return ((todayStats.value.pass / todayStats.value.total) * 100).toFixed(1) + '%'
})

// All tests done detection
const allDone = computed(() => {
  if (totalCount.value === 0) return false
  return allItems.value.every(i => i.status === 'pass' || i.status === 'fail' || i.status === 'skipped')
})

watch(allDone, async (done) => {
  if (!done) return
  const hasFail = failCount.value > 0
  resultOverlay.value = hasFail ? 'fail' : 'pass'

  try {
    const s = await invoke<{ keep_production_mode?: boolean }>('cmd_load_settings')
    keepProductionMode.value = !!s.keep_production_mode
  } catch { /* ignore */ }

  if (!hasFail) {
    resultOverlayTimer = setTimeout(() => {
      resultOverlay.value = null
      reportStore.loadReports()
    }, 3000)
  } else {
    reportStore.loadReports()
  }
})

function dismissOverlay() {
  resultOverlay.value = null
  if (resultOverlayTimer) {
    clearTimeout(resultOverlayTimer)
    resultOverlayTimer = null
  }
}

// Log auto-scroll
watch(() => production.logs.length, () => {
  if (logExpanded.value) {
    nextTick(() => {
      logScrollRef.value?.scrollTo({ top: 99999 })
    })
  }
})

function toggleLog() {
  logExpanded.value = !logExpanded.value
  if (logExpanded.value) {
    nextTick(() => {
      logScrollRef.value?.scrollTo({ top: 99999 })
    })
  }
}

// Auto resize on connect
watch(() => device.connected, async (connected) => {
  if (!connected) return
  try {
    const win = getCurrentWindow()
    const settings = await invoke<{ test_items: { id: string; enabled: boolean }[] }>('cmd_load_settings')
    const enabled = settings.test_items.filter(i => i.enabled)
    const modemCount = enabled.filter(i => i.id.startsWith('MD')).length
    const mcuCount = enabled.filter(i => !i.id.startsWith('MD')).length

    const monitors = await availableMonitors()
    const scaleFactor = monitors.length > 0 ? monitors[0].scaleFactor : 1
    const screenH = monitors.length > 0 ? Math.floor(monitors[0].size.height / scaleFactor) : 1080

    const size = await win.outerSize()
    const factor = await win.scaleFactor()
    const curW = Math.round(size.width / factor)

    const CARD_W = 175
    const CARD_H = 92
    const CARD_GAP = 10
    const CHROME_H = 52 + 32 + 40 + 60 + 64 + 56 + 40

    const contentW = curW - 48
    const cols = Math.max(1, Math.floor(contentW / (CARD_W + CARD_GAP)))
    const modemRows = Math.ceil(modemCount / cols)
    const mcuRows = Math.ceil(mcuCount / cols)
    const neededH = CHROME_H + (modemRows + mcuRows) * (CARD_H + CARD_GAP)

    const curH = Math.round(size.height / factor)
    if (neededH > curH) {
      const maxH = Math.min(Math.floor(screenH * 0.85), 900)
      const finalH = Math.min(neededH, maxH)
      await win.setSize(new LogicalSize(curW, finalH))
    }
  } catch {}
})

function statusLabel(s: string): string {
  const map: Record<string, string> = {
    pending: '待测', running: '执行中', pass: 'PASS', fail: 'FAIL',
    skipped: '跳过', manual_pending: '待人工',
  }
  return map[s] || s
}

function cardBorderColor(s: string): string {
  if (s === 'pass') return 'var(--tesla-success)'
  if (s === 'fail') return 'var(--tesla-error)'
  if (s === 'running') return 'var(--tesla-blue)'
  if (s === 'manual_pending') return 'var(--tesla-blue)'
  return 'var(--tesla-cloud)'
}

function displayData(item: TestItem): string {
  if (item.error) return item.error
  if (item.rawResponse) return item.rawResponse
  return ''
}

function logColor(level: string): string {
  if (level === 'success') return '#4ADE80'
  if (level === 'error') return '#F87171'
  if (level === 'warn') return '#FBBF24'
  return '#94A3B8'
}

function judgeTypeLabel(t: string): string {
  if (t === 'auto') return '自动'
  if (t === 'manual') return '人工'
  if (t === 'semi_auto') return '半自动'
  return t
}

// --- Test entry points ---

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

// --- Manual test modal ---

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

// --- Key test ---

const keyTestPassedCount = computed(() => production.keyInfos.filter(k => k.state === 'passed').length)
const keyTestRemaining = computed(() => {
  const { timeoutS } = production.getKeyTestParams()
  return Math.max(0, timeoutS - keyTestElapsed.value)
})

function keyStateBorder(state: string): string {
  if (state === 'passed') return 'var(--tesla-success)'
  if (state === 'pressed') return 'var(--tesla-blue)'
  if (state === 'stuck') return 'var(--tesla-error)'
  return 'var(--tesla-cloud)'
}

function keyStateBg(state: string): string {
  if (state === 'passed') return 'rgba(24,160,88,0.06)'
  if (state === 'pressed') return 'rgba(62,106,225,0.06)'
  if (state === 'stuck') return 'rgba(208,48,80,0.06)'
  return 'var(--tesla-white)'
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
  showKeyTest.value = true

  keyTestElapsed.value = 0
  keyTestElapsedTimer = setInterval(() => {
    keyTestElapsed.value++
  }, 1000)

  await production.startKeyTestLoop()
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
  <div class="production">
    <!-- Empty state when not connected -->
    <div v-if="!device.connected" class="empty-state">
      <div class="empty-state__icon">
        <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="var(--tesla-pale)" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <rect x="4" y="4" width="16" height="16" rx="2" />
          <path d="M9 1v3M15 1v3M9 20v3M15 20v3" />
        </svg>
      </div>
      <div class="empty-state__title">连接设备开始产测</div>
      <div class="empty-state__desc">在顶部选择串口并点击连接按钮</div>
    </div>

    <template v-if="device.connected">
      <!-- Top bar -->
      <div class="topbar">
        <div class="topbar__left">
          <NButton
            type="primary"
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
        </div>

        <div class="topbar__right">
          <span class="topbar__stat">
            今日 <b>{{ todayStats.total }}</b>
          </span>
          <span class="topbar__stat topbar__stat--pass">
            ✓{{ todayStats.pass }}
          </span>
          <span class="topbar__stat topbar__stat--fail">
            ✗{{ todayStats.fail }}
          </span>
          <span class="topbar__stat">
            良率 <b>{{ todayYield }}</b>
          </span>
          <span class="topbar__sep">|</span>
          <span class="topbar__stat">
            {{ testedCount }}/{{ totalCount }}
          </span>
          <span class="topbar__progress-text" :style="{ color: failCount > 0 ? 'var(--tesla-error)' : (testedCount === totalCount && totalCount > 0 ? 'var(--tesla-success)' : 'var(--tesla-pewter)') }">
            {{ progressPercent }}%
          </span>
        </div>
      </div>

      <!-- Test sections -->
      <div class="sections">
        <!-- Modem section -->
        <div class="section">
          <div class="section__header">
            <span class="section__dot section__dot--modem" />
            <span class="section__title">模组测试</span>
            <span class="section__count">{{ modemItems.length }} 项</span>
          </div>
          <div class="card-grid">
            <div
              v-for="item in modemItems"
              :key="item.id"
              class="test-card"
              :class="{
                'test-card--pass': item.status === 'pass',
                'test-card--fail': item.status === 'fail',
                'test-card--running': item.status === 'running',
                'test-card--active': item.status === 'manual_pending',
              }"
              @click="!busy && item.status !== 'running' && handleTest(item)"
            >
              <div class="test-card__top">
                <span class="test-card__name">{{ item.name }}</span>
                <span class="test-card__badge" :style="{ background: cardBorderColor(item.status) }">
                  {{ statusLabel(item.status) }}
                </span>
              </div>
              <div class="test-card__response" v-if="displayData(item)" :title="displayData(item)">
                {{ displayData(item) }}
              </div>
              <div class="test-card__bottom">
                <span class="test-card__type">{{ judgeTypeLabel(item.judgeType) }}</span>
                <span v-if="item.durationMs" class="test-card__duration">{{ item.durationMs }}ms</span>
              </div>
            </div>
          </div>
        </div>

        <!-- MCU section -->
        <div class="section">
          <div class="section__header">
            <span class="section__dot section__dot--mcu" />
            <span class="section__title">MCU 测试</span>
            <span class="section__count">{{ mcuItemsNormal.length }} 项</span>
          </div>
          <div class="card-grid">
            <div
              v-for="item in mcuItemsNormal"
              :key="item.id"
              class="test-card"
              :class="{
                'test-card--pass': item.status === 'pass',
                'test-card--fail': item.status === 'fail',
                'test-card--running': item.status === 'running',
                'test-card--active': item.status === 'manual_pending',
              }"
              @click="!busy && item.status !== 'running' && handleTest(item)"
            >
              <div class="test-card__top">
                <span class="test-card__name">{{ item.name }}</span>
                <span class="test-card__badge" :style="{ background: cardBorderColor(item.status) }">
                  {{ statusLabel(item.status) }}
                </span>
              </div>
              <div class="test-card__response" v-if="displayData(item)" :title="displayData(item)">
                {{ displayData(item) }}
              </div>
              <div class="test-card__bottom">
                <span class="test-card__type">{{ judgeTypeLabel(item.judgeType) }}</span>
                <span v-if="item.durationMs" class="test-card__duration">{{ item.durationMs }}ms</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Log bar (always visible when connected) -->
      <div class="log-panel">
        <div class="log-bar" @click="toggleLog">
          <span class="log-bar__arrow">{{ logExpanded ? '▼' : '▶' }}</span>
          <span
            v-if="latestLog"
            class="log-bar__text"
            :style="{ color: logColor(latestLog.level) }"
          >
            [{{ latestLog.time }}] {{ latestLog.message }}
          </span>
          <span v-else class="log-bar__text" style="color: #475569;">暂无日志</span>
          <NButton
            v-if="logExpanded"
            size="tiny"
            quaternary
            style="color: #64748B; margin-left: auto;"
            @click.stop="production.clearLogs()"
          >
            清除
          </NButton>
        </div>

        <div v-if="logExpanded" class="log-body-wrap">
          <NScrollbar ref="logScrollRef" class="log-body">
            <div class="log-selectable log-content">
              <div v-if="production.logs.length === 0" style="color: #475569;">暂无日志</div>
              <div v-for="(entry, idx) in production.logs" :key="idx" :style="{ color: logColor(entry.level) }">
                <span style="color: #64748B;">[{{ entry.time }}]</span>
                {{ entry.message }}
              </div>
            </div>
          </NScrollbar>
        </div>
      </div>

      <!-- Result overlay -->
      <Teleport to="body">
        <Transition name="result-fade">
          <div v-if="resultOverlay" class="result-overlay" @click="dismissOverlay">
            <div class="result-overlay__content" :class="resultOverlay === 'pass' ? 'result-overlay--pass' : 'result-overlay--fail'">
              <div class="result-overlay__icon">{{ resultOverlay === 'pass' ? '✓' : '✗' }}</div>
              <div class="result-overlay__text">{{ resultOverlay === 'pass' ? 'PASS' : 'FAIL' }}</div>
              <div class="result-overlay__detail">
                {{ passCount }} 通过 / {{ failCount }} 失败
              </div>
              <div v-if="failCount > 0" class="result-overlay__fails">
                <div v-for="item in allItems.filter(i => i.status === 'fail')" :key="item.id" class="result-overlay__fail-item">
                  {{ item.name }}: {{ item.error || item.rawResponse || 'FAIL' }}
                </div>
              </div>
              <div class="result-overlay__saved">报告已保存</div>
              <div v-if="keepProductionMode" class="result-overlay__saved" style="color: var(--tesla-warning);">设备仍在产测模式</div>
              <div v-if="resultOverlay === 'fail'" class="result-overlay__dismiss">点击关闭</div>
            </div>
          </div>
        </Transition>
      </Teleport>

      <!-- Manual test modal -->
      <NModal
        v-model:show="manualModal"
        preset="card"
        :title="manualItem?.name || '手动测试'"
        style="width: 400px;"
        :mask-closable="false"
        :closable="false"
      >
        <div class="manual-modal">
          <p class="manual-modal__hint">
            {{ MANUAL_HINTS[manualItem?.id || ''] || '请观察设备状态' }}
          </p>
          <NSpace justify="center" :size="16">
            <NButton
              type="primary" size="large" style="width: 120px; background: var(--tesla-success); border: none;"
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
          <div style="margin-top: 16px; text-align: center;">
            <NButton quaternary size="small" @click="cancelManualTest">取消</NButton>
          </div>
        </div>
      </NModal>

      <!-- Key test modal -->
      <NModal
        v-model:show="showKeyTest"
        preset="card"
        title="按键测试"
        style="width: 450px;"
        :mask-closable="false"
        :closable="false"
      >
        <div class="keytest-modal">
          <div class="keytest-grid">
            <div
              v-for="key in production.keyInfos"
              :key="key.name"
              class="keytest-key"
              :style="{ borderColor: keyStateBorder(key.state), background: keyStateBg(key.state) }"
              :class="{ 'keytest-key--pulse': key.state === 'pressed' }"
            >
              <div class="keytest-key__icon">{{ keyStateIcon(key.state) }}</div>
              <div class="keytest-key__name">{{ key.label }}</div>
              <div class="keytest-key__state">{{ keyStateLabel(key.state) }}</div>
            </div>
          </div>
          <div class="keytest-progress">
            进度: {{ keyTestPassedCount }}/{{ production.keyInfos.length }}
            <span style="margin-left: 12px; color: var(--tesla-silver);">剩余: {{ keyTestRemaining }}s</span>
          </div>
          <div class="keytest-hint">
            {{ production.keyTestActive ? '请依次按下并释放所有按键...' : (keyTestPassedCount === production.keyInfos.length ? '全部通过!' : '测试已结束') }}
          </div>
          <NDivider style="margin: 16px 0 12px;" />
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
.production {
  display: flex;
  flex-direction: column;
  height: 100%;
}

/* Empty state */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  flex: 1;
  gap: 12px;
  padding: 60px 20px;
}

.empty-state__icon {
  opacity: 0.6;
}

.empty-state__title {
  font-size: 16px;
  font-weight: 500;
  color: var(--tesla-graphite);
}

.empty-state__desc {
  font-size: 13px;
  color: var(--tesla-silver);
}

/* Top bar */
.topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
  flex-shrink: 0;
}

.topbar__left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.topbar__right {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 12px;
  color: var(--tesla-pewter);
}

.topbar__stat b {
  font-weight: 600;
  color: var(--tesla-carbon);
}

.topbar__stat--pass {
  color: var(--tesla-success);
  font-weight: 500;
}

.topbar__stat--fail {
  color: var(--tesla-error);
  font-weight: 500;
}

.topbar__sep {
  color: var(--tesla-cloud);
}

.topbar__progress-text {
  font-size: 13px;
  font-weight: 600;
}

/* Sections */
.sections {
  flex: 1;
  min-height: 0;
  overflow: auto;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.section__header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}

.section__dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.section__dot--modem { background: var(--tesla-blue); }
.section__dot--mcu { background: var(--tesla-warning); }

.section__title {
  font-size: 13px;
  font-weight: 600;
  color: var(--tesla-carbon);
}

.section__count {
  font-size: 12px;
  color: var(--tesla-silver);
}

/* Card grid */
.card-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(165px, 1fr));
  gap: 8px;
}

/* Test card */
.test-card {
  background: var(--tesla-white);
  border: 1px solid var(--tesla-cloud);
  border-radius: 8px;
  padding: 10px 12px;
  cursor: pointer;
  transition: all var(--tesla-transition);
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-height: 60px;
}

.test-card:hover {
  border-color: var(--tesla-pale);
  background: var(--tesla-ash);
}

.test-card--pass {
  border-left: 3px solid var(--tesla-success);
}

.test-card--fail {
  border-left: 3px solid var(--tesla-error);
  background: rgba(208, 48, 80, 0.02);
}

.test-card--running {
  border-left: 3px solid var(--tesla-blue);
  background: rgba(62, 106, 225, 0.02);
}

.test-card--active {
  border-left: 3px solid var(--tesla-blue);
}

.test-card__top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
}

.test-card__name {
  font-size: 13px;
  font-weight: 500;
  color: var(--tesla-carbon);
  white-space: nowrap;
}

.test-card__badge {
  font-size: 10px;
  font-weight: 600;
  color: #fff;
  padding: 1px 6px;
  border-radius: 3px;
  white-space: nowrap;
  line-height: 1.5;
}

.test-card__response {
  font-family: 'SF Mono', 'Cascadia Code', monospace;
  font-size: 11px;
  color: var(--tesla-pewter);
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  line-height: 1.4;
  word-break: break-all;
}

.test-card__bottom {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.test-card__type {
  font-size: 11px;
  color: var(--tesla-silver);
}

.test-card__duration {
  font-size: 11px;
  color: var(--tesla-silver);
  font-family: 'SF Mono', 'Cascadia Code', monospace;
}

/* Log panel */
.log-panel {
  flex-shrink: 0;
  margin-top: 8px;
}

.log-bar {
  display: flex;
  align-items: center;
  background: var(--tesla-carbon);
  border-radius: 6px;
  padding: 5px 12px;
  cursor: pointer;
  gap: 8px;
}

.log-bar__arrow {
  color: #64748B;
  font-size: 10px;
  flex-shrink: 0;
}

.log-bar__text {
  font-family: 'SF Mono', 'Cascadia Code', monospace;
  font-size: 11px;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.log-body-wrap {
  margin-top: 4px;
}

.log-body {
  height: 140px;
  background: var(--tesla-carbon);
  border-radius: 6px;
  padding: 10px 12px;
}

.log-content {
  font-family: 'SF Mono', 'Cascadia Code', monospace;
  font-size: 11px;
  line-height: 1.6;
}

/* Result overlay */
.result-overlay {
  position: fixed;
  inset: 0;
  z-index: 9999;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.5);
  cursor: pointer;
}

.result-overlay__content {
  text-align: center;
  padding: 48px 64px;
  border-radius: 16px;
  min-width: 320px;
}

.result-overlay--pass {
  background: rgba(24, 160, 88, 0.95);
  color: #fff;
}

.result-overlay--fail {
  background: rgba(208, 48, 80, 0.95);
  color: #fff;
}

.result-overlay__icon {
  font-size: 72px;
  font-weight: 700;
  line-height: 1;
}

.result-overlay__text {
  font-size: 48px;
  font-weight: 700;
  letter-spacing: 0.1em;
  margin-top: 8px;
}

.result-overlay__detail {
  font-size: 16px;
  margin-top: 16px;
  opacity: 0.9;
}

.result-overlay__fails {
  margin-top: 16px;
  text-align: left;
  font-size: 13px;
  opacity: 0.85;
  max-height: 200px;
  overflow: auto;
}

.result-overlay__fail-item {
  padding: 4px 0;
  border-bottom: 1px solid rgba(255, 255, 255, 0.15);
}

.result-overlay__saved {
  font-size: 12px;
  margin-top: 20px;
  opacity: 0.7;
}

.result-overlay__dismiss {
  font-size: 12px;
  margin-top: 8px;
  opacity: 0.5;
}

.result-fade-enter-active {
  transition: opacity 0.3s ease;
}
.result-fade-leave-active {
  transition: opacity 0.5s ease;
}
.result-fade-enter-from,
.result-fade-leave-to {
  opacity: 0;
}

/* Manual test modal */
.manual-modal {
  text-align: center;
  padding: 12px 0;
}

.manual-modal__hint {
  font-size: 15px;
  color: var(--tesla-graphite);
  margin-bottom: 24px;
}

/* Key test modal */
.keytest-modal {
  text-align: center;
}

.keytest-grid {
  display: flex;
  justify-content: center;
  gap: 16px;
  margin: 20px 0;
}

.keytest-key {
  width: 96px;
  height: 96px;
  border: 2px solid var(--tesla-cloud);
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 4px;
  transition: all var(--tesla-transition);
}

.keytest-key--pulse {
  animation: key-pulse 1.2s ease-in-out infinite;
}

.keytest-key__icon {
  font-size: 22px;
  font-weight: 600;
}

.keytest-key__name {
  font-size: 12px;
  font-weight: 500;
  color: var(--tesla-graphite);
}

.keytest-key__state {
  font-size: 11px;
  color: var(--tesla-silver);
}

.keytest-progress {
  color: var(--tesla-pewter);
  font-size: 13px;
  margin-bottom: 4px;
}

.keytest-hint {
  color: var(--tesla-silver);
  font-size: 12px;
}

@keyframes key-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}
</style>
