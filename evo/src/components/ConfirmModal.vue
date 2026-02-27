<script setup lang="ts">
import { useConfirm } from '../composables/useConfirm';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();
const { state, handleConfirm, handleCancel } = useConfirm();
</script>

<template>
  <div v-if="state.show" class="modal-overlay" @click.self="handleCancel">
    <div class="modal-content">
      <div class="modal-header">
        <h3>{{ state.title || t('confirmTitle') }}</h3>
      </div>
      <div class="modal-body">
        <p>{{ state.message }}</p>
      </div>
      <div class="modal-footer">
        <button class="cancel-btn" @click="handleCancel">{{ t('cancel') }}</button>
        <button class="confirm-btn" @click="handleConfirm">{{ t('confirm') }}</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background-color: rgba(0, 0, 0, 0.5);
  display: flex;
  justify-content: center;
  align-items: center;
  z-index: 1000;
  animation: fadeIn 0.2s ease-out;
}

.modal-content {
  background-color: white;
  padding: 1.5rem;
  border-radius: 8px;
  width: 90%;
  max-width: 400px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  animation: slideIn 0.2s ease-out;
}

.modal-header h3 {
  margin: 0 0 1rem 0;
  color: var(--text-primary, #333);
  font-size: 1.25rem;
}

.modal-body {
  margin-bottom: 1.5rem;
  color: var(--text-secondary, #666);
  line-height: 1.5;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
}

.cancel-btn {
  background-color: white;
  color: var(--text-secondary, #666);
  border: 1px solid var(--border-color, #e5e7eb);
  padding: 0.5rem 1rem;
  border-radius: 4px;
  cursor: pointer;
}

.cancel-btn:hover {
  background-color: var(--bg-color, #f3f4f6);
}

.confirm-btn {
  background-color: var(--primary-color, #4f46e5);
  color: white;
  border: none;
  padding: 0.5rem 1rem;
  border-radius: 4px;
  cursor: pointer;
}

.confirm-btn:hover {
  opacity: 0.9;
}

@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

@keyframes slideIn {
  from { transform: translateY(-20px); opacity: 0; }
  to { transform: translateY(0); opacity: 1; }
}
</style>
