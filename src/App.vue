<script setup lang="ts">
import { onMounted } from 'vue'
import { NConfigProvider, NLayout, NLayoutHeader, NLayoutContent, NLayoutFooter, NMessageProvider, NDialogProvider } from 'naive-ui'
import AppHeader from '@/components/layout/AppHeader.vue'
import AppFooter from '@/components/layout/AppFooter.vue'

onMounted(() => {
  document.addEventListener('contextmenu', (e) => e.preventDefault())
})
</script>

<template>
  <NConfigProvider>
    <NMessageProvider>
      <NDialogProvider>
        <NLayout position="absolute">
          <NLayoutHeader bordered style="height: 56px; padding: 0 16px;">
            <AppHeader />
          </NLayoutHeader>
          <NLayoutContent
            content-style="padding: 16px; height: 100%; box-sizing: border-box; display: flex; flex-direction: column;"
            style="top: 56px; bottom: 32px;"
            position="absolute"
          >
            <router-view style="flex: 1; min-height: 0;" />
          </NLayoutContent>
          <NLayoutFooter
            bordered
            position="absolute"
            style="height: 32px; padding: 0 16px; display: flex; align-items: center;"
          >
            <AppFooter />
          </NLayoutFooter>
        </NLayout>
      </NDialogProvider>
    </NMessageProvider>
  </NConfigProvider>
</template>

<style>
body {
  margin: 0;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  -webkit-user-select: none;
  user-select: none;
}
body * {
  -webkit-user-select: none;
  user-select: none;
}
input, textarea, [contenteditable], .log-selectable, .log-selectable * {
  -webkit-user-select: text;
  user-select: text;
}
</style>
