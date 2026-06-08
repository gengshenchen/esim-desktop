<script setup lang="ts">
import { ref, computed } from 'vue'
import {
  NCard, NSpace, NButton, NInput, NAlert, NRadioGroup, NRadioButton,
  NProgress, NTag, NDivider, NSelect, NModal, useMessage, useDialog,
} from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import { useDeviceStore } from '@/stores/device'

const device = useDeviceStore()
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

// Template state
const templates = ref<ConfigTemplate[]>([])
const selectedTemplate = ref<string | null>(null)
const showSaveTemplate = ref(false)
const newTemplateName = ref('')
const newTemplateDesc = ref('')

const templateOptions = computed(() =>
  templates.value.map(t => ({ label: `${t.name} — ${t.description}`, value: t.name }))
)

// Diff computation
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
  <div>
    <NAlert v-if="!device.connected" type="warning" style="margin-bottom: 16px;">
      请先连接设备
    </NAlert>

    <NCard title="配置管理" size="small">
      <NSpace vertical>
        <NSpace align="center">
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
            <NButton size="small" type="error" quaternary @click="deleteSelectedTemplate" :disabled="!selectedTemplate">
              删除
            </NButton>
          </template>

          <NButton v-if="source === 'file'" size="small" @click="handleFileImport">
            选择文件...
          </NButton>
        </NSpace>

        <NInput
          v-model:value="editorContent"
          type="textarea"
          :rows="12"
          placeholder="version=1&#10;server=192.168.1.100&#10;port=9000&#10;apn=cmnet&#10;volume=8"
          style="font-family: monospace;"
        />

        <NSpace>
          <NButton type="primary" @click="pushToDevice" :loading="uploading" :disabled="!device.connected">
            推送到设备
          </NButton>
          <NButton @click="readFromDevice" :disabled="!device.connected">
            从设备回读
          </NButton>
          <NButton @click="restoreDefault" :disabled="!device.connected">
            恢复默认
          </NButton>
          <NButton @click="clearConfig" :disabled="!device.connected">
            清空设备配置
          </NButton>
          <NButton @click="showSaveTemplate = true" :disabled="!editorContent.trim()">
            另存为模板
          </NButton>
        </NSpace>

        <div v-if="uploading">
          <NProgress
            type="line"
            :percentage="uploadTotal > 0 ? Math.round((uploadProgress / uploadTotal) * 100) : 0"
          />
          <span style="font-size: 12px;">{{ uploadProgress }}/{{ uploadTotal }} 行</span>
        </div>

        <div v-if="diffMatch !== null" style="margin-top: 8px;">
          <NTag :type="diffMatch ? 'success' : 'error'" size="small">
            {{ diffMatch ? '回读校验: 与推送内容一致' : '回读校验: 内容不一致' }}
          </NTag>
        </div>

        <!-- Diff view -->
        <div v-if="diffLines.length > 0" style="margin-top: 8px;">
          <NDivider>对比视图（推送 vs 回读）</NDivider>
          <div style="display: flex; gap: 1px; font-family: monospace; font-size: 12px; border: 1px solid #eee; border-radius: 4px; overflow: hidden;">
            <div style="flex: 1; background: #fafafa;">
              <div style="padding: 4px 8px; background: #f0f0f0; font-weight: bold; font-size: 11px;">推送内容</div>
              <div
                v-for="(line, idx) in diffLines"
                :key="'p' + idx"
                style="padding: 2px 8px; min-height: 20px; border-bottom: 1px solid #f5f5f5;"
                :style="{ background: line.match ? '#fff' : '#fff0f0' }"
              >
                {{ line.pushed }}
              </div>
            </div>
            <div style="flex: 1; background: #fafafa;">
              <div style="padding: 4px 8px; background: #f0f0f0; font-weight: bold; font-size: 11px;">回读内容</div>
              <div
                v-for="(line, idx) in diffLines"
                :key="'r' + idx"
                style="padding: 2px 8px; min-height: 20px; border-bottom: 1px solid #f5f5f5;"
                :style="{ background: line.match ? '#fff' : '#fff0f0' }"
              >
                <NSpace align="center" :wrap="false">
                  <span style="flex: 1;">{{ line.readback }}</span>
                  <span v-if="line.match" style="color: #18a058; font-size: 11px;">&#10003;</span>
                  <span v-else style="color: #d03050; font-size: 11px;">&#10007;</span>
                </NSpace>
              </div>
            </div>
          </div>
        </div>

        <!-- Readback only (no diff) -->
        <div v-else-if="readbackContent && !pushedContent" style="margin-top: 8px;">
          <NDivider>回读内容</NDivider>
          <NInput
            :value="readbackContent"
            type="textarea"
            :rows="8"
            readonly
            style="font-family: monospace;"
          />
        </div>
      </NSpace>
    </NCard>

    <!-- Save template modal -->
    <NModal v-model:show="showSaveTemplate" preset="card" title="保存为模板" style="width: 400px;">
      <NSpace vertical>
        <NInput v-model:value="newTemplateName" placeholder="模板名称" />
        <NInput v-model:value="newTemplateDesc" placeholder="模板描述（可选）" />
        <NSpace justify="end">
          <NButton @click="showSaveTemplate = false">取消</NButton>
          <NButton type="primary" @click="doSaveTemplate">保存</NButton>
        </NSpace>
      </NSpace>
    </NModal>
  </div>
</template>
