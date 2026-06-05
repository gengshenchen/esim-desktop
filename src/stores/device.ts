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
  btVersion: string
  btMac: string
}

export type ProductionMode = 'idle' | 'entering' | 'production' | 'exiting'

export const useDeviceStore = defineStore('device', () => {
  const availablePorts = ref<string[]>([])
  const selectedPort = ref<string | null>(null)
  const connected = ref(false)
  const capability = ref<DeviceCapability | null>(null)
  const productionMode = ref<ProductionMode>('idle')
  const deviceInfo = reactive<DeviceInfo>({
    imei: '',
    iccid: '',
    fwVersion: '',
    btVersion: '',
    btMac: '',
  })

  let unlistenDisconnect: (() => void) | null = null

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
    try {
      capability.value = await invoke<DeviceCapability>('connect', { port })
      connected.value = true
      productionMode.value = 'idle'

      try {
        unlistenDisconnect = (await listen('serial:disconnected', () => {
          handleDisconnect()
        })) as unknown as () => void
      } catch (e) {
        console.warn('event listen not available:', e)
      }
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
    handleDisconnect()
  }

  function handleDisconnect() {
    connected.value = false
    capability.value = null
    productionMode.value = 'idle'
    Object.assign(deviceInfo, { imei: '', iccid: '', fwVersion: '', btVersion: '', btMac: '' })
    if (unlistenDisconnect) {
      unlistenDisconnect()
      unlistenDisconnect = null
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
    deviceInfo,
    scanPorts,
    connect,
    disconnect,
    enterProductionMode,
    exitProductionMode,
  }
})
