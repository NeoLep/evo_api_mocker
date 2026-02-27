import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useToast } from './useToast';
import { useI18n } from 'vue-i18n';
import { useConfirm } from './useConfirm';

export interface ServerConfig {
  port: number;
  host: string;
  running: boolean;
}

const serverConfig = ref<ServerConfig>({ port: 3000, host: '127.0.0.1', running: true });
const serverStatus = ref(false);

export function useServer() {
  const toast = useToast();
  const { t } = useI18n();
  const { confirm: showConfirm } = useConfirm();

  async function fetchServerConfig() {
    try {
      const result = await invoke<ServerConfig>("get_server_config");
      serverConfig.value = result;
      serverStatus.value = result.running; // Initial assumption
    } catch (error) {
      console.error("Failed to fetch server config:", error);
    }
  }

  async function handleSaveSettings() {
    try {
      await invoke("update_server_config", { config: serverConfig.value });
      toast.success(t('settingsSaved'));
      
      // Ask to restart
      if (await showConfirm(t('restartServerConfirm'))) {
          try {
              await invoke("restart_server");
              // Sync status with config
              serverStatus.value = serverConfig.value.running;
              toast.success(t('serverRestarted'));
          } catch (e) {
              toast.error(t('serverRestartFailed', { error: String(e) }));
          }
      }
    } catch (error) {
      console.error("Failed to save settings:", error);
      toast.error(t('settingsSaveFailed', { error: String(error) }));
    }
  }

  async function toggleServer() {
      try {
          if (serverStatus.value) {
              await invoke("stop_server");
              serverStatus.value = false;
              toast.info(t('serverStopped'));
          } else {
              await invoke("start_server_cmd");
              serverStatus.value = true;
              toast.success(t('serverStarted'));
          }
          
          // Update config silently to persist state
          serverConfig.value.running = serverStatus.value;
          await invoke("update_server_config", { config: serverConfig.value });
          
      } catch (error) {
          console.error("Failed to toggle server:", error);
          toast.error(String(error));
      }
  }

  return {
    serverConfig,
    serverStatus,
    fetchServerConfig,
    handleSaveSettings,
    toggleServer
  };
}