<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { NSelect, NButton, useMessage } from 'naive-ui'
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

const currentTab = computed(() => router.currentRoute.value.name as string)

function handleTabChange(name: string) {
  router.push({ name })
}

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

const connectionBtnType = computed(() => {
  if (device.connected) return 'error' as const
  if (device.autoReconnecting) return 'warning' as const
  return 'primary' as const
})

const connectionBtnLabel = computed(() => {
  if (device.connected) return '断开'
  if (device.autoReconnecting) return '取消等待'
  if (connecting.value) return '连接中...'
  return '连接'
})

const statusColor = computed(() => {
  if (device.connected) return 'var(--tesla-success)'
  if (device.autoReconnecting) return 'var(--tesla-warning)'
  return 'var(--tesla-pale)'
})

const statusText = computed(() => {
  if (device.connected) {
    return device.productionMode === 'production' ? 'PROD' : 'IDLE'
  }
  if (device.autoReconnecting) return '等待设备'
  return '未连接'
})

onMounted(() => {
  device.scanPorts()
  device.loadAutoReconnectSetting()
})
</script>

<template>
  <div class="header">
    <div class="header__brand">
      <span class="header__logo">HECSION</span>
    </div>

    <nav class="header__nav">
      <button
        v-for="tab in tabs"
        :key="tab.name"
        class="nav-tab"
        :class="{ 'nav-tab--active': currentTab === tab.name }"
        @click="handleTabChange(tab.name)"
      >
        {{ tab.label }}
      </button>
    </nav>

    <div class="header__actions">
      <NSelect
        v-model:value="device.selectedPort"
        :options="portOptions"
        placeholder="选择串口"
        size="small"
        style="width: 150px;"
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
        :type="connectionBtnType"
        @click="toggleConnection"
        :disabled="(!device.selectedPort && !device.connected && !device.autoReconnecting) || connecting"
        :loading="connecting"
      >
        {{ connectionBtnLabel }}
      </NButton>

      <div class="header__status">
        <span class="header__status-dot" :style="{ background: statusColor }" />
        <span class="header__status-text">{{ statusText }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.header {
  display: flex;
  align-items: center;
  height: 100%;
  gap: 24px;
}

.header__brand {
  display: flex;
  align-items: baseline;
  gap: 6px;
  flex-shrink: 0;
}

.header__logo {
  font-size: 15px;
  font-weight: 700;
  color: var(--tesla-carbon);
  letter-spacing: 0.08em;
}

.header__nav {
  display: flex;
  gap: 4px;
  flex: 1;
  justify-content: center;
}

.nav-tab {
  position: relative;
  padding: 6px 16px;
  border: none;
  background: transparent;
  font-size: 14px;
  font-weight: 500;
  color: var(--tesla-pewter);
  cursor: pointer;
  border-radius: var(--tesla-radius);
  transition: color var(--tesla-transition), background var(--tesla-transition);
  font-family: inherit;
}

.nav-tab:hover {
  color: var(--tesla-carbon);
  background: var(--tesla-ash);
}

.nav-tab--active {
  color: var(--tesla-blue);
}

.nav-tab--active::after {
  content: '';
  position: absolute;
  bottom: -1px;
  left: 50%;
  transform: translateX(-50%);
  width: 20px;
  height: 2px;
  background: var(--tesla-blue);
  border-radius: 1px;
}

.header__actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.header__status {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 8px;
}

.header__status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  transition: background var(--tesla-transition);
}

.header__status-text {
  font-size: 12px;
  font-weight: 500;
  color: var(--tesla-pewter);
}
</style>
