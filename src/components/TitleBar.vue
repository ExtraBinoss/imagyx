<script setup lang="ts">
import { ref } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { Minus, Square, X } from '@lucide/vue'

const appWindow = getCurrentWindow()
const isMaximized = ref(false)

async function minimize(event: MouseEvent) {
  event.stopPropagation()
  event.preventDefault()
  await appWindow.minimize()
}

async function toggleMaximize(event: MouseEvent) {
  event.stopPropagation()
  event.preventDefault()
  await appWindow.toggleMaximize()
  isMaximized.value = await appWindow.isMaximized()
}

async function close(event: MouseEvent) {
  event.stopPropagation()
  event.preventDefault()
  await appWindow.close()
}
</script>

<template>
  <div class="titlebar" data-tauri-drag-region>
    <div class="titlebar-traffic-lights" @click.stop @pointerdown.stop>
      <button class="mac-btn mac-close" title="Close" @click="close" @pointerdown.stop>
        <X :size="9" class="mac-icon" />
      </button>
      <button class="mac-btn mac-minimize" title="Minimize" @click="minimize" @pointerdown.stop>
        <Minus :size="9" class="mac-icon" />
      </button>
      <button class="mac-btn mac-maximize" title="Maximize" @click="toggleMaximize" @pointerdown.stop>
        <Square :size="8" class="mac-icon" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.titlebar {
  display: flex;
  align-items: center;
  height: 38px;
  padding: 0 12px;
  user-select: none;
  background: transparent;
  cursor: default;
}

.titlebar-traffic-lights {
  display: flex;
  align-items: center;
  gap: 8px;
}

.mac-btn {
  display: grid;
  place-items: center;
  width: 14px;
  height: 14px;
  padding: 0;
  border: none;
  border-radius: 50%;
  cursor: pointer;
  transition: all 0.15s cubic-bezier(0.16, 1, 0.3, 1);
}

.mac-btn .mac-icon {
  opacity: 0;
  transition: opacity 0.15s ease;
  color: rgba(0, 0, 0, 0.75);
}

/* Hovering the traffic lights group scales all 3 buttons smoothly like macOS */
.titlebar-traffic-lights:hover .mac-btn {
  transform: scale(1.1);
}

.titlebar-traffic-lights:hover .mac-icon {
  opacity: 1;
}

.mac-close {
  background: #ff5f56;
  border: 0.5px solid #e0443e;
}
.mac-minimize {
  background: #ffbd2e;
  border: 0.5px solid #dea123;
}
.mac-maximize {
  background: #27c93f;
  border: 0.5px solid #1aab29;
}

.mac-btn:hover {
  filter: brightness(0.88);
  transform: scale(1.2) !important;
}
</style>
