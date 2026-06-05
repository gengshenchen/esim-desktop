<script setup lang="ts">
import { NText } from 'naive-ui'
import { useDeviceStore } from '@/stores/device'

const device = useDeviceStore()
</script>

<template>
  <div style="display: flex; align-items: center; height: 100%; gap: 16px; font-size: 12px; color: #666;">
    <NText v-if="device.capability">
      设备: {{ device.capability.product }}
    </NText>
    <NText v-if="device.deviceInfo.imei">
      IMEI: {{ device.deviceInfo.imei }}
    </NText>
    <NText v-if="device.deviceInfo.fwVersion">
      FW: {{ device.deviceInfo.fwVersion }}
    </NText>
    <NText v-if="!device.connected" depth="3">
      请连接设备
    </NText>
  </div>
</template>
