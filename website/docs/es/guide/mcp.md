# Servidores MCP

## ¿Qué es MCP?

El [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) es un estándar abierto que permite a los modelos de IA interactuar con herramientas y fuentes de datos externas. AxAgent actúa como cliente MCP — añades servidores MCP y la IA puede llamar a las herramientas que exponen durante una conversación.

---

## Protocolos de transporte

AxAgent soporta tres protocolos de transporte para comunicarse con servidores MCP:

| Protocolo | Conexión | Caso de uso | Configuración |
|-----------|---------|------------|---------------|
| **Stdio** | Proceso local | Herramientas instaladas en tu máquina, lanzadas via `npx`, `uvx`, `python`, etc. | `command` + `args` + `env` opcional |
| **SSE** | Servidor remoto | Endpoint Server-Sent Events alojado en una máquina remota | `url` |
| **StreamableHTTP** | Servidor remoto | Endpoint HTTP streaming, alternativa más nueva a SSE | `url` |

---

## Añadir servidores MCP

### Creación por formulario

1. Ve a **Configuración → Servidores MCP**.
2. Haz clic en **Añadir servidor MCP**.
3. Introduce un nombre y selecciona el protocolo de transporte.
4. Completa los campos para tu protocolo elegido.
5. Haz clic en **Guardar**.

### Importación JSON

Haz clic en **Importar JSON** y pega un objeto de configuración:

```json
{
  "mcpServers": {
    "fetch": {
      "command": "uvx",
      "args": ["mcp-server-fetch"]
    }
  }
}
```

---

## Herramientas integradas

| Herramienta | Descripción |
|------------|-------------|
| **@axagent/fetch** | Obtener páginas web y recursos HTTP |
| **@axagent/search-file** | Buscar archivos en tu sistema de archivos local |

---

## Próximos pasos

- [Pasarela API](./gateway) — exponer tus proveedores como servidor API local
- [Inicio rápido](./getting-started) — volver a la guía de inicio rápido
