<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import {
  NCard, NSpace, NButton, NInput, NInputNumber, NForm, NFormItem,
  NText, NSwitch, NCollapse, NCollapseItem,
  useMessage,
} from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import { useProductionStore } from '@/stores/production'
import { useDeviceStore } from '@/stores/device'

const message = useMessage()
const production = useProductionStore()
const device = useDeviceStore()

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
  auto_reconnect: boolean
  keep_production_mode: boolean
  test_items: TestItemConfig[]
}

const TEST_ITEM_META: Record<string, { name: string; domain: string }> = {
  MDSIM: { name: 'SIM 状态', domain: '模组' },
  MDREG: { name: '网络注册', domain: '模组' },
  MDSIG: { name: '信号质量', domain: '模组' },
  MDDATA: { name: '数据业务', domain: '模组' },
  MDALL: { name: '综合测试', domain: '模组' },
  MDPING: { name: 'Ping 测试', domain: '模组' },
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
  auto_reconnect: true,
  keep_production_mode: true,
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

watch(settings, () => {
  if (!loaded.value) return
  if (saveTimer) clearTimeout(saveTimer)
  saveTimer = setTimeout(doSave, 500)
}, { deep: true })

async function doSave() {
  try {
    await invoke('cmd_save_settings', { settingsData: settings.value })
    device.autoReconnect = settings.value.auto_reconnect
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
  <div style="max-width: 720px;">
    <NCard title="基本设置" size="small" style="margin-bottom: 16px;">
      <NForm label-placement="left" label-width="100" size="small">
        <NFormItem label="操作员">
          <NInput v-model:value="settings.operator" placeholder="输入操作员姓名，记录在报告中" />
        </NFormItem>
        <NFormItem label="波特率">
          <NInputNumber v-model:value="settings.baud_rate" :min="9600" :max="921600" :step="9600" />
        </NFormItem>
        <NFormItem label="自动重连">
          <NSpace align="center" :size="12">
            <NSwitch size="small" v-model:value="settings.auto_reconnect" />
            <NText depth="3" style="font-size: 12px;">设备拔线后自动等待重连</NText>
          </NSpace>
        </NFormItem>
        <NFormItem label="保持产测模式">
          <NSpace align="center" :size="12">
            <NSwitch size="small" v-model:value="settings.keep_production_mode" />
            <NText depth="3" style="font-size: 12px;">测试完成后不退出产测模式，适用于后续需要 DUT 测试的场景</NText>
          </NSpace>
        </NFormItem>
        <NFormItem label="数据目录">
          <span class="data-dir">{{ dataDir || settings.data_dir }}</span>
        </NFormItem>
      </NForm>
    </NCard>

    <NCard title="测试项配置" size="small" style="margin-bottom: 16px;">
      <template #header-extra>
        <NButton size="tiny" quaternary @click="resetDefaults">恢复默认</NButton>
      </template>
      <p class="section-desc">配置每个测试项的开关、超时、重试次数及专属参数</p>

      <table class="tesla-table">
        <thead>
          <tr>
            <th style="width: 56px;">启用</th>
            <th style="width: 56px;">域</th>
            <th>测试项</th>
            <th style="width: 100px;">超时(ms)</th>
            <th style="width: 80px;">重试</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="id in Object.keys(TEST_ITEM_META)" :key="id">
            <td>
              <NSwitch size="small" v-model:value="getItemConfig(id).enabled" />
            </td>
            <td>
              <span
                class="domain-badge"
                :class="TEST_ITEM_META[id].domain === '模组' ? 'domain-badge--modem' : 'domain-badge--mcu'"
              >
                <span class="domain-dot" />
                {{ TEST_ITEM_META[id].domain }}
              </span>
            </td>
            <td style="font-weight: 500;">{{ TEST_ITEM_META[id].name }}</td>
            <td>
              <NInputNumber
                size="small"
                v-model:value="getItemConfig(id).timeout_ms"
                :min="1000" :max="60000" :step="1000"
                style="width: 90px;"
              />
            </td>
            <td>
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
          <NForm label-placement="left" label-width="150" size="small">
            <NFormItem label="CSQ 最小值 (0-31制)">
              <NSpace align="center" :size="8">
                <NInputNumber
                  :value="getParam('MDSIG', 'csq_min', 10)"
                  @update:value="(v: number | null) => setParam('MDSIG', 'csq_min', v ?? 10)"
                  :min="0" :max="31"
                />
                <NText depth="3" style="font-size: 12px;">CSQ为正值时使用</NText>
              </NSpace>
            </NFormItem>
            <NFormItem label="RSSI 最小值 (dBm)">
              <NSpace align="center" :size="8">
                <NInputNumber
                  :value="getParam('MDSIG', 'rssi_min', -90)"
                  @update:value="(v: number | null) => setParam('MDSIG', 'rssi_min', v ?? -90)"
                  :min="-120" :max="-30"
                />
                <NText depth="3" style="font-size: 12px;">CSQ为负值(dBm)时使用</NText>
              </NSpace>
            </NFormItem>
            <NFormItem label="RSRP 最小值 (dBm)">
              <NSpace align="center" :size="8">
                <NInputNumber
                  :value="getParam('MDSIG', 'rsrp_min', -110)"
                  @update:value="(v: number | null) => setParam('MDSIG', 'rsrp_min', v ?? -110)"
                  :min="-140" :max="-44"
                />
                <NText depth="3" style="font-size: 12px;">&ge;-110 为正常</NText>
              </NSpace>
            </NFormItem>
          </NForm>
        </NCollapseItem>

        <NCollapseItem title="Ping 测试 (MDPING) 参数" name="MDPING">
          <NForm label-placement="left" label-width="120" size="small">
            <NFormItem label="目标地址">
              <NInput
                size="small"
                :value="getParam('MDPING', 'ping_host', '8.8.8.8')"
                @update:value="(v: string) => setParam('MDPING', 'ping_host', v)"
                style="width: 200px;"
              />
            </NFormItem>
            <NFormItem label="Ping 次数">
              <NInputNumber
                :value="getParam('MDPING', 'ping_count', 3)"
                @update:value="(v: number | null) => setParam('MDPING', 'ping_count', v ?? 3)"
                :min="1" :max="10"
              />
            </NFormItem>
          </NForm>
        </NCollapseItem>

        <NCollapseItem title="电池电压 (MCUVBAT) 参数" name="MCUVBAT">
          <NForm label-placement="left" label-width="120" size="small">
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
            <NFormItem label="总超时 (秒)">
              <NInputNumber
                :value="getParam('MCUKEY', 'timeout_s', 30)"
                @update:value="(v: number | null) => setParam('MCUKEY', 'timeout_s', v ?? 30)"
                :min="10" :max="120"
              />
            </NFormItem>
            <NFormItem label="单键卡住超时 (秒)">
              <NInputNumber
                :value="getParam('MCUKEY', 'key_timeout_s', 10)"
                @update:value="(v: number | null) => setParam('MCUKEY', 'key_timeout_s', v ?? 10)"
                :min="3" :max="60"
              />
            </NFormItem>
          </NForm>
        </NCollapseItem>
      </NCollapse>
    </NCard>
  </div>
</template>

<style scoped>
.section-desc {
  font-size: 12px;
  color: var(--tesla-silver);
  margin: 0 0 12px;
}

.data-dir {
  font-family: 'SF Mono', 'Cascadia Code', monospace;
  font-size: 12px;
  color: var(--tesla-pewter);
}
</style>
