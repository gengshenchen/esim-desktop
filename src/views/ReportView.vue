<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import {
  NSpace, NButton, NInput, NSelect, NDatePicker,
  NModal, NDescriptions, NDescriptionsItem, NDivider,
  useMessage, useDialog,
} from 'naive-ui'
import { save } from '@tauri-apps/plugin-dialog'
import { open as shellOpen } from '@tauri-apps/plugin-shell'
import { invoke } from '@tauri-apps/api/core'
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
    const dataDir = await invoke<string>('cmd_get_data_dir')
    const defaultName = `reports_export_${new Date().toISOString().slice(0, 10)}.csv`
    const filePath = await save({
      defaultPath: `${dataDir}/reports/${defaultName}`,
      filters: [{ name: 'CSV', extensions: ['csv'] }],
    })
    if (!filePath) return

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
    const savedPath = await reportStore.exportCsv(filter, filePath)
    message.success(`已导出: ${savedPath}`)
    openFolder(savedPath)
  } catch (e) {
    message.error(`导出失败: ${e}`)
  }
}

async function openFolder(filePath: string) {
  try {
    const dir = filePath.replace(/[/\\][^/\\]*$/, '')
    await shellOpen(dir)
  } catch {
    // ignore
  }
}

async function openReportFile() {
  if (!detailReport.value) return
  try {
    const filePath = await reportStore.getReportPath(detailReport.value.id)
    openFolder(filePath)
  } catch (e) {
    message.error(`打开失败: ${e}`)
  }
}

function formatTime(ts: string): string {
  if (ts.length >= 16) return ts.slice(0, 16).replace('T', ' ')
  return ts
}

function formatDuration(ms: number): string {
  if (!ms) return '-'
  if (ms < 1000) return `${ms}ms`
  const s = (ms / 1000).toFixed(1)
  return `${s}s`
}

function itemStatusDotClass(s: string): string {
  if (s === 'pass') return 'status-dot--pass'
  if (s === 'fail') return 'status-dot--fail'
  if (s === 'manual_pending') return 'status-dot--info'
  return 'status-dot--pending'
}

function itemStatusLabel(s: string): string {
  if (s === 'pass') return 'PASS'
  if (s === 'fail') return 'FAIL'
  if (s === 'manual_pending') return '待人工'
  return s
}

onMounted(() => {
  reportStore.loadReports()
})
</script>

<template>
  <div>
    <!-- Stats bar -->
    <div class="stats-bar">
      <div class="stat-card">
        <span class="stat-card__value">{{ todayStats.total }}</span>
        <span class="stat-card__label">今日测试</span>
      </div>
      <div class="stat-card">
        <span class="stat-card__value" style="color: var(--tesla-success);">{{ todayStats.pass }}</span>
        <span class="stat-card__label">通过</span>
      </div>
      <div class="stat-card">
        <span class="stat-card__value" style="color: var(--tesla-error);">{{ todayStats.fail }}</span>
        <span class="stat-card__label">失败</span>
      </div>
      <div v-if="todayStats.total > 0" class="stat-card">
        <span class="stat-card__value">{{ ((todayStats.pass / todayStats.total) * 100).toFixed(0) }}%</span>
        <span class="stat-card__label">良率</span>
      </div>
    </div>

    <!-- Search bar -->
    <div class="search-bar">
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
    </div>

    <!-- Empty state -->
    <div v-if="reportStore.reports.length === 0 && !reportStore.loading" class="empty-state">
      暂无测试报告，完成产测后自动生成
    </div>

    <!-- Report table -->
    <div v-else class="report-table-wrap">
      <table class="tesla-table">
        <thead>
          <tr>
            <th style="width: 40px;">#</th>
            <th>时间</th>
            <th>IMEI</th>
            <th>产品</th>
            <th>操作员</th>
            <th style="width: 72px;">结果</th>
            <th>通过/失败/总计</th>
            <th style="width: 120px;">操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(r, idx) in reportStore.reports" :key="r.id">
            <td>{{ idx + 1 }}</td>
            <td style="font-size: 13px;">{{ formatTime(r.timestamp) }}</td>
            <td class="mono">{{ r.imei || '-' }}</td>
            <td>{{ r.product }}</td>
            <td>{{ r.operator || '-' }}</td>
            <td>
              <span class="result-badge" :class="r.overall === 'PASS' ? 'result-badge--pass' : 'result-badge--fail'">
                {{ r.overall }}
              </span>
            </td>
            <td style="font-size: 13px;">
              <span style="color: var(--tesla-success); font-weight: 500;">{{ r.pass_count }}</span>
              <span style="color: var(--tesla-pale);"> / </span>
              <span style="color: var(--tesla-error); font-weight: 500;">{{ r.fail_count }}</span>
              <span style="color: var(--tesla-pale);"> / </span>
              <span>{{ r.total_count }}</span>
            </td>
            <td>
              <NSpace size="small">
                <NButton size="tiny" @click="viewDetail(r)">详情</NButton>
                <NButton size="tiny" quaternary @click="confirmDelete(r)" style="color: var(--tesla-error);">删除</NButton>
              </NSpace>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Detail modal -->
    <NModal
      v-model:show="showDetail"
      preset="card"
      title="测试报告详情"
      style="width: 700px; max-height: 80vh;"
      content-style="overflow-y: auto; max-height: calc(80vh - 60px);"
    >
      <template v-if="detailReport">
        <NDescriptions label-placement="left" :column="2" size="small" bordered>
          <NDescriptionsItem label="报告 ID">{{ detailReport.id }}</NDescriptionsItem>
          <NDescriptionsItem label="时间">{{ formatTime(detailReport.timestamp) }}</NDescriptionsItem>
          <NDescriptionsItem label="产品">{{ detailReport.device.product }}</NDescriptionsItem>
          <NDescriptionsItem label="IMEI">
            <span class="mono-text">{{ detailReport.device.imei || '-' }}</span>
          </NDescriptionsItem>
          <NDescriptionsItem label="ICCID">
            <span class="mono-text">{{ detailReport.device.iccid || '-' }}</span>
          </NDescriptionsItem>
          <NDescriptionsItem label="固件版本">{{ detailReport.device.fw_version || '-' }}</NDescriptionsItem>
          <NDescriptionsItem label="蓝牙版本">{{ detailReport.device.bt_version || '-' }}</NDescriptionsItem>
          <NDescriptionsItem label="蓝牙 MAC">
            <span class="mono-text">{{ detailReport.device.bt_mac || '-' }}</span>
          </NDescriptionsItem>
          <NDescriptionsItem label="操作员">{{ detailReport.operator || '-' }}</NDescriptionsItem>
          <NDescriptionsItem label="总耗时">{{ formatDuration(detailReport.duration_ms) }}</NDescriptionsItem>
          <NDescriptionsItem label="总结果" :span="2">
            <span
              class="result-badge"
              :class="detailReport.overall === 'PASS' ? 'result-badge--pass' : 'result-badge--fail'"
            >
              {{ detailReport.overall }}
            </span>
          </NDescriptionsItem>
        </NDescriptions>

        <NDivider>测试项明细</NDivider>

        <table class="tesla-table">
          <thead>
            <tr>
              <th style="width: 36px;">#</th>
              <th style="width: 56px;">域</th>
              <th>测试项</th>
              <th>响应</th>
              <th style="width: 64px;">耗时</th>
              <th style="width: 72px;">状态</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(item, idx) in detailReport.items" :key="item.id">
              <td>{{ idx + 1 }}</td>
              <td>
                <span
                  class="domain-badge"
                  :class="item.domain === 'modem' ? 'domain-badge--modem' : 'domain-badge--mcu'"
                >
                  <span class="domain-dot" />
                  {{ item.domain === 'modem' ? '模组' : 'MCU' }}
                </span>
              </td>
              <td style="font-weight: 500;">{{ item.name }}</td>
              <td class="mono">{{ item.raw }}</td>
              <td style="font-size: 12px; color: var(--tesla-pewter);">{{ formatDuration(item.duration_ms) }}</td>
              <td>
                <span class="status-label">
                  <span class="status-dot" :class="itemStatusDotClass(item.status)" />
                  {{ itemStatusLabel(item.status) }}
                </span>
              </td>
            </tr>
          </tbody>
        </table>

        <div style="margin-top: 16px; text-align: right;">
          <NButton size="small" quaternary @click="openReportFile">
            打开文件位置
          </NButton>
        </div>
      </template>
    </NModal>
  </div>
</template>

<style scoped>
.stats-bar {
  display: flex;
  gap: 24px;
  margin-bottom: 16px;
  padding: 12px 0;
}

.stat-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  min-width: 64px;
}

.stat-card__value {
  font-size: 24px;
  font-weight: 600;
  color: var(--tesla-carbon);
  line-height: 1.2;
}

.stat-card__label {
  font-size: 11px;
  color: var(--tesla-pewter);
  margin-top: 2px;
}

.search-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 16px;
}

.empty-state {
  text-align: center;
  padding: 60px 20px;
  color: var(--tesla-silver);
  font-size: 14px;
}

.report-table-wrap {
  overflow: auto;
}

.result-badge {
  display: inline-block;
  padding: 2px 10px;
  border-radius: var(--tesla-radius);
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.02em;
}

.result-badge--pass {
  background: rgba(24, 160, 88, 0.1);
  color: var(--tesla-success);
}

.result-badge--fail {
  background: rgba(208, 48, 80, 0.1);
  color: var(--tesla-error);
}

.status-label {
  display: inline-flex;
  align-items: center;
  font-size: 12px;
  font-weight: 500;
  color: var(--tesla-graphite);
}

.mono-text {
  font-family: 'SF Mono', 'Cascadia Code', monospace;
  font-size: 12px;
}
</style>
