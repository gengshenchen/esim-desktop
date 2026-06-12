<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useDeviceStore } from '@/stores/device'

const device = useDeviceStore()

const now = ref('')
let timer: ReturnType<typeof setInterval> | null = null

function updateTime() {
  const d = new Date()
  const mm = String(d.getMonth() + 1).padStart(2, '0')
  const dd = String(d.getDate()).padStart(2, '0')
  const hh = String(d.getHours()).padStart(2, '0')
  const mi = String(d.getMinutes()).padStart(2, '0')
  now.value = `${mm}-${dd} ${hh}:${mi}`
}

onMounted(() => {
  updateTime()
  timer = setInterval(updateTime, 30000)
})

onUnmounted(() => {
  if (timer) clearInterval(timer)
})
</script>

<template>
  <div class="footer">
    <div class="footer__left">
      <template v-if="device.capability">
        <span class="footer__item">{{ device.capability.product }}</span>
        <span v-if="device.deviceInfo.imei" class="footer__sep">&middot;</span>
        <span v-if="device.deviceInfo.imei" class="footer__item footer__mono">{{ device.deviceInfo.imei }}</span>
        <span v-if="device.deviceInfo.fwVersion" class="footer__sep">&middot;</span>
        <span v-if="device.deviceInfo.fwVersion" class="footer__item">{{ device.deviceInfo.fwVersion }}</span>
        <span v-if="device.deviceInfo.fwDate" class="footer__sep">&middot;</span>
        <span v-if="device.deviceInfo.fwDate" class="footer__item">{{ device.deviceInfo.fwDate }}</span>
        <span v-if="device.deviceInfo.fwBranch" class="footer__sep">&middot;</span>
        <span v-if="device.deviceInfo.fwBranch" class="footer__item">{{ device.deviceInfo.fwBranch }}</span>
        <span v-if="device.deviceInfo.btVersion" class="footer__sep">&middot;</span>
        <span v-if="device.deviceInfo.btVersion" class="footer__item">BT {{ device.deviceInfo.btVersion }}</span>
        <span v-if="device.deviceInfo.btMac" class="footer__sep">&middot;</span>
        <span v-if="device.deviceInfo.btMac" class="footer__item footer__mono">{{ device.deviceInfo.btMac }}</span>
      </template>
    </div>
    <div class="footer__right">
      <span class="footer__right-text">{{ now }}</span>
      <span class="footer__right-sep">&middot;</span>
      <span class="footer__right-text">hecsion.com</span>
    </div>
  </div>
</template>

<style scoped>
.footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  height: 100%;
  font-size: 12px;
  color: var(--tesla-pewter);
}
.footer__left {
  display: flex;
  align-items: center;
  min-width: 0;
  overflow: hidden;
}
.footer__right {
  display: flex;
  align-items: baseline;
  flex-shrink: 0;
}
.footer__right-text {
  font-size: 11px;
  color: var(--tesla-silver);
  white-space: nowrap;
}
.footer__right-sep {
  margin: 0 6px;
  color: var(--tesla-pale);
  font-size: 11px;
}
.footer__item {
  white-space: nowrap;
}
.footer__mono {
  font-family: 'SF Mono', 'Cascadia Code', monospace;
  font-size: 11px;
}
.footer__sep {
  margin: 0 8px;
  color: var(--tesla-pale);
}
.footer__muted {
  color: var(--tesla-silver);
}
</style>
