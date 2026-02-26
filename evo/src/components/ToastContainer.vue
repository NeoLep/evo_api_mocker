<script setup lang="ts">
import { useToast } from '../composables/useToast';

const { toasts, removeToast } = useToast();
</script>

<template>
  <div class="toast-container">
    <transition-group name="toast">
      <div
        v-for="toast in toasts"
        :key="toast.id"
        class="toast-item"
        :class="`toast-${toast.type}`"
        @click="removeToast(toast.id)"
      >
        <div class="toast-icon">
          <span v-if="toast.type === 'success'">✓</span>
          <span v-else-if="toast.type === 'error'">✕</span>
          <span v-else-if="toast.type === 'warning'">!</span>
          <span v-else>ℹ</span>
        </div>
        <div class="toast-message">{{ toast.message }}</div>
        <button class="toast-close">×</button>
      </div>
    </transition-group>
  </div>
</template>

<style scoped>
.toast-container {
  position: fixed;
  top: 20px;
  right: 20px;
  z-index: 9999;
  display: flex;
  flex-direction: column;
  gap: 10px;
  pointer-events: none;
}

.toast-item {
  display: flex;
  align-items: center;
  min-width: 300px;
  max-width: 400px;
  padding: 12px 16px;
  border-radius: 8px;
  background: white;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  pointer-events: auto;
  cursor: pointer;
  transition: all 0.3s ease;
  border-left: 4px solid transparent;
}

.toast-success {
  border-left-color: var(--success-color, #10b981);
}

.toast-error {
  border-left-color: var(--danger-color, #ef4444);
}

.toast-warning {
  border-left-color: var(--warning-color, #f59e0b);
}

.toast-info {
  border-left-color: var(--primary-color, #4f46e5);
}

.toast-icon {
  margin-right: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border-radius: 50%;
  font-weight: bold;
  font-size: 14px;
  color: white;
}

.toast-success .toast-icon {
  background-color: var(--success-color, #10b981);
}

.toast-error .toast-icon {
  background-color: var(--danger-color, #ef4444);
}

.toast-warning .toast-icon {
  background-color: var(--warning-color, #f59e0b);
}

.toast-info .toast-icon {
  background-color: var(--primary-color, #4f46e5);
}

.toast-message {
  flex: 1;
  font-size: 14px;
  color: #333;
  line-height: 1.4;
}

.toast-close {
  background: transparent;
  border: none;
  color: #999;
  font-size: 20px;
  cursor: pointer;
  padding: 0;
  margin-left: 10px;
  line-height: 1;
}

.toast-close:hover {
  color: #333;
}

/* Transitions */
.toast-enter-active,
.toast-leave-active {
  transition: all 0.3s ease;
}

.toast-enter-from {
  opacity: 0;
  transform: translateX(30px);
}

.toast-leave-to {
  opacity: 0;
  transform: translateX(30px);
}
</style>
