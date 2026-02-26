<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { VueMonacoEditor } from "@guolao/vue-monaco-editor";
import ToastContainer from "./components/ToastContainer.vue";
import { useToast } from "./composables/useToast";
import "./App.css";

const { t, locale } = useI18n();
const toast = useToast();

interface MockApi {
  id: string;
  path: string;
  method: string;
  response_body: string;
  status_code: number;
  response_type: string;
}

interface DbConfig {
  name: string;
  url: string;
}

interface ServerConfig {
  port: number;
  host: string;
  running: boolean;
}

const mocks = ref<MockApi[]>([]);
const dbConnections = ref<DbConfig[]>([]);
const serverConfig = ref<ServerConfig>({ port: 3000, host: '127.0.0.1', running: true });
const serverStatus = ref(false); // Track actual running state
const activeTab = ref<'mocks' | 'database' | 'help' | 'settings'>('mocks');

// DB Form
const dbName = ref("");
const dbUrl = ref("");
const editingDbName = ref<string | null>(null);

const path = ref("");
const method = ref("GET");
const responseBody = ref("");
const statusCode = ref(200);
const responseType = ref("json");
const editingId = ref<string | null>(null);
const expandedMocks = ref<Set<string>>(new Set());

const MONACO_EDITOR_OPTIONS = {
  automaticLayout: true,
  formatOnType: true,
  formatOnPaste: true,
  minimap: { enabled: false },
  scrollBeyondLastLine: false,
};

const JS_TEMPLATE = `// Function to handle the request
// Params:
//   request.headers: Object - Request headers
//   request.body: String - Request body content
//   request.method: String - HTTP method
//   request.path: String - Request path
//
// Methods:
//   response.setStatusCode(code: number) - Set response status code
//   db.query(connName: string, sql: string) - Execute SQL query
//   db.execute(connName: string, sql: string) - Execute SQL command
//   console.log(...args) - Log to application console
//
// Return: 
//   - String: Response body
//   - Object: Will be stringified as JSON

// Example: Echo back the body
if (request.method === "POST") {
    response.setStatusCode(201);
    return {
        message: "Created successfully",
        data: request.body
    };
}

return {
    message: "Hello from JS",
    received: request.body,
    method: request.method
};
`;

function handleMount(_editor: any, monaco: any) {
  // Inject TypeScript definitions
  monaco.languages.typescript.javascriptDefaults.addExtraLib(`
    declare interface Request {
      headers: Record<string, string>;
      body: string;
      method: string;
      path: string;
    }

    declare interface Response {
      setStatusCode(code: number): void;
    }

    declare interface DB {
      query(connectionName: string, sql: string): string; // Returns JSON string
      execute(connectionName: string, sql: string): number; // Returns rows affected
    }

    declare interface Console {
      log(...args: any[]): void;
    }

    declare const request: Request;
    declare const response: Response;
    declare const db: DB;
    declare const console: Console;
  `, 'lib.d.ts');

  monaco.languages.registerCompletionItemProvider('html', {
    triggerCharacters: ['!'],
    provideCompletionItems: (model: any, position: any) => {
      // Get text until current position
      const textUntilPosition = model.getValueInRange({
        startLineNumber: position.lineNumber,
        startColumn: 1,
        endLineNumber: position.lineNumber,
        endColumn: position.column
      });
      
      // Check if line content is just "!"
      const match = textUntilPosition.match(/^!$/);
      if (!match) {
        return { suggestions: [] };
      }

      const range = {
        startLineNumber: position.lineNumber,
        startColumn: position.column - 1,
        endLineNumber: position.lineNumber,
        endColumn: position.column,
      };

      return {
        suggestions: [
          {
            label: '!',
            kind: monaco.languages.CompletionItemKind.Snippet,
            documentation: 'Basic HTML5 Template',
            insertText: `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>\${1:Document}</title>
</head>
<body>
    \${2}
</body>
</html>`,
            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
            range: range
          }
        ]
      };
    }
  });
}

async function fetchMocks() {
  try {
    const result = await invoke<MockApi[]>("get_mock_apis");
    mocks.value = result;
  } catch (error) {
    console.error("Failed to fetch mocks:", error);
  }
}

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
    if (confirm(t('confirmRemoveDb', { name }))) {
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

async function handleSubmit() {
  try {
    if (editingId.value) {
      await invoke("update_mock_api", {
        id: editingId.value,
        path: path.value,
        method: method.value,
        responseBody: responseBody.value,
        statusCode: statusCode.value,
        responseType: responseType.value,
      });
      editingId.value = null;
    } else {
      await invoke("add_mock_api", {
        path: path.value,
        method: method.value,
        responseBody: responseBody.value,
        statusCode: statusCode.value,
        responseType: responseType.value,
      });
    }
    // Reset form
    path.value = "";
    responseBody.value = "";
    statusCode.value = 200;
    method.value = "GET";
    responseType.value = "json";
    fetchMocks();
    toast.success(editingId.value ? t('updateMock') + ' Success' : t('addMock') + ' Success');
  } catch (error) {
    console.error("Failed to save mock:", error);
    toast.error(String(error));
  }
}

function handleEdit(mock: MockApi) {
  path.value = mock.path;
  method.value = mock.method;
  responseBody.value = mock.response_body;
  statusCode.value = mock.status_code;
  responseType.value = mock.response_type || "json";
  editingId.value = mock.id;
  // Scroll to form
  window.scrollTo({ top: 0, behavior: 'smooth' });
}

function cancelEdit() {
  editingId.value = null;
  path.value = "";
  responseBody.value = "";
  statusCode.value = 200;
  method.value = "GET";
  responseType.value = "json";
}

async function handleRemove(id: string) {
  if (confirm(t('confirmDeleteMock'))) {
    try {
      await invoke("remove_mock_api", { id });
      if (editingId.value === id) {
        cancelEdit();
      }
      fetchMocks();
      toast.success(t('remove') + ' Success');
    } catch (error) {
      console.error("Failed to remove mock:", error);
      toast.error(String(error));
    }
  }
}

function toggleExpand(id: string) {
  const newSet = new Set(expandedMocks.value);
  if (newSet.has(id)) {
    newSet.delete(id);
  } else {
    newSet.add(id);
  }
  expandedMocks.value = newSet;
}

function getEditorLanguage(type: string) {
  if (type === 'raw') return 'plaintext';
  if (type === 'js') return 'javascript';
  return type;
}

function handleTypeChange() {
  if (responseType.value === 'js' && !responseBody.value) {
    responseBody.value = JS_TEMPLATE;
  } else if (responseType.value === 'json' && (responseBody.value === JS_TEMPLATE || responseBody.value.startsWith('http'))) {
    responseBody.value = '';
  } else if (responseType.value === 'proxy') {
    responseBody.value = '';
  }
}

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
    if (confirm(t('restartServerConfirm'))) {
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

onMounted(() => {
  fetchMocks();
  fetchDbConnections();
  fetchServerConfig();
});
</script>

<template>
  <ToastContainer />
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
                <button :class="{ active: activeTab === 'settings' }" @click="activeTab = 'settings'">{{ t('settings') }}</button>
                <button :class="{ active: activeTab === 'help' }" @click="activeTab = 'help'">{{ t('help') }}</button>
            </div>
        </div>
    </div>
    
    <div v-if="activeTab === 'mocks'">
        <div class="section">
            <h2>{{ editingId ? t('editMockApi') : t('addMockApi') }}</h2>
            <form @submit.prevent="handleSubmit" class="mock-form">
        <div class="form-group">
          <label>{{ t('path') }}:</label>
          <input v-model="path" placeholder="/api/users" required />
        </div>
        <div class="form-group">
          <label>{{ t('method') }}:</label>
          <select v-model="method">
            <option value="GET">GET</option>
            <option value="POST">POST</option>
            <option value="PUT">PUT</option>
            <option value="DELETE">DELETE</option>
            <option value="PATCH">PATCH</option>
            <option value="ANY">ANY</option>
          </select>
        </div>
        <div class="form-group">
          <label>{{ t('responseType') }}:</label>
          <select v-model="responseType" @change="handleTypeChange">
            <option value="json">JSON</option>
            <option value="html">HTML</option>
            <option value="raw">Raw Text</option>
            <option value="js">Dynamic JavaScript</option>
            <option value="proxy">Proxy</option>
          </select>
        </div>
        <div class="form-group" v-if="responseType !== 'proxy'">
          <label>{{ t('statusCode') }}:</label>
          <input type="number" v-model.number="statusCode" required />
        </div>
        <div class="form-group editor-container">
          <label>{{ responseType === 'proxy' ? t('targetUrl') : `${t('responseBody')} (${responseType.toUpperCase()})` }}:</label>
          <div v-if="responseType === 'proxy'" class="proxy-input">
            <input v-model="responseBody" placeholder="http://localhost:8080/api" required />
          </div>
          <div v-else class="editor-wrapper">
            <VueMonacoEditor
              v-model:value="responseBody"
              theme="vs-dark"
              :language="getEditorLanguage(responseType)"
              height="300px"
              :options="MONACO_EDITOR_OPTIONS"
              @mount="handleMount"
            />
          </div>
        </div>
        <div class="button-group">
          <button type="submit">{{ editingId ? t('updateMock') : t('addMock') }}</button>
          <button v-if="editingId" type="button" @click="cancelEdit" class="cancel-btn">{{ t('cancel') }}</button>
        </div>
      </form>
    </div>

    <div class="section">
      <div class="server-status-bar">
          <div class="status-indicator">
              <span class="status-dot" :class="{ 'running': serverStatus }"></span>
              <span>{{ serverStatus ? t('serverRunning', { url: `http://${serverConfig.host}:${serverConfig.port}` }) : t('serverStopped') }}</span>
          </div>
          <button @click="toggleServer" :class="serverStatus ? 'stop-btn' : 'start-btn'">
              {{ serverStatus ? t('stopServer') : t('startServer') }}
          </button>
      </div>
      <h2>{{ t('activeMocks') }}</h2>
      <ul class="mock-list">
        <li v-for="mock in mocks" :key="mock.id" class="mock-item">
          <div class="mock-summary" @click="toggleExpand(mock.id)">
            <div class="mock-header">
              <span :class="['method', mock.method]">{{ mock.method }}</span>
              <span class="path">{{ mock.path }}</span>
              <div class="badges">
                <span v-if="mock.response_type !== 'proxy'" class="status-badge" :class="mock.status_code >= 400 ? 'error' : 'success'">
                  {{ mock.status_code }}
                </span>
                <span class="type-badge">{{ mock.response_type || 'json' }}</span>
              </div>
            </div>
            <span class="expand-icon">{{ expandedMocks.has(mock.id) ? '▼' : '▶' }}</span>
          </div>
          
          <div v-if="expandedMocks.has(mock.id)" class="mock-body">
            <div class="body-content">
              <pre>{{ mock.response_body }}</pre>
            </div>
            <div class="actions">
              <button @click.stop="handleEdit(mock)" class="edit-btn">{{ t('edit') }}</button>
              <button @click.stop="handleRemove(mock.id)" class="delete-btn">{{ t('remove') }}</button>
            </div>
          </div>
        </li>
      </ul>
    </div>
  </div>
    <div v-if="activeTab === 'database'">
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
                    <small>{{ t('connectionExamples') }}</small>
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
    <div v-if="activeTab === 'settings'">
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
    </div>
    <div v-if="activeTab === 'help'">
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
    </div>
  </div>
</template>

<style scoped>
.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 2rem;
  padding-bottom: 1rem;
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

/* App.css is imported globally in script setup, or we can use scoped here */
.editor-container {
  display: flex;
  flex-direction: column;
}

.editor-wrapper {
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  overflow: hidden;
  margin-top: 0.5rem;
}
</style>
