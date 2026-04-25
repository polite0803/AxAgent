import { chromium } from '@playwright/test';

const browser = await chromium.launch();
const page = await browser.newPage();

page.addInitScript(() => {
  if (window.__TAURI_INTERNALS__) return;

  const createId = () => `id_${Math.random().toString(36).slice(2)}`;
  const nowTs = () => Math.floor(Date.now() / 1000);
  const stubState = {
    settings: {
      last_selected_conversation_id: null,
      default_provider_id: 'mock-provider',
      default_model_id: 'mock-model',
      shortcut_toggle_gateway: 'CmdOrCtrl+Shift+G',
      global_shortcuts_enabled: true,
      shortcut_registration_logs_enabled: false,
      shortcut_trigger_toast_enabled: false,
      proxy_enabled: false,
      proxy_url: '',
      auto_backup: false,
      backup_interval_hours: 24,
      content_safety_enabled: true,
    },
    providers: [
      {
        id: 'mock-provider',
        name: 'Mock Provider',
        provider_type: 'openai',
        api_host: 'https://mock',
        api_path: null,
        enabled: true,
        models: [
          {
            provider_id: 'mock-provider',
            model_id: 'mock-model',
            name: 'Mock Model',
            model_type: 'Chat',
            capabilities: ['TextChat', 'Reasoning'],
            max_tokens: 4096,
            enabled: true,
            param_overrides: null,
          },
        ],
        keys: [],
        proxy_config: null,
        custom_headers: null,
        icon: null,
        builtin_id: null,
        sort_order: 0,
        created_at: nowTs(),
        updated_at: nowTs(),
      },
    ],
    conversations: [],
    conversation_categories: [],
    backups: [],
    search_providers: [],
    mcp_servers: [],
    knowledge_bases: [],
    memory_namespaces: [],
    eventListeners: new Map(),
    callbackStore: new Map(),
  };

  const pluginSafe = (cmd) => {
    if (cmd === 'plugin:event|listen') return 1;
    if (cmd === 'plugin:event|unlisten') return null;
    if (cmd === 'plugin:global-shortcut|register') return null;
    if (cmd === 'plugin:global-shortcut|unregister_all') return null;
    if (cmd === 'plugin:global-shortcut|is_registered') return false;
    if (cmd === 'plugin:updater|check') return null;
    if (cmd === 'plugin:window|get_all_windows') return ['main'];
    if (cmd === 'plugin:window|is_maximized') return false;
    if (cmd === 'plugin:window|is_visible') return true;
    if (cmd === 'plugin:window|is_minimized') return false;
    if (cmd === 'plugin:window|is_fullscreen') return false;
    if (cmd === 'plugin:window|set_title') return null;
    if (cmd === 'plugin:window|emit') return null;
    return null;
  };

  const handleCommand = async (cmd, args) => {
    switch (cmd) {
      case 'get_settings':
        return stubState.settings;
      case 'save_settings':
        stubState.settings = { ...stubState.settings, ...(args?.settings || {}) };
        return stubState.settings;
      case 'list_providers':
        return stubState.providers;
      case 'list_conversation_categories':
        return stubState.conversation_categories;
      case 'list_conversations':
        return stubState.conversations.filter((c) => !c.is_archived);
      case 'list_archived_conversations':
        return stubState.conversations.filter((c) => c.is_archived);
      case 'create_conversation': {
        const { title, modelId, providerId, systemPrompt } = args || {};
        const conv = {
          id: createId(),
          title: title || 'New Conversation',
          model_id: modelId,
          provider_id: providerId,
          system_prompt: systemPrompt || null,
          temperature: null,
          max_tokens: null,
          top_p: null,
          frequency_penalty: null,
          search_enabled: false,
          search_provider_id: null,
          thinking_budget: null,
          enabled_mcp_server_ids: [],
          enabled_knowledge_base_ids: [],
          enabled_memory_namespace_ids: [],
          context_compression: false,
          category_id: null,
          parent_conversation_id: null,
          mode: 'chat',
          message_count: 0,
          is_pinned: false,
          is_archived: false,
          created_at: nowTs(),
          updated_at: nowTs(),
          scenario: null,
          enabled_skill_ids: [],
        };
        stubState.conversations.push(conv);
        return conv;
      }
      case 'update_conversation': {
        const { id, input } = args || {};
        const conv = stubState.conversations.find((c) => c.id === id);
        if (conv) {
          Object.assign(conv, input, { updated_at: nowTs() });
          return conv;
        }
        throw new Error('Conversation not found');
      }
      case 'delete_conversation': {
        const { id } = args || {};
        stubState.conversations = stubState.conversations.filter((c) => c.id !== id);
        return null;
      }
      case 'toggle_archive_conversation': {
        const { id } = args || {};
        const conv = stubState.conversations.find((c) => c.id === id);
        if (conv) {
          conv.is_archived = !conv.is_archived;
          conv.updated_at = nowTs();
          return conv;
        }
        throw new Error('Conversation not found');
      }
      case 'get_backup_settings':
      case 'get_webdav_sync_status':
        return {};
      case 'list_backups':
      case 'list_search_providers':
      case 'list_mcp_servers':
      case 'list_knowledge_bases':
      case 'list_memory_namespaces':
        return [];
      default:
        if (cmd.startsWith('plugin:')) {
          return pluginSafe(cmd);
        }
        return null;
    }
  };

  const transformCallback = (callback, once = false) => {
    const id = `__CB_${createId()}`;
    stubState.callbackStore.set(id, { callback, once });
    return id;
  };

  const unregisterCallback = (id) => {
    stubState.callbackStore.delete(id);
  };

  Object.defineProperty(window, '__TAURI_INTERNALS__', {
    value: {
      metadata: {
        currentWindow: { label: 'main' },
        currentWebview: { label: 'main-webview' },
      },
      invoke: async (cmd, args, options) => {
        console.log('[TauriStub] invoke', cmd, args);
        return handleCommand(cmd, args, options);
      },
      transformCallback,
      unregisterCallback,
      convertFileSrc: (path) => path,
    },
    configurable: true,
  });

  Object.defineProperty(window, '__TAURI_EVENT_PLUGIN_INTERNALS__', {
    value: {
      unregisterListener: () => {},
    },
    configurable: true,
  });
});

page.on('console', (msg) => {
  console.log(`[console:${msg.type()}] ${msg.text()}`);
});
page.on('pageerror', (err) => {
  console.log(`[pageerror] ${err.message}`);
  console.log(err.stack);
});
page.on('requestfailed', (request) => {
  console.log(`[requestfailed] ${request.url()} ${request.failure()?.errorText}`);
});

try {
  console.log('Navigating to http://127.0.0.1:1420');
  await page.goto('http://127.0.0.1:1420', { waitUntil: 'networkidle' });
  await page.waitForTimeout(10000);
} catch (err) {
  console.error('Navigation failed', err);
} finally {
  await browser.close();
}
