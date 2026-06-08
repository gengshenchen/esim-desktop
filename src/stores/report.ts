import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export interface TestReportItem {
  id: string
  name: string
  domain: string
  status: string
  data: Record<string, string>
  raw: string
  duration_ms: number
}

export interface DeviceReportInfo {
  product: string
  imei: string
  iccid: string
  fw_version: string
  bt_version: string
  bt_mac: string
}

export interface TestReport {
  id: string
  timestamp: string
  operator: string
  device: DeviceReportInfo
  overall: string
  duration_ms: number
  items: TestReportItem[]
}

export interface ReportSummary {
  id: string
  timestamp: string
  imei: string
  product: string
  overall: string
  operator: string
  pass_count: number
  fail_count: number
  total_count: number
}

export interface ReportFilter {
  date_from: string | null
  date_to: string | null
  result: string | null
  search: string | null
}

export const useReportStore = defineStore('report', () => {
  const reports = ref<ReportSummary[]>([])
  const currentReport = ref<TestReport | null>(null)
  const loading = ref(false)

  async function loadReports(filter?: Partial<ReportFilter>) {
    loading.value = true
    try {
      const f: ReportFilter = {
        date_from: filter?.date_from ?? null,
        date_to: filter?.date_to ?? null,
        result: filter?.result ?? null,
        search: filter?.search ?? null,
      }
      reports.value = await invoke<ReportSummary[]>('cmd_list_reports', { filter: f })
    } catch (e) {
      console.error('load reports failed:', e)
    } finally {
      loading.value = false
    }
  }

  async function getReport(id: string): Promise<TestReport | null> {
    try {
      const report = await invoke<TestReport>('cmd_get_report', { reportId: id })
      currentReport.value = report
      return report
    } catch (e) {
      console.error('get report failed:', e)
      return null
    }
  }

  async function deleteReport(id: string) {
    try {
      await invoke('cmd_delete_report', { reportId: id })
      reports.value = reports.value.filter(r => r.id !== id)
    } catch (e) {
      console.error('delete report failed:', e)
      throw e
    }
  }

  async function exportCsv(filter?: Partial<ReportFilter>): Promise<string> {
    const f: ReportFilter = {
      date_from: filter?.date_from ?? null,
      date_to: filter?.date_to ?? null,
      result: filter?.result ?? null,
      search: filter?.search ?? null,
    }
    return await invoke<string>('cmd_export_csv', { filter: f })
  }

  return {
    reports,
    currentReport,
    loading,
    loadReports,
    getReport,
    deleteReport,
    exportCsv,
  }
})
