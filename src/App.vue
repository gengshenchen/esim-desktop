<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { NConfigProvider, NLayout, NLayoutContent, NMessageProvider, NDialogProvider } from 'naive-ui'
import type { GlobalThemeOverrides } from 'naive-ui'
import { getCurrentWindow, LogicalSize, LogicalPosition, availableMonitors } from '@tauri-apps/api/window'
import { invoke } from '@tauri-apps/api/core'
import AppHeader from '@/components/layout/AppHeader.vue'
import AppFooter from '@/components/layout/AppFooter.vue'

const WINDOW_STATE_KEY = 'window_state'

interface WindowState {
  x: number
  y: number
  width: number
  height: number
}

function loadWindowState(): WindowState | null {
  try {
    const raw = localStorage.getItem(WINDOW_STATE_KEY)
    if (!raw) return null
    return JSON.parse(raw)
  } catch { return null }
}

function saveWindowState(state: WindowState) {
  localStorage.setItem(WINDOW_STATE_KEY, JSON.stringify(state))
}

let saveTimer: ReturnType<typeof setInterval> | null = null

async function startPositionSaver() {
  saveTimer = setInterval(async () => {
    try {
      const win = getCurrentWindow()
      const pos = await win.outerPosition()
      const size = await win.outerSize()
      const factor = await win.scaleFactor()
      saveWindowState({
        x: Math.round(pos.x / factor),
        y: Math.round(pos.y / factor),
        width: Math.round(size.width / factor),
        height: Math.round(size.height / factor),
      })
    } catch {}
  }, 2000)
}

async function autoResizeWindow() {
  try {
    const win = getCurrentWindow()

    const monitors = await availableMonitors()
    const scaleFactor = monitors.length > 0 ? monitors[0].scaleFactor : 1
    const screenW = monitors.length > 0 ? Math.floor(monitors[0].size.width / scaleFactor) : 1920
    const screenH = monitors.length > 0 ? Math.floor(monitors[0].size.height / scaleFactor) : 1080

    await win.setMinSize(new LogicalSize(720, 520))

    const saved = loadWindowState()
    if (saved && saved.x >= 0 && saved.y >= 0 && saved.x < screenW && saved.y < screenH) {
      await win.setSize(new LogicalSize(saved.width, saved.height))
      await win.setPosition(new LogicalPosition(saved.x, saved.y))
    } else {
      const w = 960
      const h = 680
      await win.setSize(new LogicalSize(w, h))
      const x = Math.round((screenW - w) / 2)
      const y = Math.round((screenH - h) / 2)
      await win.setPosition(new LogicalPosition(Math.max(0, x), Math.max(0, y)))
    }

    await win.show()
    startPositionSaver()
  } catch {
    const win = getCurrentWindow()
    await win.show().catch(() => {})
  }
}


const themeOverrides: GlobalThemeOverrides = {
  common: {
    primaryColor: '#3E6AE1',
    primaryColorHover: '#5A82E8',
    primaryColorPressed: '#2D54C7',
    primaryColorSuppl: '#3E6AE1',
    infoColor: '#3E6AE1',
    infoColorHover: '#5A82E8',
    infoColorPressed: '#2D54C7',
    successColor: '#18A058',
    successColorHover: '#36AD6A',
    successColorPressed: '#0C7A43',
    errorColor: '#D03050',
    errorColorHover: '#DE576D',
    errorColorPressed: '#AB1F3F',
    warningColor: '#F0A020',
    warningColorHover: '#FCB040',
    warningColorPressed: '#C97C10',
    bodyColor: '#FFFFFF',
    cardColor: '#FFFFFF',
    modalColor: '#FFFFFF',
    tableColor: '#FFFFFF',
    popoverColor: '#FFFFFF',
    inputColor: '#FFFFFF',
    textColorBase: '#171A20',
    textColor1: '#171A20',
    textColor2: '#393C41',
    textColor3: '#5C5E62',
    textColorDisabled: '#8E8E8E',
    placeholderColor: '#8E8E8E',
    borderColor: '#EEEEEE',
    dividerColor: '#EEEEEE',
    borderRadius: '4px',
    borderRadiusSmall: '4px',
    fontSize: '14px',
    fontSizeSmall: '13px',
    fontSizeMini: '12px',
    heightMedium: '36px',
    heightSmall: '32px',
    heightTiny: '28px',
  },
  Button: {
    borderRadiusMedium: '4px',
    borderRadiusSmall: '4px',
    borderRadiusTiny: '4px',
    fontWeightStrong: '500',
    textColorPrimary: '#FFFFFF',
    colorPrimary: '#3E6AE1',
    colorHoverPrimary: '#5A82E8',
    colorPressedPrimary: '#2D54C7',
    borderPrimary: 'none',
    borderHoverPrimary: 'none',
    borderPressedPrimary: 'none',
    textColor: '#393C41',
    color: '#FFFFFF',
    colorHover: '#FFFFFF',
    colorPressed: '#F4F4F4',
    border: '1px solid #EEEEEE',
    borderHover: '1px solid #D0D1D2',
    borderPressed: '1px solid #D0D1D2',
  },
  Card: {
    borderRadius: '8px',
    borderColor: '#EEEEEE',
    boxShadow: 'none',
    titleFontWeight: '500',
    titleTextColor: '#171A20',
    titleFontSizeSmall: '15px',
  },
  Tag: {
    borderRadius: '4px',
    fontSizeSmall: '12px',
  },
  Modal: {
    borderRadius: '8px',
    boxShadow: '0 4px 24px rgba(0,0,0,0.08)',
  },
  Input: {
    borderRadius: '4px',
    border: '1px solid #EEEEEE',
    borderHover: '1px solid #D0D1D2',
    borderFocus: '1px solid #3E6AE1',
    boxShadowFocus: '0 0 0 2px rgba(62,106,225,0.1)',
    color: '#FFFFFF',
    caretColor: '#3E6AE1',
    placeholderColor: '#8E8E8E',
  },
  Select: {
    peers: {
      InternalSelection: {
        borderRadius: '4px',
        border: '1px solid #EEEEEE',
        borderHover: '1px solid #D0D1D2',
        borderFocus: '1px solid #3E6AE1',
        borderActive: '1px solid #3E6AE1',
        boxShadowFocus: '0 0 0 2px rgba(62,106,225,0.1)',
        boxShadowActive: '0 0 0 2px rgba(62,106,225,0.1)',
      },
    },
  },
  Switch: {
    railColorActive: '#3E6AE1',
  },
  Scrollbar: {
    color: 'rgba(0,0,0,0.15)',
    colorHover: 'rgba(0,0,0,0.25)',
  },
  Alert: {
    borderRadius: '4px',
  },
  Divider: {
    color: '#EEEEEE',
    textColor: '#5C5E62',
  },
  Collapse: {
    titleTextColor: '#393C41',
    titleFontWeight: '500',
    titleFontSize: '13px',
    dividerColor: '#EEEEEE',
  },
  Form: {
    labelTextColor: '#5C5E62',
    feedbackTextColor: '#8E8E8E',
  },
  Descriptions: {
    borderColor: '#EEEEEE',
    thColor: '#F4F4F4',
    tdColor: '#FFFFFF',
    thTextColor: '#5C5E62',
    tdTextColor: '#393C41',
    titleTextColor: '#171A20',
    borderRadius: '4px',
  },
  Progress: {
    fillColor: '#3E6AE1',
    railColor: '#EEEEEE',
  },
  Dialog: {
    borderRadius: '8px',
    titleFontWeight: '500',
    titleTextColor: '#171A20',
  },
}

onMounted(() => {
  document.addEventListener('contextmenu', (e) => e.preventDefault())
  autoResizeWindow()
  enableDragScroll()
})

onUnmounted(() => {
  if (saveTimer) clearInterval(saveTimer)
})

function enableDragScroll() {
  let target: HTMLElement | null = null
  let startX = 0
  let startY = 0
  let scrollLeft = 0
  let scrollTop = 0
  let dragging = false

  function findScrollable(el: HTMLElement | null): HTMLElement | null {
    while (el && el !== document.body) {
      const style = getComputedStyle(el)
      const overflowY = style.overflowY
      const overflowX = style.overflowX
      if ((overflowY === 'auto' || overflowY === 'scroll') && el.scrollHeight > el.clientHeight) return el
      if ((overflowX === 'auto' || overflowX === 'scroll') && el.scrollWidth > el.clientWidth) return el
      el = el.parentElement
    }
    return null
  }

  document.addEventListener('mousedown', (e) => {
    if (e.button !== 0) return
    const tag = (e.target as HTMLElement).tagName
    if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' || tag === 'BUTTON' || tag === 'A') return
    if ((e.target as HTMLElement).closest('button, a, .n-button, .n-select, .n-input')) return

    const scrollable = findScrollable(e.target as HTMLElement)
    if (!scrollable) return

    target = scrollable
    startX = e.clientX
    startY = e.clientY
    scrollLeft = scrollable.scrollLeft
    scrollTop = scrollable.scrollTop
    dragging = false
  })

  document.addEventListener('mousemove', (e) => {
    if (!target) return
    const dx = e.clientX - startX
    const dy = e.clientY - startY
    if (!dragging && Math.abs(dx) < 3 && Math.abs(dy) < 3) return
    dragging = true
    target.style.cursor = 'grabbing'
    target.style.userSelect = 'none'
    target.scrollLeft = scrollLeft - dx
    target.scrollTop = scrollTop - dy
  })

  document.addEventListener('mouseup', () => {
    if (target) {
      target.style.cursor = ''
      target.style.userSelect = ''
    }
    target = null
    dragging = false
  })
}
</script>

<template>
  <NConfigProvider :theme-overrides="themeOverrides">
    <NMessageProvider>
      <NDialogProvider>
        <NLayout position="absolute" style="background: #FFFFFF;">
          <header class="app-header">
            <AppHeader />
          </header>
          <NLayoutContent
            content-style="padding: 20px 24px; height: 100%; box-sizing: border-box; display: flex; flex-direction: column;"
            style="top: 52px; bottom: 32px;"
            position="absolute"
          >
            <router-view style="flex: 1; min-height: 0;" />
          </NLayoutContent>
          <footer class="app-footer">
            <AppFooter />
          </footer>
        </NLayout>
      </NDialogProvider>
    </NMessageProvider>
  </NConfigProvider>
</template>

<style>
:root {
  --tesla-blue: #3E6AE1;
  --tesla-blue-hover: #5A82E8;
  --tesla-blue-light: rgba(62, 106, 225, 0.08);
  --tesla-carbon: #171A20;
  --tesla-graphite: #393C41;
  --tesla-pewter: #5C5E62;
  --tesla-silver: #8E8E8E;
  --tesla-cloud: #EEEEEE;
  --tesla-pale: #D0D1D2;
  --tesla-ash: #F4F4F4;
  --tesla-white: #FFFFFF;
  --tesla-success: #18A058;
  --tesla-error: #D03050;
  --tesla-warning: #F0A020;
  --tesla-radius: 4px;
  --tesla-radius-lg: 8px;
  --tesla-transition: 0.33s ease;
}

body {
  margin: 0;
  font-family: 'Inter', 'SF Pro Text', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  -webkit-user-select: none;
  user-select: none;
  color: var(--tesla-graphite);
  background: var(--tesla-white);
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}
body * {
  -webkit-user-select: none;
  user-select: none;
}
input, textarea, [contenteditable], .log-selectable, .log-selectable * {
  -webkit-user-select: text;
  user-select: text;
}

.app-header {
  height: 52px;
  padding: 0 24px;
  border-bottom: 1px solid var(--tesla-cloud);
  background: var(--tesla-white);
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  z-index: 10;
}

.app-footer {
  height: 32px;
  padding: 0 24px;
  border-top: 1px solid var(--tesla-cloud);
  background: var(--tesla-white);
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  display: flex;
  align-items: center;
}

/* Tesla-style tables */
.tesla-table {
  width: 100%;
  border-collapse: separate;
  border-spacing: 0;
  font-size: 13px;
}
.tesla-table thead th {
  padding: 8px 12px;
  text-align: left;
  font-weight: 500;
  font-size: 12px;
  color: var(--tesla-pewter);
  text-transform: uppercase;
  letter-spacing: 0.02em;
  border-bottom: 1px solid var(--tesla-cloud);
  background: var(--tesla-white);
  position: sticky;
  top: 0;
  z-index: 1;
}
.tesla-table tbody tr {
  transition: background var(--tesla-transition);
}
.tesla-table tbody tr:nth-child(even) {
  background: var(--tesla-ash);
}
.tesla-table tbody tr:hover {
  background: var(--tesla-blue-light);
}
.tesla-table tbody td {
  padding: 8px 12px;
  color: var(--tesla-graphite);
}
.tesla-table tbody td.mono {
  font-family: 'SF Mono', 'Cascadia Code', 'Fira Code', monospace;
  font-size: 12px;
}

/* Status dots */
.status-dot {
  display: inline-block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  margin-right: 6px;
  vertical-align: middle;
}
.status-dot--pass { background: var(--tesla-success); }
.status-dot--fail { background: var(--tesla-error); }
.status-dot--running { background: var(--tesla-warning); animation: pulse-dot 1.5s ease-in-out infinite; }
.status-dot--pending { background: var(--tesla-pale); }
.status-dot--info { background: var(--tesla-blue); }

@keyframes pulse-dot {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

/* Domain badge */
.domain-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  font-weight: 500;
}
.domain-badge--modem { color: var(--tesla-blue); }
.domain-badge--mcu { color: var(--tesla-warning); }
.domain-badge .domain-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
}
.domain-badge--modem .domain-dot { background: var(--tesla-blue); }
.domain-badge--mcu .domain-dot { background: var(--tesla-warning); }

/* Stats */
.stat-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 8px 16px;
}
.stat-card__value {
  font-size: 20px;
  font-weight: 600;
  color: var(--tesla-carbon);
  line-height: 1.2;
}
.stat-card__label {
  font-size: 11px;
  color: var(--tesla-pewter);
  margin-top: 2px;
}

/* Scrollbar */
::-webkit-scrollbar {
  width: 10px;
  height: 10px;
}
::-webkit-scrollbar-track {
  background: transparent;
}
::-webkit-scrollbar-thumb {
  background: rgba(0,0,0,0.15);
  border-radius: 5px;
  border: 2px solid transparent;
  background-clip: padding-box;
}
::-webkit-scrollbar-thumb:hover {
  background: rgba(0,0,0,0.3);
  border: 2px solid transparent;
  background-clip: padding-box;
}

/* Grab-scrollable areas */
.grab-scroll {
  cursor: grab;
}
.grab-scroll:active {
  cursor: grabbing;
}
</style>
