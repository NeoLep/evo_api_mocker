import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useToast } from './useToast';
import { useI18n } from 'vue-i18n';
import { useConfirm } from './useConfirm';

// Global logs state to persist across tab switches
const requestLogs = ref<any[]>([]);
let isListening = false;

export function useLogs() {
  const { t } = useI18n();
  const toast = useToast();
  const { confirm: showConfirm } = useConfirm();
  
  async function fetchLogs() {
    try {
        const result = await invoke<any[]>("get_request_logs");
        // Sort by timestamp desc
        requestLogs.value = result.sort((a, b) => b.timestamp - a.timestamp);
    } catch (error) {
        console.error("Failed to fetch logs:", error);
    }
  }

  async function clearLogs() {
    if (await showConfirm(t('confirmClearLogs'))) {
        try {
            await invoke("clear_request_logs");
            requestLogs.value = [];
            toast.success(t('logsCleared'));
        } catch (error) {
            console.error("Failed to clear logs:", error);
            toast.error(String(error));
        }
    }
  }

  async function startListening() {
    if (isListening) return; // Already listening globally
    
    isListening = true;
    await listen('new-request-log', (event: any) => {
      // Check if log already exists to avoid duplication (though unshift should be fine if event is unique)
      // But mainly, we just push to the global state
      requestLogs.value.unshift(event.payload);
      if (requestLogs.value.length > 100) {
          requestLogs.value.pop();
      }
    });
  }
  
  return {
    requestLogs,
    fetchLogs,
    clearLogs,
    startListening
  };
}