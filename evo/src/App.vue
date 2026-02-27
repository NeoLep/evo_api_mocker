<script setup lang="ts">
import { ref, computed } from "vue";
import { useI18n } from "vue-i18n";
import ToastContainer from "./components/ToastContainer.vue";
import ConfirmModal from "./components/ConfirmModal.vue";
import MocksView from "./components/MocksView.vue";
import DatabaseView from "./components/DatabaseView.vue";
import LogsView from "./components/LogsView.vue";
import SettingsView from "./components/SettingsView.vue";
import HelpView from "./components/HelpView.vue";
import { useLogs } from "./composables/useLogs";
import "./App.css";

const { t, locale } = useI18n();
const { startListening } = useLogs();

// Start listening for logs globally
startListening();

const activeTab = ref<'mocks' | 'database' | 'help' | 'settings' | 'logs'>('mocks');

const currentTabComponent = computed(() => {
  switch (activeTab.value) {
    case 'mocks': return MocksView;
    case 'database': return DatabaseView;
    case 'logs': return LogsView;
    case 'settings': return SettingsView;
    case 'help': return HelpView;
    default: return MocksView;
  }
});

</script>

<template>
  <ToastContainer />
  <ConfirmModal />
  <div class="container">
    <div class="header">
        <h1>{{ t('apiManager') }}</h1>
        <div class="header-controls">
            <div class="language-selector">
                <select v-model="locale">
                    <option value="zh">中文</option>
                    <option value="en">English</option>
                    <option value="ja">日本語</option>
                </select>
            </div>
            <div class="tabs">
                <button :class="{ active: activeTab === 'mocks' }" @click="activeTab = 'mocks'">{{ t('mocks') }}</button>
                <button :class="{ active: activeTab === 'database' }" @click="activeTab = 'database'">{{ t('database') }}</button>
                <button :class="{ active: activeTab === 'logs' }" @click="activeTab = 'logs'">{{ t('logs') }}</button>
                <button :class="{ active: activeTab === 'settings' }" @click="activeTab = 'settings'">{{ t('settings') }}</button>
                <button :class="{ active: activeTab === 'help' }" @click="activeTab = 'help'">{{ t('help') }}</button>
            </div>
        </div>
    </div>
    
    <KeepAlive>
      <div class="content-wrapper">
        <component :is="currentTabComponent" />
      </div>
    </KeepAlive>
  </div>
</template>

<style scoped>
.container {
  display: flex;
  flex-direction: column;
  height: 100vh;
  padding: 1rem;
  overflow: hidden;
}

.content-wrapper {
  flex: 1;
  overflow: auto;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.header {
  flex-shrink: 0;
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1rem;
  padding-bottom: 0.5rem;
  border-bottom: 1px solid var(--border-color);
  width: 100%;
}

.header h1 {
  margin: 0;
  font-size: 1.5rem;
  color: var(--primary-color);
}

.header-controls {
  display: flex;
  align-items: center;
  gap: 1rem;
}

.language-selector select {
  padding: 0.375rem;
  border-radius: var(--radius-md);
  border: 1px solid var(--border-color);
  background-color: white;
  color: var(--text-primary);
  font-size: 0.875rem;
}

.tabs {
  display: flex;
  background-color: white;
  padding: 0.25rem;
  border-radius: var(--radius-md);
  border: 1px solid var(--border-color);
  gap: 0.25rem;
}

.tabs button {
  background: transparent;
  color: var(--text-secondary);
  border: none;
  margin: 0;
  padding: 0.375rem 1rem;
  border-radius: var(--radius-md);
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.tabs button:hover {
  color: var(--text-primary);
  background-color: var(--bg-color);
}

.tabs button.active {
  background: var(--primary-color);
  color: white;
  box-shadow: var(--shadow-sm);
}
</style>