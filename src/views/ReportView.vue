<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import {
  NCard, NSpace, NButton, NTag, NInput, NSelect, NDatePicker,
  NModal, NDescriptions, NDescriptionsItem, NDivider,
  useMessage, useDialog,
} from 'naive-ui'
import { useReportStore } from '@/stores/report'
import type { TestReport, ReportSummary } from '@/stores/report'

const reportStore = useReportStore()
const message = useMessage()
const dialog = useDialog()

const searchText = ref('')
const filterResult = ref<string | null>(null)
const filterDateRange = ref<[number, number] | null>(null)
const showDetail = ref(false)
const detailReport = ref<TestReport | null>(null)

const resultOptions = [
  { label: '全部', value: '' },
  { label: 'PASS', value: 'PASS' },
  { label: 'FAIL', value: 'FAIL' },
]

const todayStats = computed(() => {
  const today = new Date().toISOString().slice(0, 10)
  const todayReports = reportStore.reports.filter(r => r.timestamp.startsWith(today))
  return {
    total: todayReports.length,
    pass: todayReports.filter(r => r.overall === 'PASS').length,
    fail: todayReports.filter(r => r.overall === 'FAIL').length,
  }
})

async function doSearch() {
  const filter: Record<string, string | null> = {
    search: searchText.value || null,
    result: filterResult.value || null,
    date_from: null,
    date_to: null,
  }
  if (filterDateRange.value) {
    filter.date_from = new Date(filterDateRange.value[0]).toISOString().slice(0, 10)
    filter.date_to = new Date(filterDateRange.value[1]).toISOString().slice(0, 10)
  }
  await reportStore.loadReports(filter)
}

async function viewDetail(r: ReportSummary) {
  const report = await reportStore.getReport(r.id)
  if (report) {
    detailReport.value = report
    showDetail.value = true
  }
}

function confirmDelete(r: ReportSummary) {
  dialog.warning({
    title: '确认删除',
    content: `确定删除报告 ${r.imei} (${r.timestamp}) ?`,
    positiveText: '删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      try {
        await reportStore.deleteReport(r.id)
        message.success('已删除')
      } catch (e) {
        message.error(`删除失败: ${e}`)
      }
    },
  })
}

async function handleExport() {
  try {
    const filter: Record<string, string | null> = {
      search: searchText.value || null,
      result: filterResult.value || null,
      date_from: null,
      date_to: null,
    }
    if (filterDateRange.value) {
      filter.date_from = new Date(filterDateRange.value[0]).toISOString().slice(0, 10)
      filter.date_to = new Date(filterDateRange.value[1]).toISOString().slice(0, 10)
    }
    const path = await reportStore.exportCsv(filter)
    message.success(`已导出到: ${path}`)
  } catch (e) {
    message.error(`导出失败: ${e}`)
  }
}

function formatTime(ts: string): string {
  if (ts.length >= 16) return ts.slice(0, 16).replace('T', ' ')
  return ts
}

function statusType(s: string): 'success' | 'error' {
  return s === 'PASS' ? 'success' : 'error'
}

function formatDuration(ms: number): string {
  if (!ms) return '-'
  if (ms < 1000) return `${ms}ms`
  const s = (ms / 1000).toFixed(1)
  return `${s}s`
}

function itemStatusType(s: string): 'success' | 'error' | 'warning' | 'default' {
  if (s === 'pass') return 'success'
  if (s === 'fail') return 'error'
  if (s === 'manual_pending') return 'warning'
  return 'default'
}

onMounted(() => {
  reportStore.loadReports()
})
</script>

<template>
  <div>
    <NCard title="测试报告" size="small">
      <template #header-extra>
        <NSpace size="small">
          <span style="font-size: 12px; color: #666;">
            今日: {{ todayStats.total }} 台 |
            <span style="color: #18a058;">PASS {{ todayStats.pass }}</span> |
            <span style="color: #d03050;">FAIL {{ todayStats.fail }}</span>
          </span>
        </NSpace>
      </template>

      <NSpace style="margin-bottom: 12px;" align="center">
        <NInput
          v-model:value="searchText"
          placeholder="搜索 IMEI / MAC / 操作员..."
          size="small"
          style="width: 220px;"
          clearable
          @keydown.enter="doSearch"
        />
        <NSelect
          v-model:value="filterResult"
          :options="resultOptions"
          size="small"
          placeholder="结果"
          style="width: 100px;"
        />
        <NDatePicker
          v-model:value="filterDateRange"
          type="daterange"
          size="small"
          clearable
          style="width: 240px;"
        />
        <NButton size="small" type="primary" @click="doSearch" :loading="reportStore.loading">
          查询
        </NButton>
        <NButton size="small" @click="handleExport" :disabled="reportStore.reports.length === 0">
          导出 CSV
        </NButton>
      </NSpace>

      <div v-if="reportStore.reports.length === 0 && !reportStore.loading" style="text-align: center; padding: 40px; color: #999;">
        暂无测试报告，完成产测后自动生成
      </div>

      <table v-else style="width: 100%; border-collapse: collapse;">
        <thead>
          <tr style="text-align: left; border-bottom: 1px solid #eee;">
            <th style="padding: 8px; width: 40px;">#</th>
            <th style="padding: 8px;">时间</th>
            <th style="padding: 8px;">IMEI</th>
            <th style="padding: 8px;">产品</th>
            <th style="padding: 8px;">操作员</th>
            <th style="padding: 8px; width: 80px;">结果</th>
            <th style="padding: 8px;">通过/失败/总计</th>
            <th style="padding: 8px; width: 120px;">操作</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="(r, idx) in reportStore.reports"
            :key="r.id"
            style="border-bottom: 1px solid #f5f5f5;"
          >
            <td style="padding: 8px;">{{ idx + 1 }}</td>
            <td style="padding: 8px; font-size: 13px;">{{ formatTime(r.timestamp) }}</td>
            <td style="padding: 8px; font-family: monospace; font-size: 12px;">{{ r.imei || '-' }}</td>
            <td style="padding: 8px;">{{ r.product }}</td>
            <td style="padding: 8px;">{{ r.operator || '-' }}</td>
            <td style="padding: 8px;">
              <NTag :type="statusType(r.overall)" size="small">{{ r.overall }}</NTag>
            </td>
            <td style="padding: 8px; font-size: 12px;">
              <span style="color: #18a058;">{{ r.pass_count }}</span> /
              <span style="color: #d03050;">{{ r.fail_count }}</span> /
              {{ r.total_count }}
            </td>
            <td style="padding: 8px;">
              <NSpace size="small">
                <NButton size="tiny" @click="viewDetail(r)">详情</NButton>
                <NButton size="tiny" type="error" quaternary @click="confirmDelete(r)">删除</NButton>
              </NSpace>
            </td>
          </tr>
        </tbody>
      </table>
    </NCard>

    <NModal v-model:show="showDetail" preset="card" title="测试报告详情" style="width: 700px;">
      <template v-if="detailReport">
        <NDescriptions label-placement="left" :column="2" size="small" bordered>
          <NDescriptionsItem label="报告 ID">{{ detailReport.id }}</NDescriptionsItem>
          <NDescriptionsItem label="时间">{{ formatTime(detailReport.timestamp) }}</NDescriptionsItem>
          <NDescriptionsItem label="产品">{{ detailReport.device.product }}</NDescriptionsItem>
          <NDescriptionsItem label="IMEI">{{ detailReport.device.imei || '-' }}</NDescriptionsItem>
          <NDescriptionsItem label="ICCID">{{ detailReport.device.iccid || '-' }}</NDescriptionsItem>
          <NDescriptionsItem label="固件版本">{{ detailReport.device.fw_version || '-' }}</NDescriptionsItem>
          <NDescriptionsItem label="蓝牙版本">{{ detailReport.device.bt_version || '-' }}</NDescriptionsItem>
          <NDescriptionsItem label="蓝牙 MAC">{{ detailReport.device.bt_mac || '-' }}</NDescriptionsItem>
          <NDescriptionsItem label="操作员">{{ detailReport.operator || '-' }}</NDescriptionsItem>
          <NDescriptionsItem label="总耗时">{{ formatDuration(detailReport.duration_ms) }}</NDescriptionsItem>
          <NDescriptionsItem label="总结果" :span="2">
            <NTag :type="statusType(detailReport.overall)" size="small">{{ detailReport.overall }}</NTag>
          </NDescriptionsItem>
        </NDescriptions>

        <NDivider>测试项明细</NDivider>

        <table style="width: 100%; border-collapse: collapse;">
          <thead>
            <tr style="text-align: left; border-bottom: 1px solid #eee;">
              <th style="padding: 6px;">#</th>
              <th style="padding: 6px;">域</th>
              <th style="padding: 6px;">测试项</th>
              <th style="padding: 6px;">响应</th>
              <th style="padding: 6px;">耗时</th>
              <th style="padding: 6px;">状态</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(item, idx) in detailReport.items" :key="item.id" style="border-bottom: 1px solid #f5f5f5;">
              <td style="padding: 6px;">{{ idx + 1 }}</td>
              <td style="padding: 6px;">
                <NTag :type="item.domain === 'modem' ? 'info' : 'warning'" size="small">
                  {{ item.domain === 'modem' ? '模组' : 'MCU' }}
                </NTag>
              </td>
              <td style="padding: 6px;">{{ item.name }}</td>
              <td style="padding: 6px; font-family: monospace; font-size: 12px;">{{ item.raw }}</td>
              <td style="padding: 6px;">{{ formatDuration(item.duration_ms) }}</td>
              <td style="padding: 6px;">
                <NTag :type="itemStatusType(item.status)" size="small">
                  {{ item.status === 'pass' ? 'PASS' : item.status === 'fail' ? 'FAIL' : item.status }}
                </NTag>
              </td>
            </tr>
          </tbody>
        </table>
      </template>
    </NModal>
  </div>
</template>
