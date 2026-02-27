<script setup lang="ts">
import { onMounted } from "vue";
import { useI18n } from "vue-i18n";
import { useServer } from "../composables/useServer";

const { t } = useI18n();
const { serverConfig, fetchServerConfig, handleSaveSettings } = useServer();

onMounted(() => {
    fetchServerConfig();
});
</script>

<template>
  <div class="section">
      <h2>{{ t('serverSettings') }}</h2>
      <form @submit.prevent="handleSaveSettings" class="mock-form">
          <div class="form-group">
              <label>{{ t('serverPort') }}:</label>
              <input type="number" v-model.number="serverConfig.port" required />
          </div>
          <div class="form-group">
              <label>{{ t('serverHost') }}:</label>
              <select v-model="serverConfig.host">
                  <option value="127.0.0.1">Localhost (127.0.0.1)</option>
                  <option value="0.0.0.0">All Interfaces (0.0.0.0)</option>
              </select>
          </div>
          <div class="form-group">
              <label class="checkbox-label">
                  <input type="checkbox" v-model="serverConfig.running" />
                  {{ t('serverEnabled') }}
              </label>
          </div>
          <div class="button-group">
              <button type="submit">{{ t('saveAndRestart') }}</button>
          </div>
      </form>
  </div>
</template>