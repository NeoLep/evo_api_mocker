<script setup lang="ts">
import { useI18n } from "vue-i18n";

const { t } = useI18n();
</script>

<template>
  <div class="section">
      <h2>{{ t('help') }}</h2>
      <div class="help-content">
          <h3>{{ t('helpMocksTitle') }}</h3>
          <p>{{ t('helpMocksDesc') }}</p>
          <ul>
              <li><strong>{{ t('path') }}:</strong> {{ t('helpPathDesc') }}</li>
              <li><strong>{{ t('method') }}:</strong> {{ t('helpMethodDesc') }}</li>
              <li><strong>{{ t('responseType') }}:</strong> {{ t('helpResponseTypeDesc') }}</li>
              <li><strong>{{ t('jsTemplate') }}:</strong> {{ t('helpJsTemplateDesc') }}</li>
          </ul>

          <h3>{{ t('helpDatabaseTitle') }}</h3>
          <p>{{ t('helpDatabaseDesc') }}</p>
          <ul>
              <li><strong>{{ t('connectionUrl') }}:</strong> {{ t('helpConnectionUrlDesc') }}</li>
          </ul>
      </div>
  </div>
</template>