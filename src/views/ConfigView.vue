<script setup lang="ts">
import { ref } from 'vue'
import {
  NCard, NSpace, NButton, NInput, NAlert, NRadioGroup, NRadioButton,
  NProgress, NTag, NDivider,
} from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import { useDeviceStore } from '@/stores/device'

const device = useDeviceStore()

const source = ref<'edit' | 'template' | 'file'>('edit')
const editorContent = ref('')
const uploadProgress = ref(0)
const uploadTotal = ref(0)
const uploading = ref(false)
const readbackContent = ref('')
const diffMatch = ref<boolean | null>(null)

async function readFromDevice() {
  try {
    const lines = await invoke<string[]>('config_read')
    editorContent.value = lines.join('\n')
    readbackContent.value = ''
    diffMatch.value = null
  } catch (e: any) {
    console.error('config_read failed:', e)
  }
}

async function pushToDevice() {
  if (!editorContent.value.trim() && editorContent.value.length > 0) return
  uploading.value = true
  uploadProgress.value = 0

  const lines = editorContent.value.split('\n')
  uploadTotal.value = lines.length

  try {
    const result = await invoke<{ lines_sent: number; readback: string[] }>('config_upload', {
      lines,
    })

    readbackContent.value = result.readback.join('\n')
    const pushed = editorContent.value.trim()
    const readback = readbackContent.value.trim()
    diffMatch.value = pushed === readback
    uploadProgress.value = uploadTotal.value
  } catch (e: any) {
    console.error('config_upload failed:', e)
  }

  uploading.value = false
}

async function restoreDefault() {
  try {
    await invoke('config_restore_default')
    editorContent.value = ''
    readbackContent.value = ''
    diffMatch.value = null
  } catch (e: any) {
    console.error('config_restore_default failed:', e)
  }
}

async function clearConfig() {
  try {
    await invoke('config_clear')
    editorContent.value = ''
    readbackContent.value = ''
    diffMatch.value = null
  } catch (e: any) {
    console.error('config_clear failed:', e)
  }
}
</script>

<template>
  <div>
    <NAlert v-if="!device.connected" type="warning" style="margin-bottom: 16px;">
      请先连接设备
    </NAlert>

    <NCard title="配置管理" size="small">
      <NSpace vertical>
        <NRadioGroup v-model:value="source" size="small">
          <NRadioButton value="edit">手动编辑</NRadioButton>
          <NRadioButton value="template">从模板</NRadioButton>
          <NRadioButton value="file">从文件导入</NRadioButton>
        </NRadioGroup>

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

        <div v-if="readbackContent" style="margin-top: 8px;">
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
  </div>
</template>
