<script setup lang="ts">
import { ref, computed } from 'vue'
import {
  NCard, NSpace, NButton, NInput, NRadioGroup, NRadioButton,
  NProgress, NDivider, NSelect, NModal, useMessage, useDialog,
} from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import { useDeviceStore } from '@/stores/device'
import { useProductionStore } from '@/stores/production'

const device = useDeviceStore()
const production = useProductionStore()
const message = useMessage()
const dialogApi = useDialog()

interface ConfigTemplate {
  name: string
  content: string
  description: string
  updated_at: string
}

const source = ref<'edit' | 'template' | 'file'>('edit')
const editorContent = ref('')
const uploadProgress = ref(0)
const uploadTotal = ref(0)
const uploading = ref(false)
const readbackContent = ref('')
const pushedContent = ref('')
const diffMatch = ref<boolean | null>(null)

const templates = ref<ConfigTemplate[]>([])
const selectedTemplate = ref<string | null>(null)
const showSaveTemplate = ref(false)
const newTemplateName = ref('')
const newTemplateDesc = ref('')

const templateOptions = computed(() =>
  templates.value.map(t => ({ label: `${t.name} — ${t.description}`, value: t.name }))
)

const diffLines = computed(() => {
  if (!pushedContent.value && !readbackContent.value) return []
  const pushed = pushedContent.value.split('\n')
  const readback = readbackContent.value.split('\n')
  const max = Math.max(pushed.length, readback.length)
  const lines: Array<{ pushed: string; readback: string; match: boolean }> = []
  for (let i = 0; i < max; i++) {
    const p = pushed[i] ?? ''
    const r = readback[i] ?? ''
    lines.push({ pushed: p, readback: r, match: p.trim() === r.trim() })
  }
  return lines
})

async function loadTemplates() {
  try {
    templates.value = await invoke<ConfigTemplate[]>('cmd_list_templates')
  } catch (e) {
    console.error('load templates failed:', e)
  }
}

function loadSelectedTemplate() {
  if (!selectedTemplate.value) return
  const tpl = templates.value.find(t => t.name === selectedTemplate.value)
  if (tpl) {
    editorContent.value = tpl.content
    message.success(`已加载模板: ${tpl.name}`)
  }
}

async function doSaveTemplate() {
  if (!newTemplateName.value.trim()) {
    message.warning('请输入模板名称')
    return
  }
  try {
    const now = new Date().toISOString().slice(0, 19)
    await invoke('cmd_save_template', {
      template: {
        name: newTemplateName.value.trim(),
        content: editorContent.value,
        description: newTemplateDesc.value.trim(),
        updated_at: now,
      },
    })
    message.success('模板已保存')
    showSaveTemplate.value = false
    newTemplateName.value = ''
    newTemplateDesc.value = ''
    await loadTemplates()
  } catch (e: any) {
    message.error(`保存模板失败: ${e}`)
  }
}

async function deleteSelectedTemplate() {
  if (!selectedTemplate.value) return
  dialogApi.warning({
    title: '删除模板',
    content: `确定删除模板 "${selectedTemplate.value}" ?`,
    positiveText: '删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      try {
        await invoke('cmd_delete_template', { name: selectedTemplate.value })
        message.success('模板已删除')
        selectedTemplate.value = null
        await loadTemplates()
      } catch (e: any) {
        message.error(`删除失败: ${e}`)
      }
    },
  })
}

function handleFileImport() {
  const input = document.createElement('input')
  input.type = 'file'
  input.accept = '.ini,.txt,.conf,.cfg'
  input.onchange = (e: Event) => {
    const file = (e.target as HTMLInputElement).files?.[0]
    if (!file) return
    const reader = new FileReader()
    reader.onload = () => {
      editorContent.value = reader.result as string
      message.success(`已导入文件: ${file.name}`)
    }
    reader.readAsText(file)
  }
  input.click()
}

async function readFromDevice() {
  if (!await ensureProductionMode()) return
  try {
    const lines = await invoke<string[]>('config_read')
    editorContent.value = lines.join('\n')
    readbackContent.value = ''
    pushedContent.value = ''
    diffMatch.value = null
    message.success('已从设备回读配置')
  } catch (e: any) {
    message.error(`读取配置失败: ${e}`)
  }
}

async function pushToDevice() {
  if (!editorContent.value.trim() && editorContent.value.length > 0) return
  if (!await ensureProductionMode()) return
  uploading.value = true
  uploadProgress.value = 0

  const lines = editorContent.value.split('\n')
  uploadTotal.value = lines.length
  pushedContent.value = editorContent.value

  try {
    const result = await invoke<{ lines_sent: number; readback: string[] }>('config_upload', {
      lines,
    })

    readbackContent.value = result.readback.join('\n')
    const pushed = editorContent.value.trim()
    const readback = readbackContent.value.trim()
    diffMatch.value = pushed === readback
    uploadProgress.value = uploadTotal.value
    message.success(diffMatch.value ? '推送成功，回读校验一致' : '推送完成，回读内容不一致')
  } catch (e: any) {
    message.error(`推送失败: ${e}`)
  }

  uploading.value = false
}

async function restoreDefault() {
  dialogApi.warning({
    title: '恢复默认配置',
    content: '将删除设备当前配置并恢复为程序默认值，确定？',
    positiveText: '确认',
    negativeText: '取消',
    onPositiveClick: async () => {
      if (!await ensureProductionMode()) return
      try {
        await invoke('config_restore_default')
        editorContent.value = ''
        readbackContent.value = ''
        pushedContent.value = ''
        diffMatch.value = null
        message.success('已恢复默认配置')
      } catch (e: any) {
        message.error(`恢复失败: ${e}`)
      }
    },
  })
}

async function clearConfig() {
  dialogApi.warning({
    title: '清空设备配置',
    content: '将清空设备上的配置文件，确定？',
    positiveText: '确认',
    negativeText: '取消',
    onPositiveClick: async () => {
      if (!await ensureProductionMode()) return
      try {
        await invoke('config_clear')
        editorContent.value = ''
        readbackContent.value = ''
        pushedContent.value = ''
        diffMatch.value = null
        message.success('设备配置已清空')
      } catch (e: any) {
        message.error(`清空失败: ${e}`)
      }
    },
  })
}

async function ensureProductionMode(): Promise<boolean> {
  if (production.keyTestActive) {
    message.warning('按键测试进行中，请先完成按键测试')
    return false
  }
  if (device.productionMode === 'production') return true
  try {
    await device.enterProductionMode()
    return true
  } catch (e: any) {
    message.error(`进入产测模式失败: ${e}`)
    return false
  }
}

loadTemplates()
</script>

<template>
  <div class="config-page">
    <div v-if="!device.connected" class="empty-state">
      <div class="empty-state__icon">
        <svg width="56" height="56" viewBox="0 0 24 24" fill="none" stroke="var(--tesla-pale)" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 20h9" />
          <path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z" />
        </svg>
      </div>
      <div class="empty-state__title">连接设备后可管理配置</div>
      <div class="empty-state__desc">在顶部选择串口并点击连接按钮</div>
    </div>

    <NCard v-else title="配置管理" size="small">
      <div class="config-layout">
        <!-- Source selector -->
        <div class="source-bar">
          <NRadioGroup v-model:value="source" size="small">
            <NRadioButton value="edit">手动编辑</NRadioButton>
            <NRadioButton value="template">从模板</NRadioButton>
            <NRadioButton value="file">从文件导入</NRadioButton>
          </NRadioGroup>

          <template v-if="source === 'template'">
            <NSelect
              v-model:value="selectedTemplate"
              :options="templateOptions"
              placeholder="选择模板"
              size="small"
              style="width: 260px;"
              clearable
            />
            <NButton size="small" @click="loadSelectedTemplate" :disabled="!selectedTemplate">
              加载
            </NButton>
            <NButton size="small" quaternary @click="deleteSelectedTemplate" :disabled="!selectedTemplate" style="color: var(--tesla-error);">
              删除
            </NButton>
          </template>

          <NButton v-if="source === 'file'" size="small" @click="handleFileImport">
            选择文件...
          </NButton>
        </div>

        <!-- Editor -->
        <NInput
          v-model:value="editorContent"
          type="textarea"
          :rows="12"
          placeholder="version=1&#10;server=192.168.1.100&#10;port=9000&#10;apn=cmnet&#10;volume=8"
          class="config-editor"
        />

        <!-- Action buttons -->
        <div class="action-bar">
          <NButton type="primary" size="small" @click="pushToDevice" :loading="uploading" :disabled="!device.connected">
            推送到设备
          </NButton>
          <NButton size="small" @click="readFromDevice" :disabled="!device.connected">
            从设备回读
          </NButton>
          <NButton size="small" @click="restoreDefault" :disabled="!device.connected">
            恢复默认
          </NButton>
          <NButton size="small" @click="clearConfig" :disabled="!device.connected">
            清空设备配置
          </NButton>
          <NButton size="small" @click="showSaveTemplate = true" :disabled="!editorContent.trim()">
            另存为模板
          </NButton>
        </div>

        <!-- Upload progress -->
        <div v-if="uploading" class="progress-bar">
          <NProgress
            type="line"
            :percentage="uploadTotal > 0 ? Math.round((uploadProgress / uploadTotal) * 100) : 0"
          />
          <span class="progress-text">{{ uploadProgress }}/{{ uploadTotal }} 行</span>
        </div>

        <!-- Diff status -->
        <div v-if="diffMatch !== null" class="diff-status">
          <span
            class="result-badge"
            :class="diffMatch ? 'result-badge--pass' : 'result-badge--fail'"
          >
            {{ diffMatch ? '回读校验: 与推送内容一致' : '回读校验: 内容不一致' }}
          </span>
        </div>

        <!-- Diff view -->
        <div v-if="diffLines.length > 0" class="diff-view">
          <NDivider>对比视图（推送 vs 回读）</NDivider>
          <div class="diff-columns">
            <div class="diff-col">
              <div class="diff-col__header">推送内容</div>
              <div
                v-for="(line, idx) in diffLines"
                :key="'p' + idx"
                class="diff-line"
                :class="{ 'diff-line--mismatch': !line.match }"
              >
                {{ line.pushed }}
              </div>
            </div>
            <div class="diff-col">
              <div class="diff-col__header">回读内容</div>
              <div
                v-for="(line, idx) in diffLines"
                :key="'r' + idx"
                class="diff-line"
                :class="{ 'diff-line--mismatch': !line.match }"
              >
                <span style="flex: 1;">{{ line.readback }}</span>
                <span v-if="line.match" style="color: var(--tesla-success); font-size: 11px;">&#10003;</span>
                <span v-else style="color: var(--tesla-error); font-size: 11px;">&#10007;</span>
              </div>
            </div>
          </div>
        </div>

        <!-- Readback only -->
        <div v-else-if="readbackContent && !pushedContent" style="margin-top: 12px;">
          <NDivider>回读内容</NDivider>
          <NInput
            :value="readbackContent"
            type="textarea"
            :rows="8"
            readonly
            class="config-editor"
          />
        </div>
      </div>
    </NCard>

    <!-- Save template modal -->
    <NModal v-model:show="showSaveTemplate" preset="card" title="保存为模板" style="width: 400px;">
      <NSpace vertical :size="12">
        <NInput v-model:value="newTemplateName" placeholder="模板名称" />
        <NInput v-model:value="newTemplateDesc" placeholder="模板描述（可选）" />
        <NSpace justify="end">
          <NButton size="small" @click="showSaveTemplate = false">取消</NButton>
          <NButton size="small" type="primary" @click="doSaveTemplate">保存</NButton>
        </NSpace>
      </NSpace>
    </NModal>
  </div>
</template>

<style scoped>
.config-page {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  flex: 1;
  gap: 12px;
  padding: 60px 20px;
}

.empty-state__icon {
  opacity: 0.6;
}

.empty-state__title {
  font-size: 16px;
  font-weight: 500;
  color: var(--tesla-graphite);
}

.empty-state__desc {
  font-size: 13px;
  color: var(--tesla-silver);
}

.config-layout {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.source-bar {
  display: flex;
  align-items: center;
  gap: 8px;
}

.config-editor {
  font-family: 'SF Mono', 'Cascadia Code', 'Fira Code', monospace !important;
  font-size: 13px !important;
}
.config-editor :deep(textarea) {
  font-family: 'SF Mono', 'Cascadia Code', 'Fira Code', monospace !important;
  background: var(--tesla-ash) !important;
}

.action-bar {
  display: flex;
  gap: 8px;
}

.progress-bar {
  display: flex;
  align-items: center;
  gap: 12px;
}

.progress-text {
  font-size: 12px;
  color: var(--tesla-pewter);
  white-space: nowrap;
}

.diff-status {
  margin-top: 4px;
}

.result-badge {
  display: inline-block;
  padding: 2px 10px;
  border-radius: var(--tesla-radius);
  font-size: 12px;
  font-weight: 600;
}
.result-badge--pass {
  background: rgba(24, 160, 88, 0.1);
  color: var(--tesla-success);
}
.result-badge--fail {
  background: rgba(208, 48, 80, 0.1);
  color: var(--tesla-error);
}

.diff-view {
  margin-top: 4px;
}

.diff-columns {
  display: flex;
  gap: 1px;
  font-family: 'SF Mono', 'Cascadia Code', monospace;
  font-size: 12px;
  border: 1px solid var(--tesla-cloud);
  border-radius: var(--tesla-radius);
  overflow: hidden;
}

.diff-col {
  flex: 1;
  background: var(--tesla-ash);
}

.diff-col__header {
  padding: 6px 12px;
  background: var(--tesla-cloud);
  font-weight: 500;
  font-size: 11px;
  color: var(--tesla-pewter);
}

.diff-line {
  display: flex;
  padding: 3px 12px;
  min-height: 22px;
  border-bottom: 1px solid rgba(0,0,0,0.04);
  background: var(--tesla-white);
  align-items: center;
}

.diff-line--mismatch {
  background: #FEF2F2;
}
</style>
