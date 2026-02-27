<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "../composables/useToast";
import { useConfirm } from "../composables/useConfirm";

const { t } = useI18n();
const toast = useToast();
const { confirm: showConfirm } = useConfirm();

interface DbConfig {
  name: string;
  url: string;
}

const dbConnections = ref<DbConfig[]>([]);
const dbName = ref("");
const dbUrl = ref("");
const editingDbName = ref<string | null>(null);

async function fetchDbConnections() {
  try {
    const result = await invoke<DbConfig[]>("get_db_connections");
    dbConnections.value = result;
  } catch (error) {
    console.error("Failed to fetch DB connections:", error);
  }
}

async function handleTestDb() {
  if (!dbUrl.value) {
    toast.warning(t('enterConnectionUrl'));
    return;
  }
  try {
    const result = await invoke<string>("test_db_connection", { url: dbUrl.value });
    toast.success(t('testSuccess') + '\n' + result);
  } catch (error) {
    toast.error(t('connectionFailed', { error: String(error) }));
  }
}

async function handleTestDbList(url: string) {
  try {
    const result = await invoke<string>("test_db_connection", { url });
    toast.success(t('testSuccess') + '\n' + result);
  } catch (error) {
    toast.error(t('connectionFailed', { error: String(error) }));
  }
}

async function handleAddDb() {
  try {
    // If editing and name changed, remove old one first
    if (editingDbName.value && editingDbName.value !== dbName.value) {
       await invoke("remove_db_connection", { name: editingDbName.value });
    }

    await invoke("add_db_connection", {
      name: dbName.value,
      url: dbUrl.value
    });
    
    // Reset form
    dbName.value = "";
    dbUrl.value = "";
    editingDbName.value = null;
    fetchDbConnections();
    toast.success(t('connectAndSave') + ' Success');
  } catch (error) {
    console.error("Failed to add DB connection:", error);
    toast.error(t('failedToConnect', { error: String(error) }));
  }
}

function handleEditDb(conn: DbConfig) {
    dbName.value = conn.name;
    dbUrl.value = conn.url;
    editingDbName.value = conn.name;
    window.scrollTo({ top: 0, behavior: 'smooth' });
}

function cancelEditDb() {
    dbName.value = "";
    dbUrl.value = "";
    editingDbName.value = null;
}

async function handleRemoveDb(name: string) {
    if (await showConfirm(t('confirmRemoveDb', { name }))) {
        try {
            await invoke("remove_db_connection", { name });
            if (editingDbName.value === name) {
                cancelEditDb();
            }
            fetchDbConnections();
            toast.success(t('disconnect') + ' Success');
        } catch (error) {
            console.error("Failed to remove DB connection:", error);
            toast.error(String(error));
        }
    }
}

onMounted(() => {
  fetchDbConnections();
});
</script>

<template>
  <div>
    <div class="section">
        <h2>{{ editingDbName ? t('editDatabaseConnection') : t('addDatabaseConnection') }}</h2>
        <form @submit.prevent="handleAddDb" class="mock-form">
            <div class="form-group">
                <label>{{ t('connectionName') }}:</label>
                <input v-model="dbName" placeholder="my-db" required />
            </div>
            <div class="form-group">
                <label>{{ t('connectionUrl') }}:</label>
                <input v-model="dbUrl" placeholder="sqlite://db.sqlite" required />
                <small>{{ t('connectionExamples') }} sqlite://db.sqlite, mysql://user:pass@localhost/db, postgres://user:pass@localhost/db</small>
            </div>
            <div class="button-group">
              <button type="submit">{{ editingDbName ? t('updateDatabaseConnection') : t('connectAndSave') }}</button>
              <button v-if="editingDbName" type="button" @click="cancelEditDb" class="cancel-btn">{{ t('cancel') }}</button>
              <button type="button" @click="handleTestDb" class="btn-warning">{{ t('testConnection') }}</button>
            </div>
        </form>
    </div>
    
    <div class="section">
        <h2>{{ t('activeConnections') }}</h2>
        <ul class="mock-list">
            <li v-for="conn in dbConnections" :key="conn.name" class="mock-item">
                <div class="mock-summary" style="cursor: default;">
                    <div class="mock-header">
                        <span class="path">{{ conn.name }}</span>
                        <span class="url">{{ conn.url }}</span>
                    </div>
                    <div class="actions">
                        <button @click="handleTestDbList(conn.url)" class="btn-warning-sm">{{ t('testConnection') }}</button>
                        <button @click="handleEditDb(conn)" class="edit-btn">{{ t('edit') }}</button>
                        <button @click="handleRemoveDb(conn.name)" class="delete-btn">{{ t('disconnect') }}</button>
                    </div>
                </div>
            </li>
        </ul>
    </div>
  </div>
</template>