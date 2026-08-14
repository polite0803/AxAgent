# AxAgent

[![Release](https://img.shields.io/github/v/release/polite0803/AxAgent?style=flat-square)](https://github.com/polite0803/AxAgent/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/polite0803/AxAgent/release.yml?style=flat-square)](https://github.com/polite0803/AxAgent/actions)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux%20%7C%20Android%20%7C%20iOS-blue?style=flat-square)
![License](https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square)

<p align="center">
  <a href="./media/poster-axagent.svg">
    <img src="./media/poster-axagent.svg" alt="AxAgent Poster" width="80%" />
  </a>
</p>

**AxAgent** es un cliente de escritorio de IA multiplataforma basado en Tauri 2 (Windows / macOS / Linux / Android / iOS), concebido como un espacio de trabajo impulsado por IA para el desarrollo diario, la investigación, la gestión del conocimiento y la automatización. Incluye un motor de agente ReAct, enrutamiento cognitivo (enrutamiento jerárquico de tres niveles + enrutamiento aumentado por recuperación RAR), orquestación visual de flujos de trabajo, base de conocimiento RAG local, extensión mediante el protocolo MCP, pasarela unificada multimodelo, automatización de navegador y control de computadora, llevando a la IA de la "conversación" a la "ejecución".

> **Idiomas**: [简体中文](./README.md) | [English](./README-EN.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

---

## Posicionamiento del proyecto

AxAgent resuelve tres problemas fundamentales:

1. **Integración unificada y enrutamiento inteligente de múltiples modelos** — Usa OpenAI, Anthropic Claude, Google Gemini, DeepSeek, Qwen, GLM, Kimi, Wenxin, modelos locales de Ollama y cualquier API compatible con OpenAI desde una única interfaz, con rotación automática de cuotas para múltiples claves, enrutamiento inteligente según el tipo de tarea y comparación en streaming
2. **El bucle cerrado de la IA, de la conversación a la ejecución** — 163+ herramientas integradas + flujos de trabajo visuales + extensiones MCP + control de navegador/computadora, la IA puede manipular archivos, ejecutar código, gestionar Git y programar tareas
3. **Soberanía de datos con prioridad local** — Los registros de conversación, la base de conocimiento, la memoria y la configuración se almacenan en una base de datos SQLite local; las claves de API se cifran con AES-256-GCM; las funciones principales funcionan sin servicios en la nube de terceros

---

## Capacidades principales

### Sistema de enrutamiento cognitivo (Cognitive Router)

AxAgent usa `cognitive_query` como punto de entrada unificado para todas las conversaciones y mapea la intención del usuario a capacidades concretas mediante **enrutamiento jerárquico de tres niveles**:

- **Enrutador de dominio L1** (`domain_router`): reglas + respaldo de LLM, identifica 9 dominios de negocio (análisis de datos / creación de contenido / comunicación / operaciones / medios de IA / finanzas / automatización / uso general, etc.)
- **Enrutador de clúster L2** (`cluster_router`): localiza clústeres de capacidades dentro del dominio (27 clústeres que cubren 8 dominios de negocio)
- **Enrutador de capacidades L3**: **enrutamiento aumentado por recuperación (RAR)** — recupera los Top-K flujos de trabajo similares desde la base de vectores de capacidades e inyecta en el Prompt, combinado con la búsqueda de rutas en el grafo DAG del flujo de trabajo, para generar la ruta de salida (p. ej. `/finance/stock_analysis/tech`) y el modo de ejecución
- **Modo de ejecución**: `Ask / Plan / Act / Workflow / Direct / Delegate / ParameterExtract / Clarify`, seleccionado automáticamente según la confianza
- **Sistema de capacidades**: registro unificado (`CapabilityRegistry`) + índice vectorial (`CapabilityIndexer`) + recuperación híbrida (`CapabilityRetriever`, vectores + BM25 + coincidencia estricta de etiquetas + exclusión de muestras negativas)
- **Aislamiento de capacidades del sistema**: el orquestador cognitivo y los flujos de trabajo de negocio están físicamente aislados; las capacidades del sistema llevan la marca de visibilidad `SYSTEM_ONLY`; la capa de enrutamiento incorpora un cortocircuito de auto-referencia para prevenir la paradoja de la auto-referencia
- **Enrutamiento de tres niveles implementado con DAG de flujo de trabajo**: 4 plantillas de flujo de trabajo de enrutamiento predefinidas (orquestación principal de ~20 nodos + subenrutadores L1/L2/L3), ejecutadas por el motor `rt-workflow`

### Motor multimodelo

- **13 adaptadores de proveedores**: OpenAI (Chat Completions + Responses + Realtime), Anthropic Claude, Google Gemini, DeepSeek, Qwen, GLM, Kimi, Wenxin Yiyan, Ollama, Llama.cpp (modelos locales GGUF), OpenClaw, Hermes, así como todas las APIs compatibles con OpenAI
- **Rotación de múltiples claves**: varias claves de API por proveedor, rotación automática según la cuota, conmutación automática cuando una clave individual alcanza el límite de velocidad
- **Enrutamiento inteligente**: selección automática del mejor modelo según el tipo de tarea (revisión de código / resumen / traducción / uso general), con soporte de reglas personalizadas
- **Monitoreo de salud de proveedores**: seguimiento en tiempo real de la tasa de éxito, latencia y estado de disponibilidad, con degradación automática por niveles
- **Generación de imágenes con IA**: ajustes preestablecidos de múltiples tamaños para DALL-E 3 y Flux
- **Voz en tiempo real**: conversación de voz por WebSocket basada en la API Realtime de OpenAI, con interrupción y transcripción en streaming

### Sistema de agente (motor ReAct)

- **Planificador jerárquico** (`hierarchical_planner`): descompone tareas complejas en un plan estructurado Phase → Task, compilado para su ejecución topológica como DAG
- **Investigación profunda** (`deep_research`): orquestación de búsqueda multi-fuente, que incluye plan de búsqueda, ejecución de búsqueda, síntesis de contenido y seguimiento de citas
- **Verificador de hechos** (`fact_checker`): verificación de hechos impulsada por IA, con clasificador de fuentes y evaluación de credibilidad
- **Árbol de pensamiento** (`tree_of_thoughts`): exploración de razonamiento multi-ruta, con evaluación de ramas y retroceso
- **Reflexor** (`reflector`): autoevaluación y sugerencias de mejora tras la ejecución de la tarea
- **Auto-verificación** (`self_verifier`): validación automática de los resultados del razonamiento, incluida la detección de bucles
- **Recuperación de errores** (`error_recovery_engine`): clasificación del tipo de error → selección de estrategia de recuperación → reintento automático o ajuste del plan, con retroceso exponencial
- **Pruebas A/B** (`ab_testing`): evaluación comparativa de diferentes estrategias de razonamiento
- **Sistema de evaluación** (`evaluator`): marco de pruebas de referencia integrado
- **Ajuste fino LoRA** (`fine_tune`): canalización de entrenamiento integrada, con gestión de adaptadores LoRA
- **Optimizador RL** (`rl_optimizer`): aprendizaje por refuerzo de políticas basado en retroalimentación de la experiencia

**Colaboración multi-agente**:

- Arquitectura de coordinación maestro-esclavo, con ejecución paralela de subagentes y programación consciente de dependencias
- Pizarra compartida para el intercambio de información entre agentes
- Modo de debate adversarial (rondas Pro/Con con puntuación de la fuerza de los argumentos)
- Modo de clúster Swarm, clúster de agentes multiproceso
- Modo proactivo: los agentes pueden iniciar sugerencias y acciones de forma proactiva

**Control de computadora**: clics de ratón, entrada de teclado y desplazamiento de pantalla impulsados por IA, con permisos de tres niveles (predeterminado / aceptar edición / acceso completo) y aislamiento de rutas en sandbox

**Automatización de navegador**: control del navegador mediante el protocolo CDP, con soporte de navegación, capturas de pantalla, clics, relleno de formularios y extracción de texto

### Sistema de habilidades

- **Mercado de habilidades**: explorar e instalar habilidades de la comunidad
- **Creación asistida por IA**: creación automática de la estructura de una habilidad a partir de una propuesta en lenguaje natural (`skill:create`)
- **Evolución de habilidades** (`evolution_engine`): análisis automático y mejora de habilidades basado en la retroalimentación de la ejecución
- **Coincidencia semántica**: recomendación automática de habilidades relevantes según la semántica del contexto de la conversación
- **Descomposición de habilidades** (`skill_decomposition`): descomposición automática de tareas complejas en combinaciones de habilidades atómicas
- **Generación de herramientas**: la IA genera y registra nuevas herramientas
- **Ejecución en sandbox**: las habilidades se ejecutan de forma segura en un sandbox aislado

### Flujo de trabajo visual

Editor de flujos de trabajo DAG de arrastrar y soltar basado en ReactFlow 12:

- **32 tipos de nodos**: disparador, agente, llamada a LLM, rama condicional, bifurcación paralela, bucle, fusión, retardo, llamada a herramienta, ejecución de código, subflujo de trabajo, recuperación vectorial, análisis de documentos, validación, fin, solicitud HTTP, Switch, consulta de base de datos, notificación, aprobación, operación de archivos, transformación de datos, envío de Webhook, registro, clasificador de LLM, agregador, correo, debate, Swarm, multi-agente, almacenamiento, reglas de negocio
- **Ejecución con ordenamiento topológico de Kahn**: detección automática de dependencias circulares y programación de canalizaciones paralelas
- **Plantillas integradas**: revisión de código, corrección de errores, generación de documentación, pruebas, refactorización, exploración, análisis de rendimiento, revisión de seguridad, desarrollo de funciones
- **Serialización YAML**: importación y exportación de definiciones de flujos de trabajo
- **Gestión de versiones**: control de versiones de plantillas
- **Diseño asistido por IA**: diseño de flujos de trabajo asistido por IA, recomendación de nodos y diagnóstico

### Gestión del conocimiento

- **RAG de múltiples bases de conocimiento**: carga de documentos → análisis automático (PDF/DOCX/XLSX/PPTX/TXT) → fragmentación → indexación vectorial
- **Recuperación híbrida**: similitud vectorial (sqlite-vec + incrustaciones locales con candle) + búsqueda de texto completo BM25 (FTS5), con clasificación híbrida
- **Self-RAG**: reflexión y validación automáticas de los resultados de la recuperación
- **Reordenación**: reordenación de resultados con Cross-encoder
- **Grafo de conocimiento**: extracción de entidades → construcción de relaciones → grafo visual
- **Monitoreo de archivos**: monitoreo de cambios de archivos en tiempo real basado en `notify`, con indexación incremental automática
- **LLM Wiki**: compilador y validador de Wiki asistido por IA

### Sistema de memoria

- **Memoria de múltiples espacios de nombres**: aislada por proyecto/tema, con soporte de entrada manual y extracción automática por IA
- **Integración de persistencia**: memoria de bucle cerrado con Honcho y Mem0
- **Perfil de usuario**: aprendizaje automático del estilo de código, preferencias de pila tecnológica y estilo de comunicación
- **Transferencia de estilo**: extracción de características del estilo de código → aplicación al código generado por IA
- **Integración onírica**: integración automática en segundo plano de fragmentos de memoria y patrones de comportamiento para generar conocimiento estructurado
- **Memoria de proyecto**: persistencia de contexto por dimensión de proyecto

### Pasarela de API

Pasarela HTTP + WebSocket integrada basada en `axum`:

- **Puntos finales compatibles**: OpenAI `/v1/chat/completions`, API de mensajes de Claude, API de Gemini, así como OpenAI Responses y Realtime WebSocket
- **Gestión de claves**: generación, revocación, habilitación/deshabilitación de claves de acceso, con soporte de caducidad
- **Seguimiento de uso**: estadísticas de volumen de solicitudes y consumo de tokens por clave/proveedor/fecha, exportación de métricas a Prometheus
- **Limitación de velocidad**: algoritmo de cubeta de fichas basado en `governor`
- **SSL/TLS**: certificado autofirmado integrado (`rcgen`), con soporte de certificados personalizados
- **Enlaces externos**: integración con un clic de herramientas externas como Claude CLI y OpenCode, con sincronización automática de claves de API
- **Tickets en tiempo real**: tickets de autenticación temporal basados en HMAC para la transmisión segura de conexiones WebSocket
- **Modo servidor**: binario opcional `axagent-server` que expone las capacidades de la aplicación de escritorio como servicio

### Integración de plataformas de mensajería

Pasarela multiplataforma implementada con `rt-messaging`, con soporte de recepción de mensajes, análisis de comandos y respuesta automática de IA para **DingTalk, Feishu, QQ, Slack, WeChat, WhatsApp, Telegram y Discord**.

### Sistema de herramientas

**163+ herramientas integradas**, registradas de forma unificada mediante el trait `Tool`, que cubren 15 categorías principales:

| Categoría            | Ejemplos de herramientas                                                                                                                                                    |
| -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Archivos             | `file_read`, `file_write`, `file_edit`, `glob`, `grep`, 11 en total incluyendo directorio/eliminar/mover, etc.                                                              |
| Shell/Web            | `bash`, `web_fetch`, `web_search`                                                                                                                                           |
| Red                  | `http_request`, `ping`, `dns_lookup`, `json_api`, `rss_reader`, `graphql`, `websocket`                                                                                      |
| Navegador            | `browser_navigate`, `browser_click`, `browser_fill`, `browser_screenshot`, 10 en total (CDP)                                                                                |
| Control de PC        | `computer_use` (ratón/teclado/capturas de pantalla)                                                                                                                         |
| Git                  | `git_status`, `git_diff`, `git_commit`, `git_log`, `git_branch`, `git_review`                                                                                               |
| Base de conocimiento | `list_knowledge_bases`, `search_knowledge`, `add_knowledge_document`, 6 en total                                                                                            |
| Gestión de tareas    | `todo_write`, `task_*` (6), `cron_*` (3), relacionados con `plan`                                                                                                           |
| Notificaciones       | `push_notification`, `send_message`, herramientas de colaboración en equipo                                                                                                 |
| Base de datos        | `database_query`, `database_list_tables`, `database_migration_status`                                                                                                       |
| Almacenamiento       | `get_storage_info`, `upload_storage_file`, `download_storage_file`, 5 en total                                                                                              |
| Exportación/formato  | `export_word`, `export_pdf`, `export_xlsx`, `export_pptx`, `render_markdown`, 9 en total                                                                                    |
| OCR                  | `ocr_image`, `ocr_detect_langs`                                                                                                                                             |
| Obsidian             | `obsidian_search`, `obsidian_read`, `obsidian_backlinks`, 9 en total                                                                                                        |
| Otros                | `agent`, `delegate_task`, `skills_*`, `lsp`, `repl`, `monitor`, `workspace_*`, `session_search`, `generate_image`, `sequential_thinking`, CI/CD, DevOps, RPC, pruebas, etc. |

### Protocolo MCP

Implementación completa de MCP (Model Context Protocol) basada en `rmcp`:

- **Capa de transporte**: subprocesos stdio + Streamable HTTP + SSE
- **Autenticación OAuth**: soporte del flujo de autorización OAuth para servidores MCP
- **Descubrimiento de herramientas**: descubrimiento y registro automáticos de las herramientas expuestas por los servidores MCP
- **Gestor de MCP**: gestión del ciclo de vida de los servidores, comprobaciones de salud y reconexión automática

### Sistema de complementos

Arquitectura de complementos de tres niveles compatible con OpenClaw (integrados/empaquetados/externos):

- Instalación mediante paquetes npm, con interfaz de mercado integrada para buscar e instalar
- Definición del manifiesto del complemento, declaración de permisos y ejecución aislada en sandbox
- Registro de herramientas personalizadas, proveedores de Agent y interceptación de Hooks
- Instalador de habilidades: instala habilidades desde paquetes de complementos al sistema de habilidades

### Motor de UI dinámica

- **Impulsado por esquema**: construcción declarativa de interfaces mediante JSON Schema, sin necesidad de escribir código
- **31 componentes integrados**: contenedores (7) / visualización de datos (6) / formularios (9) / medios (4) / otros (5)
- **Enlace de datos**: enlace declarativo de fuentes de datos y renderizado condicional
- **NL2UI**: generación directa de interfaces de UI dinámicas a partir de lenguaje natural

### SDK de cliente ACP

- **ACP (Agent Client Protocol)**: SDK bilingüe (TypeScript + Python), sin dependencias de terceros
- Gestión de sesiones, envío de Prompts, registro de llamadas a herramientas y flujo de eventos WebSocket
- Comunicación con el servicio AxAgent a través del punto final `/acp/v1/*`

### Seguridad

- **Cifrado AES-256-GCM**: almacenamiento cifrado local de claves de API y configuración sensible (crate `crypto`)
- **Protección contra inyección de prompts**: canalización de defensa de cuatro niveles (`prompt-guard`) — detección de patrones → escape de delimitadores → envoltorio XML → etiquetas de confianza, integrada en toda la cadena de sesiones, construcción de prompts, Git y RAG
- **Protección SSRF**: comprobación de seguridad de URLs para bloquear solicitudes a direcciones de red interna
- **Filtrado de contenido**: filtrado de seguridad de contenido de múltiples tipos
- **Limitación de velocidad**: limitación de tipo cubeta de fichas para llamadas a herramientas y solicitudes de API
- **Cortacircuitos**: cortocircuito automático tras fallos consecutivos
- **Control de acceso**: control de acceso a herramientas basado en políticas
- **Aislamiento en sandbox**: aislamiento de los entornos de ejecución de agentes y habilidades

### Herramientas para desarrolladores

- **Trazado distribuido** (`telemetry`): integración de OpenTelemetry, visualización de Span/Trace
- **Registro estructurado**: tracing-subscriber + marcas de tiempo con chrono
- **Depuración con reproducción**: grabación (`trajectory_recorder`) y reproducción de las trayectorias de ejecución de los agentes
- **Panel DevTools**: visor de línea de tiempo Trace Explorer, Benchmark Runner y Tool Recommender
- **Pruebas de referencia**: benchmarks de Criterion (tool_exec / llm_call / search)
- **Comprobaciones de CI**: `npm run ci:check` integra comprobación de tipos, lint y validación de formato

### Experiencia de escritorio y móvil

- **Diseño adaptable**: puntos de interrupción CSS adaptados a escritorio/tableta/teléfono (diseños de 3 niveles de dispositivo: `desktop` / `tablet` / `mobile`)
- **11 idiomas**: chino simplificado, chino tradicional, inglés, japonés, coreano, francés, alemán, español, ruso, hindi y árabe
- **Motor de temas** (`rt-theme`): temas oscuro/claro + varios ajustes preestablecidos, personalización profunda de Ant Design 6
- **Editor Monaco**: resaltado de sintaxis, vista previa de diferencias y soporte multilingüe
- **Terminal xterm.js**: WebLinks, Unicode 11 y búsqueda
- **Desplazamiento virtual**: @tanstack/react-virtual + react-virtuoso
- **Renderizado de gráficos**: D2 + Mermaid + Recharts + Sigma (grafos)
- **Paleta de comandos**: panel global de comandos con Ctrl+K
- **Bandeja del sistema + atajos globales + inicio automático**: ejecución en segundo plano sin distracciones
- **Actualización automática**: detección de versiones en GitHub Releases con intervalo configurable
- **Soporte de proxy**: configuración de proxy HTTP / SOCKS5
- **Espacio de trabajo en la nube**: sincronización de almacenamiento con S3 y WebDAV, con detección de conflictos y sincronización bidireccional

### Móvil

- Android APK/AAB (arm64-v8a, armeabi-v7a, x86_64)
- iOS IPA (arm64)
- Adaptación específica para móvil: adaptación de zonas seguras, navegación inferior y navegación con Drawer

---

## Arquitectura técnica

### Pila tecnológica

| Capa                        | Tecnología                               | Versión |
| --------------------------- | ---------------------------------------- | ------- |
| Marco de escritorio         | Tauri                                    | 2.11    |
| Marco frontend              | React                                    | 19      |
| Sistema de tipos            | TypeScript                               | 7       |
| Librería de UI              | Ant Design                               | 6       |
| Marco CSS                   | TailwindCSS                              | 4       |
| Gestión de estado           | Zustand                                  | 5       |
| Enrutamiento                | React Router                             | 7       |
| Editor de código            | Monaco Editor                            | 0.55    |
| Terminal                    | xterm.js                                 | 6       |
| Editor de flujos de trabajo | ReactFlow                                | 12      |
| Gráficos                    | D2 + Mermaid + Recharts + Sigma          |         |
| Animación                   | Framer Motion                            | 12      |
| Desplazamiento virtual      | @tanstack/react-virtual + react-virtuoso |         |
| Arrastrar y soltar          | @dnd-kit                                 | 6       |
| Renderizado Markdown        | markstream-react + stream-markdown       |         |
| Internacionalización        | i18next + react-i18next                  |         |
| Herramienta de compilación  | Vite                                     | 8       |
| Pruebas                     | Vitest + Playwright                      |         |
| Formato                     | dprint (TS/JSON/Markdown/TOML) + rustfmt |         |
| Lint                        | ESLint + Oxlint + Clippy                 |         |

### Arquitectura backend: patrón de inyección de dependencias Harness

Arquitectura de workspace de Rust con **37 miembros** (crate principal + 35 crates de librería + schema-gen), que sigue la **arquitectura de inyección de dependencias Harness**:

> Todos los crates se desacoplan a través de las interfaces de trait definidas por axagent-harness; el runtime los ensambla e inyecta las dependencias en tiempo de ejecución.
> Dirección de dependencias: `implementación concreta → harness ← llamador`

**harness** es la piedra angular de la arquitectura — sin lógica de negocio, sin implementaciones concretas, solo definiciones de traits, DTOs de datos puros, constantes y tipos de error unificados. Es dependido por todos los demás crates y no depende de ningún crate axagent-* (más de 200 definiciones de traits que cubren Agent/Provider/Tool/RAG/almacenamiento/MCP/complementos/seguridad/observabilidad/memoria/aprendizaje/navegador/mensajería/enrutamiento cognitivo, etc.).

```
src-tauri/crates/
├── harness/          # Piedra angular de la arquitectura — interfaces de trait, DTOs, tipos de error, contratos DI
├── entities/         # Modelos de entidad SeaORM
├── dao/              # Capa de acceso a datos (CRUD)
├── migration/        # Migraciones de base de datos
├── crypto/           # Cifrado/descifrado AES-256-GCM y gestión de claves
├── credential/       # Almacenamiento seguro de credenciales
├── storage/          # Abstracción de almacenamiento de archivos (local/S3/WebDAV), lectura/escritura ZIP
├── cache/            # Capa de caché en memoria
├── disk-cache/       # Caché a nivel de archivo en disco
├── search/           # Motor de búsqueda (FTS5 + sqlite-vec + incrustaciones locales con candle)
├── document-parser/  # Extracción de texto de documentos (PDF/DOCX/XLSX/PPTX)
├── kit/              # Conjunto de utilidades generales (rutas/codificación/hashes/fechas)
├── runtime-core/     # Tipos comunes del runtime, constantes de configuración
├── runtime/          # Orquestación de servicios del runtime — contenedor DI que ensambla todos los crates
├── rt-workflow/      # Motor de flujos de trabajo — orquestación DAG, ejecutores de nodos, serialización YAML
├── rt-messaging/     # Pasarela de plataformas de mensajería — DingTalk/Feishu/QQ/Slack/WeChat/WhatsApp/Telegram/Discord
├── rt-webhook/       # Servidor Webhook genérico
├── rt-dashboard/     # Marco de complementos de panel de control
├── rt-theme/         # Motor de temas
├── agent/            # Núcleo del agente de IA — 80+ módulos
│                     #   Motor ReAct/planificación jerárquica/investigación profunda/verificación de hechos/árbol de pensamiento/reflexión/
│                     #   auto-verificación/recuperación de errores/optimización RL/ajuste fino LoRA/evaluación/recomendación de herramientas/pruebas A/B/
│                     #   coordinador/pizarra/canalización visual/búsqueda web/búsqueda académica/compilación Wiki, etc.
├── orchestrator/     # Orquestación de agentes — programación multi-agente, descomposición DAG, ejecución de subgrafos dinámicos
├── providers/        # Adaptadores de proveedores de modelos (13)
├── tools/            # Sistema de herramientas — trait Tool/registro/orquestación/streaming/sandbox/163+ herramientas integradas
├── gateway/          # Pasarela de API — servidor axum HTTP/WS, OAuth, limitación de velocidad, Prometheus
├── mcp/              # Protocolo MCP — stdio + Streamable HTTP + SSE, basado en rmcp
├── trajectory/       # Sistema de aprendizaje — memoria/evolución de habilidades/perfil de usuario/integración onírica
├── plugins/          # Sistema de complementos — compatible con OpenClaw, instalación de paquetes npm, mercado
├── telemetry/        # Observabilidad — OpenTelemetry, registro estructurado, métricas del runtime
├── prompt-guard/     # Protección contra inyección de prompts — canalización de detección multinivel L1-L4
├── npm/              # Cliente del registro npm
├── crdt/             # Estructuras de datos para edición colaborativa
├── device/           # Gestión de dispositivos
├── axagent-mobile/   # Capa de adaptación móvil
├── agent-macro/      # Macros de agente
├── agent-command-types/ # Tipos de comandos de agente
└── schema-gen/       # Herramienta de generación de esquemas de base de datos
```

### Arquitectura frontend

```
src/
├── pages/            # Páginas (24)
│   ├── ChatPage           # Interfaz principal de conversación — barra lateral/flujo de mensajes/panel de Agent/multi-pestañas
│   ├── DashboardPage      # Panel de datos — estadísticas de uso/distribución de modelos/gráficos de tendencias
│   ├── WorkflowPage       # Editor de flujos de trabajo — visualización DAG con ReactFlow
│   ├── KnowledgeHubPage   # Gestión de base de conocimiento — carga de documentos/indexación/búsqueda
│   ├── MemoryPage         # Gestión de memoria
│   ├── SkillsPage         # Mercado de habilidades
│   ├── SettingsPage       # Panel de configuración — 40+ opciones de configuración
│   ├── TerminalPage       # Terminal integrado — xterm.js
│   ├── FilesPage          # Gestión de archivos
│   ├── GatewayLinkPage    # Pasarela de API y gestión de enlaces externos
│   ├── QuickBarPage       # Barra rápida (ventana independiente)
│   ├── DynamicUIManagerPage / DynamicPageViewer  # Motor de UI dinámica
│   ├── WikiGraphPage / WikiEditPage / IngestPage # LLM Wiki
│   ├── LearningGraphPage  # Grafo de aprendizaje
│   ├── FineTunePage       # Ajuste fino LoRA
│   ├── PersonaPage        # Gestión de roles
│   ├── WorkflowMarketplace # Mercado de flujos de trabajo
│   ├── DevTools/          # TraceExplorer / BenchmarkRunner / ToolRecommender
│   └── Workflow/          # WorkflowListPage
│
├── components/       # 33 módulos, 500+ componentes
│   ├── chat/         # Conversación (flujo de mensajes/entrada/ChatView/TabBar/RightPanel/adjuntos/renderizado de llamadas a herramientas)
│   ├── layout/       # Diseño — AppInitializer / Sidebar / ContentArea / TitleBar /
│   │                 #   CommandPalette / GlobalCopyMenu / GlobalErrorBoundary /
│   │                 #   GlobalStatusBar / ErrorNotificationToast / AppHeader, etc.
│   ├── agent/        # Panel de Agent/entrada/panel en miniatura
│   ├── workflow/     # Editor de flujos de trabajo (nodos/conexiones/paneles/plantillas/asistencia de IA)
│   ├── settings/     # Panel de configuración (40+ subcomponentes)
│   ├── skill/        # Editor/renderizador/panel flotante de habilidades
│   ├── dynamicUI/    # Componentes de UI dinámica (31 componentes integrados)
│   ├── gateway/      # Gestión de la pasarela de API
│   ├── files/        # Gestión de archivos
│   ├── terminal/     # Componentes de terminal
│   ├── search/       # Interfaz de búsqueda
│   ├── benchmark/    # Panel de pruebas de referencia
│   ├── decomposition/# Descomposición de habilidades y generación de herramientas
│   ├── devtools/     # Línea de tiempo Trace/Span + panel de entrenamiento RL
│   ├── approval/     # Interfaz de flujo de aprobación
│   ├── recommendation/ # Recomendación de herramientas/modelos
│   ├── onboarding/   # WelcomeWizard / InteractiveTutorial
│   ├── help/         # Panel de ayuda
│   ├── notification/ # Componentes de notificación
│   ├── proactive/    # Sugerencias proactivas
│   ├── llm-wiki/     # Componentes de LLM Wiki
│   ├── wiki/         # Componentes de Wiki
│   ├── fine-tune/    # Interfaz de ajuste fino
│   ├── trace/        # Componentes de Trace
│   ├── style/        # Estilos/temas
│   ├── shared/       # Componentes compartidos (ErrorBoundary / PageContextProvider)
│   └── common/       # Componentes generales (Icon, etc.)
│
├── stores/           # Gestión de estado con Zustand (82 stores)
│   ├── domain/       # 9 stores de negocio principales (conversación/flujo/compresión/preferencias/multimodelo, etc.)
│   ├── feature/      # 61 stores de módulos funcionales (agente/flujo de trabajo/base de conocimiento/habilidades/pasarela/memoria/terminal, etc.)
│   ├── shared/       # 8 stores compartidos entre componentes (UI/pestañas/espacio de trabajo/estado del backend, etc.)
│   └── devtools/     # 4 stores de herramientas para desarrolladores
│
├── hooks/            # React Hooks (atajos/paleta de comandos/adaptabilidad/barra de desplazamiento/tema/Avatar, etc.)
├── lib/              # Librería de funciones de utilidad (invoke/pageRegistry/shortcuts/skillLifecycle/
│                     #   chartGenerator/codeExecutor/tokenEstimator/workflowLayout, 45+ módulos)
├── types/            # Definiciones de tipos TypeScript
├── theme/            # Motor de temas Shadcn
├── i18n/             # Archivos de traducción de 11 idiomas (zh-CN/zh-TW/en-US/ja/ko/fr/de/es/ru/hi/ar)
├── constants/        # Constantes e interruptores de funciones
└── sdk/              # SDK de cliente ACP (TypeScript + Python)
```

### Interruptores de funciones

El proyecto gestiona el lanzamiento progresivo de funciones mediante `featureFlags.ts`:

| Interruptor         | Estado | Descripción                                                  |
| ------------------- | ------ | ------------------------------------------------------------ |
| `AGENT_IN_THE_LOOP` | ✅     | Panel de Agent global + inyección de contexto de página      |
| `DYNAMIC_UI`        | ✅     | Motor de construcción de UI dinámica                         |
| `SELF_EVOLUTION_UI` | ❌     | Panel de control frontend de auto-evolución                  |
| `NL_EXTENSION`      | ❌     | Extensión de negocio dinámica impulsada por lenguaje natural |

### Complementos de Tauri

| Complemento         | Uso                                   |
| ------------------- | ------------------------------------- |
| `autostart`         | Inicio automático                     |
| `clipboard-manager` | Lectura/escritura del portapapeles    |
| `dialog`            | Diálogo de selección de archivos      |
| `fs`                | Acceso al sistema de archivos         |
| `global-shortcut`   | Registro de atajos globales           |
| `notification`      | Notificaciones del sistema            |
| `opener`            | Apertura de enlaces/archivos externos |
| `process`           | Gestión de procesos                   |
| `updater`           | Actualización automática              |

---

## Directorios de datos

```
~/.axagent/                    # Configuración de la aplicación
├── axagent.db                 # Base de datos principal SQLite (SeaORM)
├── master.key                 # Clave maestra AES-256
├── vector_db/                 # Índice vectorial sqlite-vec
└── ssl/                       # Certificados SSL autofirmados

~/Documents/axagent/          # Archivos del usuario
├── images/                   # Adjuntos de imágenes
├── files/                    # Adjuntos de archivos
└── backups/                  # Copias de seguridad automáticas
```

---

## Inicio rápido

### Requisitos del entorno

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+ (edition 2024)
- [npm](https://www.npmjs.com/) 10+
- Windows: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (MSVC + Windows SDK)
- macOS: Xcode Command Line Tools
- Linux: `build-essential` + `libwebkit2gtk-4.1-dev` + `libssl-dev`

### Desarrollo

```bash
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent
npm install
npm run tauri dev      # Modo desarrollo (Vite HMR del frontend + ventana de Tauri)
```

### Compilación

```bash
npm run tauri build    # Compilación de producción de escritorio

npm run tauri:android:build   # Compilación de Android
npm run tauri:ios:build       # Compilación de iOS
```

Los artefactos de compilación de escritorio se encuentran en `src-tauri/target/release/`.

### Pruebas

```bash
npm run test           # Pruebas unitarias del frontend (watch de Vitest)
npm run test:run       # Pruebas unitarias del frontend (ejecución única)
npm run test:e2e       # Pruebas E2E (Playwright)

# Pruebas del backend Rust
cd src-tauri && cargo test

# Comprobación de tipos y Lint
npm run typecheck
cd src-tauri && cargo clippy -- -D warnings
npm run format         # Formato con dprint
npm run lint:eslint    # Comprobación de ESLint
npm run contracts      # Comprobación de contratos de API

# Comprobación completa de CI
npm run ci:check
```

### Scripts habituales

| Comando                  | Uso                                      |
| ------------------------ | ---------------------------------------- |
| `npm run bump`           | Actualización de versión (interactiva)   |
| `npm run docs`           | Genera documentación TypeDoc             |
| `npm run skill:create`   | Crea el andamiaje de una nueva habilidad |
| `npm run skill:validate` | Valida la definición de una habilidad    |
| `npm run check:types`    | Comprobación de consistencia de tipos    |

---

## Plataformas compatibles

| Plataforma | Arquitectura                          |
| ---------- | ------------------------------------- |
| Windows    | x86_64, ARM64                         |
| macOS      | Apple Silicon (arm64), Intel (x86_64) |
| Linux      | x86_64, ARM64                         |
| Android    | arm64-v8a, armeabi-v7a, x86_64        |
| iOS        | arm64                                 |

---

## Licencia de código abierto

Este proyecto está publicado bajo la licencia [AGPL-3.0-only](LICENSE).

---

## Agradecimientos

AxAgent se construye sobre numerosos proyectos de código abierto excelentes:

- [Tauri](https://tauri.app/) — marco de escritorio multiplataforma
- [React](https://react.dev/) + [Ant Design](https://ant.design/) — UI del frontend
- [SeaORM](https://www.sea-ql.org/SeaORM/) — ORM de Rust
- [sqlite-vec](https://github.com/asg017/sqlite-vec) — búsqueda vectorial
- [candle](https://github.com/huggingface/candle) — inferencia de incrustaciones local
- [rmcp](https://github.com/nicholasxjy/rmcp) — SDK de MCP para Rust
- [ReactFlow](https://reactflow.dev/) — editor visual de flujos de trabajo
- [axum](https://github.com/tokio-rs/axum) — marco HTTP
- [Monaco Editor](https://microsoft.github.io/monaco-editor/) — editor de código
- [xterm.js](https://xtermjs.org/) — emulador de terminal
- [Zustand](https://zustand.docs.pmnd.rs/) — gestión de estado
- [Framer Motion](https://www.framer.com/motion/) — librería de animación
- [Recharts](https://recharts.org/) — librería de gráficos
