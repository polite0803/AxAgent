# Configurar proveedores

AxAgent se conecta simultáneamente a cualquier número de proveedores de IA. Cada proveedor tiene sus propias claves API, lista de modelos y parámetros predeterminados.

## Proveedores soportados

| Proveedor | Modelos ejemplo |
|----------|----------------|
| **OpenAI** | GPT-4o, GPT-4, o3, o4-mini |
| **Anthropic** | Claude 4 Sonnet, Claude 4 Opus, Claude 3.5 Sonnet |
| **Google** | Gemini 2.5 Pro, Gemini 2.5 Flash, Gemini 2.0 |
| **DeepSeek** | DeepSeek V3, DeepSeek R1 |
| **Alibaba Cloud** | Serie Qwen |
| **Zhipu AI** | Serie GLM |
| **xAI** | Serie Grok |
| **API compatible OpenAI** | Ollama, vLLM, LiteLLM, relés de terceros, etc. |

---

## Añadir un proveedor

1. Ve a **Configuración → Proveedores**.
2. Haz clic en el botón **+** en la parte inferior izquierda.
3. Completa los detalles del proveedor:

| Campo | Descripción |
|-------|-------------|
| **Nombre** | Nombre para mostrar en la barra lateral (ej. *OpenAI*) |
| **Tipo** | Tipo de proveedor — determina la URL base predeterminada |
| **Ícono** | Ícono opcional para identificación visual |
| **Clave API** | La clave secreta del panel de tu proveedor |
| **URL base** | Endpoint API (prellenado para tipos integrados) |
| **Ruta API** | Ruta de solicitud — predeterminado `/v1/chat/completions` |

---

## Rotación de claves múltiples

AxAgent soporta múltiples claves API por proveedor. Haz clic en **Añadir clave** en el panel de detalles del proveedor.

---

## Gestión de modelos

Haz clic en **Obtener modelos** para obtener la lista completa de modelos disponibles. También puedes añadir IDs de modelos manualmente.

Cada modelo puede tener sus propias anulaciones de parámetros predeterminados: temperatura, tokens máximos, Top P, penalización de frecuencia, penalización de presencia.

---

## Endpoints personalizados y locales

### Ollama (modelos locales)

1. Instala e inicia [Ollama](https://ollama.com/).
2. En AxAgent, crea un nuevo proveedor con tipo **OpenAI**.
3. Establece la **URL base** a `http://localhost:11434`.
4. Haz clic en **Obtener modelos** para descubrir los modelos descargados localmente.

---

## Próximos pasos

- [Servidores MCP](./mcp) — conectar herramientas externas para ampliar las capacidades de IA
- [Pasarela API](./gateway) — exponer tus proveedores como servidor API local
