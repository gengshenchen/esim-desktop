<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import {
  NSelect, NButton, NTag, NTabs, NTab, useMessage,
} from 'naive-ui'
import { useDeviceStore } from '@/stores/device'

const router = useRouter()
const device = useDeviceStore()
const message = useMessage()
const connecting = ref(false)
const scanning = ref(false)

const portOptions = computed(() =>
  device.availablePorts.map((p) => ({ label: p, value: p }))
)

const tabs = [
  { name: 'production', label: '产测' },
  { name: 'config', label: '配置' },
  { name: 'debug', label: '调试' },
  { name: 'report', label: '报告' },
  { name: 'settings', label: '设置' },
]

function handleTabChange(name: string) {
  router.push({ name })
}

const currentTab = computed(() => router.currentRoute.value.name as string)

async function handleScan() {
  scanning.value = true
  try {
    await device.scanPorts()
    if (device.availablePorts.length === 0) {
      message.warning('未找到可用串口，请检查 USB 连接')
    } else {
      message.success(`找到 ${device.availablePorts.length} 个串口`)
    }
  } catch (e: any) {
    message.error(`扫描失败: ${e}`)
  } finally {
    scanning.value = false
  }
}

async function toggleConnection() {
  if (device.connected) {
    await device.disconnect()
    message.info('已断开连接')
    return
  }

  if (device.autoReconnecting) {
    device.stopReconnectTimer()
    message.info('已取消自动重连')
    return
  }

  if (!device.selectedPort) {
    message.warning('请先选择串口')
    return
  }

  connecting.value = true
  try {
    await device.connect(device.selectedPort)
    message.success(`已连接 ${device.selectedPort}，设备: ${device.capability?.product || '未知'}`)
  } catch (e: any) {
    message.error(`连接失败: ${e}`)
  } finally {
    connecting.value = false
  }
}

onMounted(() => {
  device.scanPorts()
  device.loadAutoReconnectSetting()
})
</script>

<template>
  <div style="display: flex; align-items: center; height: 100%; gap: 12px;">
    <strong style="white-space: nowrap;">E02T 产测工具</strong>

    <NTabs
      type="segment"
      size="small"
      :value="currentTab"
      @update:value="handleTabChange"
      style="flex: 1;"
    >
      <NTab v-for="tab in tabs" :key="tab.name" :name="tab.name">
        {{ tab.label }}
      </NTab>
    </NTabs>

    <NSelect
      v-model:value="device.selectedPort"
      :options="portOptions"
      placeholder="选择串口"
      size="small"
      style="width: 160px;"
      :disabled="device.connected || device.autoReconnecting"
    />

    <NButton
      size="small"
      @click="handleScan"
      :disabled="device.connected || device.autoReconnecting"
      :loading="scanning"
    >
      扫描
    </NButton>

    <NButton
      size="small"
      :type="device.connected ? 'error' : device.autoReconnecting ? 'warning' : 'primary'"
      @click="toggleConnection"
      :disabled="(!device.selectedPort && !device.connected && !device.autoReconnecting) || connecting"
      :loading="connecting"
    >
      {{ device.connected ? '断开' : device.autoReconnecting ? '取消等待' : connecting ? '连接中...' : '连接' }}
    </NButton>

    <NTag
      :type="device.connected ? 'success' : device.autoReconnecting ? 'warning' : 'default'"
      size="small"
    >
      {{ device.connected ? (device.productionMode === 'production' ? 'PRODUCTION' : 'IDLE') : device.autoReconnecting ? '等待设备...' : '未连接' }}
    </NTag>
  </div>
</template>
