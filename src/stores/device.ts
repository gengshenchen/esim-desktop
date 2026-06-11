import { defineStore } from 'pinia'
import { ref, reactive } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export interface DeviceCapability {
  product: string
  config: boolean
  production: boolean
  mcu: boolean
}

export interface DeviceInfo {
  imei: string
  iccid: string
  fwVersion: string
  fwDate: string
  fwBranch: string
  btVersion: string
  btMac: string
}

export interface KeyDef {
  name: string
  label: string
  note: string
}

export interface KeyTestConfig {
  keys: KeyDef[]
}

export interface ProductConfig {
  product: string
  key_test: KeyTestConfig
}

export type ProductionMode = 'idle' | 'entering' | 'production' | 'exiting'

export const useDeviceStore = defineStore('device', () => {
  const availablePorts = ref<string[]>([])
  const selectedPort = ref<string | null>(null)
  const connected = ref(false)
  const capability = ref<DeviceCapability | null>(null)
  const productionMode = ref<ProductionMode>('idle')
  const productConfig = ref<ProductConfig | null>(null)
  const autoReconnect = ref(true)
  const autoReconnecting = ref(false)
  const deviceInfo = reactive<DeviceInfo>({
    imei: '',
    iccid: '',
    fwVersion: '',
    fwDate: '',
    fwBranch: '',
    btVersion: '',
    btMac: '',
  })

  let unlistenDisconnect: (() => void) | null = null
  let reconnectTimer: ReturnType<typeof setInterval> | null = null

  function stopReconnectTimer() {
    if (reconnectTimer) {
      clearInterval(reconnectTimer)
      reconnectTimer = null
    }
    autoReconnecting.value = false
  }

  function startReconnectTimer(portName: string) {
    stopReconnectTimer()
    autoReconnecting.value = true

    // Snapshot ports at disconnect time; any new port appearing later is the re-plugged device
    let baselinePorts: string[] = [...availablePorts.value]

    reconnectTimer = setInterval(async () => {
      try {
        const ports = await invoke<string[]>('scan_ports')
        availablePorts.value = ports

        // Prefer the same port name; otherwise pick any newly appeared port
        let target: string | null = null
        if (ports.includes(portName)) {
          target = portName
        } else {
          const newPort = ports.find(p => !baselinePorts.includes(p))
          if (newPort) target = newPort
        }

        if (target) {
          stopReconnectTimer()
          try {
            await connect(target)
          } catch {
            // Update baseline so we don't keep retrying a port that fails
            baselinePorts = [...availablePorts.value]
            startReconnectTimer(portName)
          }
        }
      } catch {
        // scan failed, retry next tick
      }
    }, 1000)
  }

  async function loadAutoReconnectSetting() {
    try {
      const settings = await invoke<{ auto_reconnect?: boolean }>('cmd_load_settings')
      autoReconnect.value = settings.auto_reconnect !== false
    } catch {
      autoReconnect.value = true
    }
  }

  async function scanPorts() {
    try {
      availablePorts.value = await invoke<string[]>('scan_ports')
      if (availablePorts.value.length > 0 && !selectedPort.value) {
        selectedPort.value = availablePorts.value[0]
      }
    } catch (e) {
      console.error('scan_ports failed:', e)
    }
  }

  async function connect(port: string) {
    stopReconnectTimer()
    try {
      capability.value = await invoke<DeviceCapability>('connect', { port })
      connected.value = true
      selectedPort.value = port
      productionMode.value = 'idle'

      try {
        const ver = await invoke<Record<string, string>>('query_version')
        deviceInfo.fwVersion = ver.APP || ver.app || ''
        deviceInfo.fwDate = ver.DATE || ver.date || ''
        deviceInfo.fwBranch = ver.BRANCH || ver.branch || ''
      } catch {
        // version query optional
      }

      try {
        productConfig.value = await invoke<ProductConfig>('cmd_load_product_config', {
          product: capability.value?.product || 'UNKNOWN',
        })
      } catch {
        productConfig.value = null
      }

      try {
        unlistenDisconnect = (await listen('serial:disconnected', () => {
          handleDisconnect(false)
        })) as unknown as () => void
      } catch (e) {
        console.warn('event listen not available:', e)
      }

      import('./production').then(({ useProductionStore }) => {
        const production = useProductionStore()
        production.loadTestItemConfigs()
      }).catch(() => {})
    } catch (e) {
      console.error('connect failed:', e)
      throw e
    }
  }

  async function disconnect() {
    try {
      await invoke('disconnect')
    } catch (e) {
      console.error('disconnect failed:', e)
    }
    handleDisconnect(true)
  }

  function handleDisconnect(manual: boolean) {
    const lastPort = selectedPort.value
    connected.value = false
    capability.value = null
    productionMode.value = 'idle'
    productConfig.value = null
    Object.assign(deviceInfo, { imei: '', iccid: '', fwVersion: '', fwDate: '', fwBranch: '', btVersion: '', btMac: '' })
    if (unlistenDisconnect) {
      unlistenDisconnect()
      unlistenDisconnect = null
    }

    // Release Rust-side write port fd so kernel can reclaim the ttyACM number
    if (!manual) {
      invoke('disconnect').catch(() => {})
    }

    import('./production').then(({ useProductionStore }) => {
      const production = useProductionStore()
      production.stopKeyTestCleanup()
      production.running = false
      production.resetAll()
      production.clearLogs()
    }).catch(() => {})

    if (!manual && autoReconnect.value && lastPort) {
      selectedPort.value = lastPort
      startReconnectTimer(lastPort)
    } else {
      selectedPort.value = null
      stopReconnectTimer()
      scanPorts()
    }
  }

  async function enterProductionMode() {
    productionMode.value = 'entering'
    try {
      await invoke('enter_production_mode')
      productionMode.value = 'production'
    } catch (e) {
      productionMode.value = 'idle'
      throw e
    }
  }

  async function exitProductionMode() {
    productionMode.value = 'exiting'
    try {
      await invoke('exit_production_mode')
      productionMode.value = 'idle'
    } catch (e) {
      productionMode.value = 'production'
      throw e
    }
  }

  return {
    availablePorts,
    selectedPort,
    connected,
    capability,
    productionMode,
    productConfig,
    autoReconnect,
    autoReconnecting,
    deviceInfo,
    scanPorts,
    connect,
    disconnect,
    enterProductionMode,
    exitProductionMode,
    loadAutoReconnectSetting,
    stopReconnectTimer,
  }
})
