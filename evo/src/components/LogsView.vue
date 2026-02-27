<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { useI18n } from "vue-i18n";
import { useLogs } from "../composables/useLogs";

const { t } = useI18n();
const { requestLogs, fetchLogs, clearLogs } = useLogs();
const expandedLogs = ref<Set<string>>(new Set());
const searchQuery = ref("");
const statusFilter = ref<"all" | "success" | "error">("all");

const filteredLogs = computed(() => {
  return requestLogs.value.filter(log => {
    // Search filter
    const searchLower = searchQuery.value.toLowerCase();
    const matchesSearch = 
      log.path.toLowerCase().includes(searchLower) || 
      log.method.toLowerCase().includes(searchLower) ||
      String(log.status_code).includes(searchLower);

    // Status filter
    let matchesStatus = true;
    if (statusFilter.value === "success") {
      matchesStatus = log.status_code >= 200 && log.status_code < 400;
    } else if (statusFilter.value === "error") {
      matchesStatus = log.status_code >= 400;
    }

    return matchesSearch && matchesStatus;
  });
});

function formatTimestamp(ts: number) {
    return new Date(ts).toLocaleString();
}

function getStatusClass(code: number) {
    if (code >= 200 && code < 300) return 'status-success';
    if (code >= 300 && code < 400) return 'status-redirect';
    if (code >= 400 && code < 500) return 'status-client-error';
    if (code >= 500) return 'status-server-error';
    return 'status-default';
}

function getMethodClass(method: string) {
    return `method-${method.toUpperCase()}`;
}

function toggleExpandLog(id: string) {
  const newSet = new Set(expandedLogs.value);
  if (newSet.has(id)) {
    newSet.delete(id);
  } else {
    newSet.add(id);
  }
  expandedLogs.value = newSet;
}

onMounted(() => {
    fetchLogs();
    // Listening is handled globally in App.vue to prevent duplicates
});
</script>

<template>
  <div class="logs-container">
      <div class="header-actions">
          <div class="search-bar">
              <input 
                v-model="searchQuery" 
                type="text" 
                :placeholder="t('searchLogs')" 
                class="search-input"
              />
              <select v-model="statusFilter" class="filter-select">
                <option value="all">{{ t('all') }}</option>
                <option value="success">{{ t('success') }}</option>
                <option value="error">{{ t('error') }}</option>
              </select>
          </div>
          <button @click="clearLogs" class="delete-btn">
            <span class="icon">🗑️</span> {{ t('clearLogs') }}
          </button>
      </div>

      <div class="logs-list-wrapper">
        <ul class="logs-list">
            <li v-for="log in filteredLogs" :key="log.id" class="log-item" :class="{ expanded: expandedLogs.has(log.id) }">
                <div class="log-summary" @click="toggleExpandLog(log.id)">
                    <div class="log-main-info">
                        <span :class="['method-badge', getMethodClass(log.method)]">{{ log.method }}</span>
                        <span class="path-text" :title="log.path">{{ log.path }}</span>
                    </div>
                    
                    <div class="log-meta-info">
                        <span :class="['status-badge', getStatusClass(log.status_code)]">
                            {{ log.status_code }}
                        </span>
                        <span class="duration-badge">{{ log.duration_ms }}ms</span>
                        <span class="time-text">{{ formatTimestamp(log.timestamp) }}</span>
                        <span class="expand-arrow">{{ expandedLogs.has(log.id) ? '▼' : '▶' }}</span>
                    </div>
                </div>
                
                <div v-show="expandedLogs.has(log.id)" class="log-details-panel">
                    <div class="log-detail-section">
                        <h4>Request Body</h4>
                        <div class="code-block">
                            <pre>{{ log.request_body || '(Empty)' }}</pre>
                        </div>
                    </div>
                    <div class="log-detail-section">
                        <h4>Response Body</h4>
                        <div class="code-block">
                            <pre>{{ log.response_body || '(Empty)' }}</pre>
                        </div>
                    </div>
                </div>
            </li>
        </ul>
        <div v-if="filteredLogs.length === 0" class="empty-state">
            <div class="empty-icon">📝</div>
            <p>{{ t('noLogs') }}</p>
        </div>
      </div>
  </div>
</template>

<style scoped>
.logs-container {
    display: flex;
    flex-direction: column;
    height: 100%; /* Changed from flex: 1 */
    min-height: 0; 
    padding: 0;
    overflow: hidden;
    background-color: var(--card-bg);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
}

.header-actions {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    border-bottom: 1px solid var(--border-color);
    background-color: #fff;
}

.search-bar {
    display: flex;
    gap: 0.5rem;
    flex: 1;
    max-width: 400px;
}

.search-input {
    flex: 1;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    font-size: 0.875rem;
}

.filter-select {
    padding: 0.5rem 2rem 0.5rem 0.75rem;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    font-size: 0.875rem;
    background-color: #fff;
    cursor: pointer;
}

.logs-list-wrapper {
    flex: 1;
    overflow-y: auto;
    background-color: var(--bg-color);
    padding: 1rem;
    min-height: 0; /* Add this to ensure proper scrolling */
}

.logs-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}

.log-item {
    background-color: #fff;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    transition: box-shadow 0.2s, border-color 0.2s;
}

.log-item:hover {
    box-shadow: var(--shadow-sm);
    border-color: var(--primary-color);
}

.log-summary {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 1rem;
    cursor: pointer;
    user-select: none;
}

.log-main-info {
    display: flex;
    align-items: center;
    gap: 1rem;
    flex: 1;
    min-width: 0; /* Enable text truncation */
}

.method-badge {
    font-size: 0.75rem;
    font-weight: 700;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    min-width: 3.5rem;
    text-align: center;
    text-transform: uppercase;
}

/* Method Colors */
.method-GET { background-color: #e3f2fd; color: #1565c0; }
.method-POST { background-color: #e8f5e9; color: #2e7d32; }
.method-PUT { background-color: #fff3e0; color: #ef6c00; }
.method-DELETE { background-color: #ffebee; color: #c62828; }
.method-PATCH { background-color: #f3e5f5; color: #6a1b9a; }
.method-OPTIONS { background-color: #f5f5f5; color: #616161; }

.path-text {
    font-family: 'Menlo', 'Monaco', 'Courier New', monospace;
    font-size: 0.9rem;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.log-meta-info {
    display: flex;
    align-items: center;
    gap: 1rem;
    flex-shrink: 0;
}

.status-badge {
    font-size: 0.75rem;
    font-weight: 600;
    padding: 0.125rem 0.5rem;
    border-radius: 12px;
}

/* Status Colors */
.status-success { background-color: #d1fae5; color: #065f46; } /* 2xx */
.status-redirect { background-color: #e0f2fe; color: #075985; } /* 3xx */
.status-client-error { background-color: #fee2e2; color: #991b1b; } /* 4xx */
.status-server-error { background-color: #fef2f2; color: #991b1b; border: 1px solid #fecaca; } /* 5xx */

.duration-badge {
    font-size: 0.75rem;
    color: var(--text-secondary);
    background-color: var(--bg-color);
    padding: 0.125rem 0.5rem;
    border-radius: 4px;
}

.time-text {
    font-size: 0.75rem;
    color: var(--text-secondary);
}

.expand-arrow {
    font-size: 0.75rem;
    color: var(--text-secondary);
    width: 1rem;
    text-align: center;
}

.log-details-panel {
    border-top: 1px solid var(--border-color);
    background-color: #f8fafc;
    padding: 1rem;
    animation: slideDown 0.2s ease-out;
}

@keyframes slideDown {
    from { opacity: 0; transform: translateY(-10px); }
    to { opacity: 1; transform: translateY(0); }
}

.log-detail-section {
    margin-bottom: 1rem;
}

.log-detail-section:last-child {
    margin-bottom: 0;
}

.log-detail-section h4 {
    font-size: 0.8rem;
    text-transform: uppercase;
    color: var(--text-secondary);
    margin: 0 0 0.5rem 0;
    font-weight: 600;
}

.code-block {
    background-color: #fff;
    border: 1px solid var(--border-color);
    border-radius: 4px;
    padding: 0.75rem;
    overflow-x: auto;
}

.code-block pre {
    margin: 0;
    font-family: 'Menlo', 'Monaco', 'Courier New', monospace;
    font-size: 0.85rem;
    color: #334155;
    white-space: pre-wrap;
    word-break: break-all;
}

.empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: calc(100% - 69px);
    color: var(--text-secondary);
    padding: 2rem;
}

.empty-icon {
    font-size: 3rem;
    margin-bottom: 1rem;
    opacity: 0.5;
}
</style>