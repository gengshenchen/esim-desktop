<script setup lang="ts">
import { ref, computed, nextTick } from 'vue'
import { NCard, NSpace, NButton, NTag, NAlert, NScrollbar } from 'naive-ui'
import { useDeviceStore } from '@/stores/device'
import { useProductionStore } from '@/stores/production'
import type { TestItem } from '@/stores/production'

const device = useDeviceStore()
const production = useProductionStore()
const logScrollRef = ref<InstanceType<typeof NScrollbar> | null>(null)

const allItems = computed(() => [...production.modemItems, ...production.mcuItems])
const passCount = computed(() => allItems.value.filter(i => i.status === 'pass').length)
const failCount = computed(() => allItems.value.filter(i => i.status === 'fail').length)

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

async function handleAutoTest() {
  await production.runAutoTest()
  nextTick(() => {
    logScrollRef.value?.scrollTo({ top: 99999 })
  })
}
</script>

<template>
  <div>
    <NAlert v-if="!device.connected" type="warning" style="margin-bottom: 16px;">
      请先连接设备
    </NAlert>

    <template v-if="device.connected">
      <NSpace style="margin-bottom: 16px;">
        <NButton
          type="primary"
          @click="handleAutoTest"
          :disabled="production.running"
          :loading="production.running"
        >
          一键产测
        </NButton>
        <NButton
          v-if="production.running"
          type="error"
          @click="production.running = false"
        >
          停止
        </NButton>
        <NButton @click="production.resetAll()" :disabled="production.running">
          重置
        </NButton>
        <span>
          通过: {{ passCount }} | 失败: {{ failCount }} | 总计: {{ allItems.length }}
        </span>
      </NSpace>

      <NCard title="产测项目" size="small" style="margin-bottom: 16px;">
        <table style="width: 100%; border-collapse: collapse;">
          <thead>
            <tr style="text-align: left; border-bottom: 1px solid #eee;">
              <th style="padding: 8px; width: 40px;">#</th>
              <th style="padding: 8px; width: 60px;">域</th>
              <th style="padding: 8px;">测试项</th>
              <th style="padding: 8px;">结果</th>
              <th style="padding: 8px; width: 80px;">耗时</th>
              <th style="padding: 8px; width: 100px;">状态</th>
              <th style="padding: 8px; width: 160px;">操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(item, idx) in allItems" :key="item.id" style="border-bottom: 1px solid #f5f5f5;">
              <td style="padding: 8px;">{{ idx + 1 }}</td>
              <td style="padding: 8px;">
                <NTag :type="domainType(item)" size="small">{{ domainLabel(item) }}</NTag>
              </td>
              <td style="padding: 8px;">{{ item.name }}</td>
              <td style="padding: 8px; font-family: monospace; font-size: 12px;">{{ displayData(item) }}</td>
              <td style="padding: 8px;">{{ item.durationMs ? `${item.durationMs}ms` : '' }}</td>
              <td style="padding: 8px;">
                <NTag :type="statusType(item.status)" size="small">{{ statusLabel(item.status) }}</NTag>
              </td>
              <td style="padding: 8px;">
                <NSpace size="small">
                  <NButton size="tiny" @click="production.runSingleTest(item.id)" :disabled="production.running">
                    测试
                  </NButton>
                  <template v-if="item.judgeType === 'manual' && item.status === 'manual_pending'">
                    <NButton size="tiny" type="success" @click="production.manualJudge(item.id, true)">PASS</NButton>
                    <NButton size="tiny" type="error" @click="production.manualJudge(item.id, false)">FAIL</NButton>
                  </template>
                </NSpace>
              </td>
            </tr>
          </tbody>
        </table>
      </NCard>

      <NCard title="运行日志" size="small">
        <template #header-extra>
          <NButton size="tiny" @click="production.clearLogs()">清除</NButton>
        </template>
        <NScrollbar ref="logScrollRef" style="height: 200px; background: #1a1a1a; border-radius: 4px; padding: 8px;">
          <div style="font-family: 'Cascadia Code', 'Fira Code', monospace; font-size: 12px; line-height: 1.6;">
            <div v-if="production.logs.length === 0" style="color: #666;">暂无日志，点击"一键产测"或单项测试开始</div>
            <div v-for="(entry, idx) in production.logs" :key="idx" :style="{ color: logColor(entry.level) }">
              <span style="color: #666;">[{{ entry.time }}]</span>
              {{ entry.message }}
            </div>
          </div>
        </NScrollbar>
      </NCard>
    </template>
  </div>
</template>
