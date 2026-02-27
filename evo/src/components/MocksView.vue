<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { VueMonacoEditor } from "@guolao/vue-monaco-editor";
import { useToast } from "../composables/useToast";
import { useConfirm } from "../composables/useConfirm";
import { useServer } from "../composables/useServer";

const { t } = useI18n();
const toast = useToast();
const { confirm: showConfirm } = useConfirm();
const { serverStatus, serverConfig, toggleServer } = useServer();

interface MockApi {
  id: string;
  path: string;
  method: string;
  response_body: string;
  status_code: number;
  response_type: string;
}

const mocks = ref<MockApi[]>([]);
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
  if (await showConfirm(t('confirmDeleteMock'))) {
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

onMounted(() => {
  fetchMocks();
});
</script>

<template>
  <div>
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
</template>

<style scoped>
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