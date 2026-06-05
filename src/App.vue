<script setup lang="ts">
import { NConfigProvider, NLayout, NLayoutHeader, NLayoutContent, NLayoutFooter, NMessageProvider, NDialogProvider } from 'naive-ui'
import AppHeader from '@/components/layout/AppHeader.vue'
import AppFooter from '@/components/layout/AppFooter.vue'
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
            content-style="padding: 16px;"
            style="top: 56px; bottom: 32px;"
            position="absolute"
          >
            <router-view />
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
}
</style>
