<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from 'vue'
import {
  NCard, NSpace, NButton, NInput, NRadioGroup, NRadioButton, NAlert, NScrollbar,
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
  <div>
    <NAlert v-if="!device.connected" type="warning" style="margin-bottom: 16px;">
      请先连接设备
    </NAlert>

    <NCard title="调试终端" size="small">
      <NSpace vertical>
        <NRadioGroup v-model:value="mode" size="small">
          <NRadioButton value="at">AT 命令</NRadioButton>
          <NRadioButton value="mcu">MCU 透传</NRadioButton>
        </NRadioGroup>
        <div v-if="mode === 'mcu'" style="font-size: 12px; color: #999;">
          MCU 透传模式: 输入 MCU 命令(如 AT+BVER?)，自动包装为 AT+MCURAW="..." 发送
        </div>

        <NScrollbar ref="scrollRef" style="height: 400px; background: #1e1e1e; border-radius: 4px; padding: 8px;">
          <div style="font-family: 'Cascadia Code', 'Fira Code', monospace; font-size: 13px; line-height: 1.6;">
            <div
              v-for="(entry, idx) in logs"
              :key="idx"
              :style="{ color: entry.direction === 'tx' ? '#4fc3f7' : '#a5d6a7' }"
            >
              <span style="color: #888;">[{{ entry.time }}]</span>
              <span>{{ entry.direction === 'tx' ? ' TX > ' : ' RX < ' }}</span>
              <span>{{ entry.data }}</span>
            </div>
          </div>
        </NScrollbar>

        <NSpace>
          <NInput
            v-model:value="inputCmd"
            placeholder="输入 AT 命令..."
            style="width: 500px; font-family: monospace;"
            @keydown.enter="sendCommand"
          />
          <NButton type="primary" @click="sendCommand" :disabled="!device.connected">
            发送
          </NButton>
          <NButton @click="clearLogs">清屏</NButton>
        </NSpace>
      </NSpace>
    </NCard>
  </div>
</template>
