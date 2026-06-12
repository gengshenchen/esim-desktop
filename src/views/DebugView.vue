<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from 'vue'
import {
  NCard, NButton, NInput, NRadioGroup, NRadioButton, NScrollbar,
} from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useDeviceStore } from '@/stores/device'

const device = useDeviceStore()

interface LogEntry {
  time: string
  direction: 'tx' | 'rx'
  data: string
}

const mode = ref<'at' | 'mcu'>('at')
const inputCmd = ref('')
const logs = ref<LogEntry[]>([])
const scrollRef = ref<InstanceType<typeof NScrollbar> | null>(null)

function now(): string {
  const d = new Date()
  return d.toTimeString().split(' ')[0]
}

function addLog(direction: 'tx' | 'rx', data: string) {
  logs.value.push({ time: now(), direction, data })
  if (logs.value.length > 500) logs.value.splice(0, logs.value.length - 500)
  nextTick(() => {
    scrollRef.value?.scrollTo({ top: 99999 })
  })
}

let unlistenData: (() => void) | null = null

onMounted(async () => {
  unlistenData = (await listen<{ line: string; direction: string }>('serial:data', (event) => {
    addLog(event.payload.direction as 'tx' | 'rx', event.payload.line)
  })) as unknown as () => void
})

onUnmounted(() => {
  if (unlistenData) unlistenData()
})

async function sendCommand() {
  if (!inputCmd.value.trim()) return

  let cmd = inputCmd.value.trim()
  if (mode.value === 'mcu') {
    cmd = `AT+MCURAW="${cmd}"`
  }

  addLog('tx', cmd)

  try {
    const resp = await invoke<{ lines: string[]; success: boolean }>('send_at_command', {
      cmd,
      timeoutMs: 5000,
    })
    for (const line of resp.lines) {
      addLog('rx', line)
    }
    addLog('rx', resp.success ? 'OK' : 'ERROR')
  } catch (e: any) {
    addLog('rx', `[ERROR] ${e}`)
  }

  inputCmd.value = ''
}

function clearLogs() {
  logs.value = []
}
</script>

<template>
  <div class="debug-page">
    <div v-if="!device.connected" class="empty-state">
      <div class="empty-state__icon">
        <svg width="56" height="56" viewBox="0 0 24 24" fill="none" stroke="var(--tesla-pale)" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="4 17 10 11 4 5" />
          <line x1="12" y1="19" x2="20" y2="19" />
        </svg>
      </div>
      <div class="empty-state__title">连接设备后可使用调试终端</div>
      <div class="empty-state__desc">在顶部选择串口并点击连接按钮</div>
    </div>

    <NCard v-else title="调试终端" size="small">
      <div class="debug-layout">
        <div class="debug-toolbar">
          <NRadioGroup v-model:value="mode" size="small">
            <NRadioButton value="at">AT 命令</NRadioButton>
            <NRadioButton value="mcu">MCU 透传</NRadioButton>
          </NRadioGroup>
          <span v-if="mode === 'mcu'" class="debug-hint">
            输入 MCU 命令，自动包装为 AT+MCURAW="..." 发送
          </span>
        </div>

        <NScrollbar ref="scrollRef" class="terminal">
          <div class="terminal__content">
            <div
              v-for="(entry, idx) in logs"
              :key="idx"
              class="terminal__line"
              :class="entry.direction === 'tx' ? 'terminal__line--tx' : 'terminal__line--rx'"
            >
              <span class="terminal__time">[{{ entry.time }}]</span>
              <span class="terminal__dir">{{ entry.direction === 'tx' ? ' TX > ' : ' RX < ' }}</span>
              <span>{{ entry.data }}</span>
            </div>
            <div v-if="logs.length === 0" class="terminal__empty">输入命令开始调试...</div>
          </div>
        </NScrollbar>

        <div class="input-bar">
          <NInput
            v-model:value="inputCmd"
            :placeholder="mode === 'at' ? '输入 AT 命令...' : '输入 MCU 命令...'"
            class="input-bar__field"
            @keydown.enter="sendCommand"
          />
          <NButton type="primary" @click="sendCommand" :disabled="!device.connected">
            发送
          </NButton>
          <NButton @click="clearLogs">清屏</NButton>
        </div>
      </div>
    </NCard>
  </div>
</template>

<style scoped>
.debug-page {
  height: 100%;
  display: flex;
  flex-direction: column;
}

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

.debug-layout {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.debug-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
}

.debug-hint {
  font-size: 12px;
  color: var(--tesla-silver);
}

.terminal {
  height: 420px;
  background: var(--tesla-carbon);
  border-radius: var(--tesla-radius-lg);
  padding: 12px 16px;
}

.terminal__content {
  font-family: 'SF Mono', 'Cascadia Code', 'Fira Code', monospace;
  font-size: 13px;
  line-height: 1.7;
}

.terminal__line--tx { color: #60A5FA; }
.terminal__line--rx { color: #4ADE80; }

.terminal__time { color: #64748B; }
.terminal__dir { font-weight: 500; }

.terminal__empty {
  color: #475569;
  font-style: italic;
}

.input-bar {
  display: flex;
  gap: 8px;
}

.input-bar__field {
  flex: 1;
}
.input-bar__field :deep(input) {
  font-family: 'SF Mono', 'Cascadia Code', monospace !important;
}
</style>
