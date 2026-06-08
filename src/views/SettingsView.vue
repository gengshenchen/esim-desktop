<script setup lang="ts">
import { ref, watch, onMounted, computed } from 'vue'
import {
  NCard, NSpace, NButton, NInput, NInputNumber, NForm, NFormItem,
  NTag, NText, NSwitch, NCollapse, NCollapseItem, NDynamicTags,
  useMessage,
} from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import { useProductionStore } from '@/stores/production'

const message = useMessage()
const production = useProductionStore()

interface TestItemConfig {
  id: string
  enabled: boolean
  retries: number
  timeout_ms: number
  params: Record<string, any>
}

interface AppSettings {
  operator: string
  baud_rate: number
  data_dir: string
  test_items: TestItemConfig[]
}

const TEST_ITEM_META: Record<string, { name: string; domain: string }> = {
  MDSIM: { name: 'SIM 状态', domain: '模组' },
  MDREG: { name: '网络注册', domain: '模组' },
  MDSIG: { name: '信号质量', domain: '模组' },
  MDDATA: { name: '数据业务', domain: '模组' },
  MDALL: { name: '综合测试', domain: '模组' },
  MCUBVER: { name: '蓝牙版本', domain: 'MCU' },
  MCUMAC: { name: '蓝牙 MAC', domain: 'MCU' },
  MCUCHG: { name: '充电信息', domain: 'MCU' },
  MCUVBAT: { name: '电池电压', domain: 'MCU' },
  MCULED: { name: 'LED 测试', domain: 'MCU' },
  MCUFBMIC: { name: 'FB 麦回环', domain: 'MCU' },
  MCUPMIC: { name: '主麦回环', domain: 'MCU' },
  MCUKEY: { name: '按键测试', domain: 'MCU' },
  MCUGAUGE: { name: '电量计校准', domain: 'MCU' },
  MCUTIME: { name: '时间同步', domain: 'MCU' },
  MCURST: { name: '恢复出厂', domain: 'MCU' },
}

const settings = ref<AppSettings>({
  operator: '',
  baud_rate: 115200,
  data_dir: '',
  test_items: [],
})

const dataDir = ref('')
const loaded = ref(false)
let saveTimer: ReturnType<typeof setTimeout> | null = null

function getItemConfig(id: string): TestItemConfig {
  let cfg = settings.value.test_items.find(i => i.id === id)
  if (!cfg) {
    cfg = { id, enabled: true, retries: 1, timeout_ms: 5000, params: {} }
    settings.value.test_items.push(cfg)
  }
  return cfg
}

function getParam(id: string, key: string, defaultVal: any): any {
  const cfg = getItemConfig(id)
  return cfg.params[key] ?? defaultVal
}

function setParam(id: string, key: string, val: any) {
  const cfg = getItemConfig(id)
  cfg.params[key] = val
}

const keyTestKeys = computed({
  get: () => {
    const cfg = getItemConfig('MCUKEY')
    return (cfg.params.keys as string[]) || ['PTT', 'VOL+', 'VOL-', 'POWER']
  },
  set: (val: string[]) => {
    setParam('MCUKEY', 'keys', val)
  },
})

watch(settings, () => {
  if (!loaded.value) return
  if (saveTimer) clearTimeout(saveTimer)
  saveTimer = setTimeout(doSave, 500)
}, { deep: true })

async function doSave() {
  try {
    await invoke('cmd_save_settings', { settingsData: settings.value })
    production.markConfigDirty()
  } catch (e: any) {
    message.error(`保存失败: ${e}`)
  }
}

async function loadSettings() {
  try {
    const data = await invoke<AppSettings>('cmd_load_settings')
    settings.value = data
    dataDir.value = await invoke<string>('cmd_get_data_dir')
    if (!settings.value.data_dir) {
      settings.value.data_dir = dataDir.value
    }
    if (!settings.value.test_items || settings.value.test_items.length === 0) {
      settings.value.test_items = await invoke<TestItemConfig[]>('cmd_get_default_test_items')
    }
  } catch (e) {
    console.error('load settings failed:', e)
  }
  loaded.value = true
}

async function resetDefaults() {
  try {
    settings.value.test_items = await invoke<TestItemConfig[]>('cmd_get_default_test_items')
    message.info('已重置为默认值')
  } catch {
    message.error('获取默认值失败')
  }
}

onMounted(() => {
  loadSettings()
})
</script>

<template>
  <div style="max-width: 700px;">
    <NCard title="基本设置" size="small" style="margin-bottom: 16px;">
      <NForm label-placement="left" label-width="120" size="small">
        <NFormItem label="操作员姓名">
          <NInput v-model:value="settings.operator" placeholder="输入操作员姓名，记录在报告中" />
        </NFormItem>
        <NFormItem label="串口波特率">
          <NInputNumber v-model:value="settings.baud_rate" :min="9600" :max="921600" :step="9600" />
        </NFormItem>
        <NFormItem label="数据目录">
          <NText depth="3" style="font-family: monospace; font-size: 12px;">{{ dataDir || settings.data_dir }}</NText>
        </NFormItem>
      </NForm>
    </NCard>

    <NCard title="测试项配置" size="small" style="margin-bottom: 16px;">
      <template #header-extra>
        <NButton size="tiny" quaternary @click="resetDefaults">恢复默认</NButton>
      </template>
      <div style="font-size: 12px; color: #666; margin-bottom: 12px;">
        配置每个测试项的开关、超时、重试次数及专属参数。禁用的项不参与产测。
      </div>

      <table style="width: 100%; border-collapse: collapse; font-size: 13px;">
        <thead>
          <tr style="text-align: left; border-bottom: 1px solid #e0e0e0;">
            <th style="padding: 6px 8px; width: 50px;">启用</th>
            <th style="padding: 6px 8px; width: 50px;">域</th>
            <th style="padding: 6px 8px;">测试项</th>
            <th style="padding: 6px 8px; width: 100px;">超时(ms)</th>
            <th style="padding: 6px 8px; width: 80px;">重试</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="id in Object.keys(TEST_ITEM_META)" :key="id" style="border-bottom: 1px solid #f5f5f5;">
            <td style="padding: 5px 8px;">
              <NSwitch size="small" v-model:value="getItemConfig(id).enabled" />
            </td>
            <td style="padding: 5px 8px;">
              <NTag size="small" :type="TEST_ITEM_META[id].domain === '模组' ? 'info' : 'warning'" :bordered="false">
                {{ TEST_ITEM_META[id].domain }}
              </NTag>
            </td>
            <td style="padding: 5px 8px;">{{ TEST_ITEM_META[id].name }}</td>
            <td style="padding: 5px 8px;">
              <NInputNumber
                size="small"
                v-model:value="getItemConfig(id).timeout_ms"
                :min="1000" :max="60000" :step="1000"
                style="width: 90px;"
              />
            </td>
            <td style="padding: 5px 8px;">
              <NInputNumber
                size="small"
                v-model:value="getItemConfig(id).retries"
                :min="0" :max="5"
                style="width: 70px;"
              />
            </td>
          </tr>
        </tbody>
      </table>

      <NCollapse style="margin-top: 16px;">
        <NCollapseItem title="信号质量 (MDSIG) 参数" name="MDSIG">
          <NForm label-placement="left" label-width="160" size="small">
            <NFormItem label="CSQ 最小值 (0-31制)">
              <NSpace align="center">
                <NInputNumber
                  :value="getParam('MDSIG', 'csq_min', 10)"
                  @update:value="(v: number | null) => setParam('MDSIG', 'csq_min', v ?? 10)"
                  :min="0" :max="31"
                />
                <NText depth="3" style="font-size: 12px;">CSQ为正值时使用</NText>
              </NSpace>
            </NFormItem>
            <NFormItem label="RSSI 最小值 (dBm)">
              <NSpace align="center">
                <NInputNumber
                  :value="getParam('MDSIG', 'rssi_min', -90)"
                  @update:value="(v: number | null) => setParam('MDSIG', 'rssi_min', v ?? -90)"
                  :min="-120" :max="-30"
                />
                <NText depth="3" style="font-size: 12px;">CSQ为负值(dBm)时使用</NText>
              </NSpace>
            </NFormItem>
            <NFormItem label="RSRP 最小值 (dBm)">
              <NSpace align="center">
                <NInputNumber
                  :value="getParam('MDSIG', 'rsrp_min', -110)"
                  @update:value="(v: number | null) => setParam('MDSIG', 'rsrp_min', v ?? -110)"
                  :min="-140" :max="-44"
                />
                <NText depth="3" style="font-size: 12px;">≥-110 为正常</NText>
              </NSpace>
            </NFormItem>
          </NForm>
        </NCollapseItem>

        <NCollapseItem title="综合测试 (MDALL) 参数" name="MDALL">
          <NForm label-placement="left" label-width="140" size="small">
            <NFormItem label="Ping 测试">
              <NSwitch
                size="small"
                :value="getParam('MDALL', 'ping_enabled', true)"
                @update:value="(v: boolean) => setParam('MDALL', 'ping_enabled', v)"
              />
            </NFormItem>
            <NFormItem label="Ping 目标地址">
              <NInput
                size="small"
                :value="getParam('MDALL', 'ping_host', '8.8.8.8')"
                @update:value="(v: string) => setParam('MDALL', 'ping_host', v)"
                style="width: 200px;"
              />
            </NFormItem>
            <NFormItem label="Ping 次数">
              <NInputNumber
                :value="getParam('MDALL', 'ping_count', 3)"
                @update:value="(v: number | null) => setParam('MDALL', 'ping_count', v ?? 3)"
                :min="1" :max="10"
              />
            </NFormItem>
          </NForm>
        </NCollapseItem>

        <NCollapseItem title="电池电压 (MCUVBAT) 参数" name="MCUVBAT">
          <NForm label-placement="left" label-width="140" size="small">
            <NFormItem label="电压下限 (mV)">
              <NInputNumber
                :value="getParam('MCUVBAT', 'mv_min', 3000)"
                @update:value="(v: number | null) => setParam('MCUVBAT', 'mv_min', v ?? 3000)"
                :min="2500" :max="4200" :step="100"
              />
            </NFormItem>
            <NFormItem label="电压上限 (mV)">
              <NInputNumber
                :value="getParam('MCUVBAT', 'mv_max', 4500)"
                @update:value="(v: number | null) => setParam('MCUVBAT', 'mv_max', v ?? 4500)"
                :min="3500" :max="5000" :step="100"
              />
            </NFormItem>
          </NForm>
        </NCollapseItem>

        <NCollapseItem title="按键测试 (MCUKEY) 参数" name="MCUKEY">
          <NForm label-placement="left" label-width="140" size="small">
            <NFormItem label="测试按键">
              <NDynamicTags v-model:value="keyTestKeys" />
            </NFormItem>
            <NFormItem label="超时 (秒)">
              <NInputNumber
                :value="getParam('MCUKEY', 'timeout_s', 30)"
                @update:value="(v: number | null) => setParam('MCUKEY', 'timeout_s', v ?? 30)"
                :min="10" :max="120"
              />
            </NFormItem>
          </NForm>
        </NCollapseItem>
      </NCollapse>
    </NCard>
  </div>
</template>
