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

**AxAgent** es un cliente de escritorio de asistente de IA de código abierto y multiplataforma que admite **Windows / macOS / Linux / Android / iOS**. Va mucho más allá de una interfaz de chat: integra un motor de agentes ReAct, orquestación visual de flujos de trabajo, bases de conocimiento RAG locales, extensiones del protocolo MCP, una pasarela unificada multimodelo, automatización del navegador y control del ordenador, sirviendo como una estación de trabajo de IA para el desarrollo diario, la investigación, la gestión del conocimiento y la automatización.

> **Idiomas**: [简体中文](./README.md) | [English](./README-EN.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

---

## Qué resuelve AxAgent

AxAgent aborda tres problemas centrales:

1. **Orquestación multimodelo unificada**: Use OpenAI, Anthropic Claude, Google Gemini, modelos locales Ollama y cualquier API compatible con OpenAI en una sola interfaz, con rotación multi-clave, enrutamiento inteligente de modelos y comparación en streaming
2. **Operacionalización de la capacidad de IA**: Ampliar la IA de "conversación" a "ejecución" — mediante más de 47 herramientas integradas, flujos de trabajo visuales, extensiones MCP, automatización del navegador y control del ordenador, permitiendo que la IA manipule archivos, ejecute código, gestione Git y programe tareas directamente
3. **Soberanía de datos local-first**: Las conversaciones de IA, las bases de conocimiento, las memorias y los archivos de configuración se almacenan todos en una base de datos SQLite local. Las claves de API se cifran con AES-256-GCM. Las funciones centrales se ejecutan sin servicios en la nube de terceros.

---

## Capacidades principales

### Motor multimodelo

- **9 adaptadores de proveedores**: OpenAI (Chat Completions + Responses + Realtime), Anthropic Claude, Google Gemini, Ollama (con gestión GGUF), OpenClaw, Hermes y todas las API compatibles con OpenAI
- **Rotación multi-clave**: Configure múltiples claves de API por proveedor con rotación automática basada en cuotas para evitar interrupciones por límite de velocidad
- **Enrutamiento inteligente**: Selección automática del modelo más adecuado según el tipo de tarea (revisión de código / resumen / traducción / general), con reglas de enrutamiento personalizables
- **Monitoreo de salud de proveedores**: Seguimiento en tiempo real de tasas de éxito, latencia y disponibilidad por proveedor, con degradación automática por niveles (ProviderTier)
- **Generación de imágenes con IA**: DALL-E 3 y Flux (Replicate) con ajustes preestablecidos de varios tamaños
- **Voz en tiempo real**: Conversación de voz WebSocket basada en la API OpenAI Realtime, con interrupción y transcripción en streaming

### Sistema de agentes

Todo el sistema de agentes se construye sobre un **motor ReAct (Reasoning + Acting)**, con los siguientes subsistemas implementados:

- **Planificador jerárquico** (`hierarchical_planner`): Descompone tareas complejas en planes estructurados Phase → Task con relaciones de dependencia, compilados a ejecución topológica DAG
- **Investigación en profundidad** (`deep_research`): Orquestación de búsqueda multi-fuente que incluye planificación de búsqueda (`search_planner`), ejecución de búsqueda (`search_orchestrator`), síntesis de contenido (`content_synthesizer`) y seguimiento de citas (`citation_tracker`)
- **Verificador de hechos** (`fact_checker`): Verificación de hechos impulsada por IA con clasificador de fuentes (`source_classifier`), validador de fuentes (`source_validator`) y evaluador de credibilidad (`credibility_evaluator`)
- **Árbol de pensamientos** (`tree_of_thoughts`): Exploración de razonamiento de múltiples rutas con evaluación de ramas y retroceso
- **Reflexor** (`reflector`): Autoevaluación y sugerencias de mejora tras la tarea
- **Autoverificador** (`self_verifier`): Validación automática de resultados de razonamiento con detección de ciclos (`cycle_detector`) para prevenir bucles infinitos
- **Recuperación de errores** (`error_recovery_engine`): Clasificar tipos de error → seleccionar estrategia de recuperación → reintento automático o ajuste del plan, con retroceso exponencial
- **Pruebas A/B** (`ab_testing`): Evaluación comparativa de diferentes estrategias de razonamiento
- **Sistema de evaluación** (`evaluator`): Framework de benchmark integrado con soporte para conjuntos de datos, métricas y generación de informes
- **Ajuste fino LoRA** (`fine_tune`): Pipeline de entrenamiento integrado con gestión de adaptadores LoRA
- **Optimizador RL** (`rl_optimizer`): Aprendizaje por refuerzo basado en retroalimentación de experiencia con experience replay y gradientes de política
- **Recomendador de herramientas** (`tool_recommender`): Análisis y recomendación de patrones de uso de herramientas basado en el contexto

**Colaboración multi-agente**:

- Arquitectura de coordinación maestro-esclavo (`coordinator`) con ejecución paralela de sub-agentes y programación consciente de dependencias
- Pizarra compartida (`shared_blackboard`) para el intercambio de información entre agentes
- Modo de debate adversarial con rondas a favor/en contra y puntuación de fuerza de argumentos
- Modo de enjambre (swarm) con clústeres de agentes multiproceso que admiten sincronización de permisos y reconexión automática
- Modo proactivo (`proactive_mode`): Los agentes pueden proponer proactivamente sugerencias y acciones

**Control del ordenador**: Clics de ratón, entradas de teclado y desplazamiento de pantalla impulsados por IA con tres niveles de permisos (Default / Accept Edits / Full Access) y aislamiento de rutas en sandbox

**Automatización del navegador**: Control del navegador a través del protocolo CDP con soporte para navegación, capturas de pantalla, clics, relleno de formularios, extracción de texto y monitoreo del estado de la página

### Sistema de habilidades (Skill System)

- **Mercado de habilidades**: Explore e instale habilidades de la comunidad
- **Creación asistida por IA**: Creación automática de estructuras de habilidades a partir de propuestas en lenguaje natural
- **Evolución de habilidades** (`evolution_engine`): Analiza y mejora automáticamente las habilidades basándose en la retroalimentación de ejecución
- **Coincidencia semántica** (`skill`): Coincidencia semántica y recomendación automática de habilidades relevantes basándose en el contexto de la conversación
- **Descomposición de habilidades** (`skill_decomposition`): Descompone automáticamente tareas complejas en combinaciones de habilidades atómicas
- **Herramientas generadas** (`generated_tool`): Nuevas herramientas generadas y registradas por IA
- **Ejecución en sandbox** (`sandbox`): Las habilidades se ejecutan de forma segura en entornos sandbox aislados

### Flujo de trabajo visual

Editor de flujos de trabajo DAG de arrastrar y soltar basado en ReactFlow 12:

- **17 tipos de nodos**: Trigger, Agent, LLM Call, Conditional Branch, Parallel Fork, Loop, Merge, Delay, Tool Call, Code Execution, Sub-workflow, Vector Retrieval, Document Parsing, Validation, End, Business Rule, Agent Role
- **Ejecución con orden topológico de Kahn**: Detección automática de dependencias cíclicas con programación de canalización paralela
- **Plantillas integradas**: Revisión de código, Corrección de bugs, Generación de documentos, Pruebas, Refactorización, Exploración, Análisis de rendimiento, Auditoría de seguridad, Desarrollo de funciones
- **Serialización YAML**: Las definiciones de flujos de trabajo admiten importación/exportación YAML
- **Gestión de versiones**: Control de versiones para plantillas de flujos de trabajo
- **Asistencia de IA**: Diseño de flujos de trabajo asistido por IA y recomendaciones de nodos

### Gestión del conocimiento

- **RAG multi-KB**: Carga de documentos → análisis automático (PDF/DOCX/XLSX/PPTX/TXT) → fragmentación → indexación vectorial
- **Recuperación híbrida**: Similitud de vectores (sqlite-vec + embeddings locales candle) + búsqueda de texto completo BM25 (FTS5) con clasificación híbrida
- **Self-RAG**: Generación aumentada por recuperación propia con reflexión y verificación automática de los resultados de recuperación
- **Re-ranking**: Re-clasificación de resultados con cross-encoder para mejorar la precisión
- **Grafo de conocimiento**: Extracción de entidades (`EntityExtractor`) → construcción de relaciones → grafo visual
- **Observación de archivos**: Monitoreo en tiempo real de cambios en archivos a través de `notify` con indexación incremental automática
- **Wiki LLM**: Compilador y validador de Wiki asistido por IA con extensión de navegador para recorte de Wiki

### Sistema de memoria

- **Memoria multi-namespace**: Aislada por proyecto/tema, con entrada manual y extracción automática por IA
- **Integración de persistencia**: Memoria de ciclo cerrado con Honcho y Mem0
- **Perfil de usuario** (`user_profile` / `profile`): Aprendizaje automático del estilo de codificación (sangría/nombrado/comentarios), preferencias de stack tecnológico y estilo de comunicación
- **Transferencia de estilo** (`style`): Extrae características del estilo de código → las aplica al código generado por IA
- **Integración Dream** (`dream`): Consolidación automática en segundo plano de fragmentos de memoria y patrones de comportamiento en conocimiento estructurado
- **Memoria de proyecto** (`project_memory`): Persistencia de contexto por proyecto

### Pasarela de API (API Gateway)

Servidor de pasarela HTTP + WebSocket integrado basado en `axum`:

- **Endpoints compatibles**: OpenAI `/v1/chat/completions`, API de Mensajes de Claude, API de Gemini, además de OpenAI Responses y Realtime WebSocket
- **Gestión de claves**: Genere, revoque, active/desactive claves de acceso con soporte de caducidad
- **Seguimiento de uso**: Conteo de solicitudes y estadísticas de consumo de tokens por clave, proveedor y fecha con exportación de métricas Prometheus
- **Limitación de velocidad**: Algoritmo de token bucket a través de `governor` con políticas de límite de velocidad configurables
- **SSL/TLS**: Certificados autofirmados integrados (`rcgen`) con soporte para certificados personalizados
- **Enlace externo**: Integración con un clic con Claude CLI, OpenCode y otras herramientas externas con sincronización automática de claves de API
- **Tickets en tiempo real**: Tickets de autenticación temporales basados en HMAC para una entrega segura de conexiones WebSocket en tiempo real

### Integración con plataformas de mensajería

Pasarela de plataformas de mensajería implementada a través de la crate `rt-messaging`, que admite:

DingTalk, Feishu, QQ, Slack, WeChat, WhatsApp, Telegram, Discord

Admite recepción de mensajes Webhook, análisis de comandos y entrega automática de respuestas de IA.

### Sistema de herramientas (Tool System)

47 herramientas integradas, todas registradas uniformemente a través del trait `Tool`:

| Categoría                  | Herramientas                                                                                                                                                                                               |
| -------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Operaciones de archivos    | `file_read`, `file_write`, `file_edit`, `file_system` (list/search/metadata)                                                                                                                               |
| Ejecución de código        | `bash`, `repl`                                                                                                                                                                                             |
| Búsqueda                   | `grep`, `glob`                                                                                                                                                                                             |
| Navegador                  | `browser` (control CDP)                                                                                                                                                                                    |
| Control del ordenador      | `computer_use` (ratón/teclado/captura)                                                                                                                                                                     |
| Web                        | `web_search`, `web_fetch`                                                                                                                                                                                  |
| Base de conocimiento       | `knowledge`, `document` (análisis de documentos)                                                                                                                                                           |
| Git                        | `git` (commit/push/branch/diff)                                                                                                                                                                            |
| Herramientas de desarrollo | `lsp` (Language Server Protocol), `workspace`                                                                                                                                                              |
| Gestión de tareas          | `plan`, `task_system`, `todo_write`, `cron`                                                                                                                                                                |
| Notificaciones             | `push_notification`, `messaging`                                                                                                                                                                           |
| Base de datos              | `database`                                                                                                                                                                                                 |
| Almacenamiento             | `storage`                                                                                                                                                                                                  |
| Otros                      | `agent`, `agent_memory`, `context`, `export`, `integration`, `media`, `media_delivery`, `migration_tool`, `monitor`, `obsidian`, `ocr`, `personality`, `shared_path`, `system_info`, `testing`, `worktree` |

### Protocolo MCP

Implementación completa de MCP (Model Context Protocol) basada en la crate `rmcp`:

- **Capa de transporte**: Subproceso stdio + Streamable HTTP + WebSocket
- **Autenticación OAuth**: Soporte de flujo de autorización OAuth para servidores MCP
- **Descubrimiento de herramientas**: Descubre y registra automáticamente las herramientas expuestas por los servidores MCP
- **Gestor MCP**: Gestión del ciclo de vida del servidor, comprobaciones de salud, reconexión automática

### Sistema de complementos (Plugin System)

Arquitectura de complementos de tres niveles compatible con OpenClaw (Built-in / Bundled / External), que admite:

- Instalación de paquetes npm con interfaz de mercado integrada para buscar e instalar
- Definición de manifiesto de complementos, declaraciones de permisos, ejecución aislada en sandbox
- Registro de herramientas personalizadas, proveedores de agentes, intercepción de hooks
- Instalador de habilidades: instala habilidades desde paquetes de complementos en el sistema de habilidades

### Seguridad

- **Cifrado AES-256-GCM**: Almacenamiento cifrado local para claves de API y configuración sensible (crate `crypto`)
- **Protección contra inyección de prompts**: Canal de defensa de cuatro niveles (`prompt-guard`) — detección de patrones → escape de delimitadores → envoltura XML → etiquetas de confianza, integrado en sesiones, construcción de prompts, Git y RAG en toda la canalización
- **Protección SSRF** (`ssrf_guard`): Comprobaciones de seguridad de URL que bloquean solicitudes a direcciones de red internas
- **Filtrado de contenido** (`content_filter`): Filtrado de seguridad de contenido de múltiples tipos
- **Limitación de velocidad** (`rate_limiter`): Limitación de velocidad con token bucket para llamadas a herramientas y solicitudes de API
- **Cortacircuitos** (`circuit_breaker`): Apertura automática del circuito ante fallos consecutivos para proteger la estabilidad del sistema
- **Control de acceso** (`tool_access`): Control de permisos de acceso a herramientas basado en políticas
- **Aislamiento en sandbox**: Aislamiento del entorno de ejecución para agentes y habilidades

### Experiencia del desarrollador

- **Trazado distribuido** (`telemetry`): Integración con OpenTelemetry con visualización de Span/Trace
- **Telemetría** (`telemetry`): Registro estructurado, métricas de tiempo de ejecución, recolección de eventos de rendimiento
- **Depuración con reproducción**: Grabación de la trayectoria de ejecución del agente (`trajectory_recorder`) y reproducción
- **Panel DevTools**: Visor de línea de tiempo Trace/Span integrado en el frontend
- **Framework de benchmark**: Benchmarks de Criterion (tool_exec / llm_call / search), evaluación SWE-bench y Terminal-bench

### Experiencia de escritorio y móvil

- **Diseño responsivo**: Puntos de interrupción CSS adaptativos para escritorio / tableta / móvil (600px / 900px)
- **11 idiomas**: Chino simplificado, Chino tradicional, Inglés, Japonés, Coreano, Francés, Alemán, Español, Ruso, Hindi, Árabe
- **Motor de temas** (`rt-theme`): Tema Dark/Light que sigue la preferencia del sistema o conmutación manual, profundamente personalizado con Ant Design 6
- **Editor Monaco**: Editor de código integrado con resaltado de sintaxis, vista previa de diferencias, soporte multilingüe
- **Terminal xterm.js**: Emulador de terminal integrado con soporte para WebLinks, Unicode 11, búsqueda
- **D2 / Mermaid / ECharts**: Diagramas de arquitectura, diagramas de flujo y renderizado de gráficos interactivos
- **Compartir sesión**: Generación de enlaces de compartición con un clic con permisos de acceso configurables
- **Bandeja del sistema + Atajos globales + Inicio automático**: Funcionamiento en segundo plano no intrusivo
- **Actualización automática**: Detección automática de actualizaciones de versión mediante GitHub Releases
- **Soporte de proxy**: Configuración de proxy HTTP y SOCKS5
- **Espacio de trabajo en la nube**: Sincronización de almacenamiento S3 y WebDAV con detección de conflictos y sincronización bidireccional

### Móvil

- Android APK/AAB (arm64-v8a, armeabi-v7a, x86_64)
- iOS IPA (arm64)
- Adaptaciones específicas móviles: adaptación de área segura, barra de navegación inferior, navegación Drawer

---

## Arquitectura técnica

### Stack tecnológico

| Capa                    | Tecnología                               |
| ----------------------- | ---------------------------------------- |
| Framework de escritorio | Tauri 2.11                               |
| Framework frontend      | React 19 + TypeScript                    |
| Biblioteca de UI        | Ant Design 6 + TailwindCSS 4             |
| Gestión de estado       | Zustand 5                                |
| Enrutamiento            | React Router 7                           |
| Editor de código        | Monaco Editor                            |
| Terminal                | xterm.js 6                               |
| Editor de flujos        | ReactFlow 12                             |
| Gráficos                | D2 + Mermaid + Recharts + ECharts        |
| Desplazamiento virtual  | @tanstack/react-virtual + react-virtuoso |
| Arrastrar y soltar      | @dnd-kit                                 |
| Renderizado Markdown    | markstream-react + stream-markdown       |
| Internacionalización    | i18next + react-i18next                  |
| Herramienta de build    | Vite 8                                   |
| Pruebas                 | Vitest + Playwright + cargo-nextest      |
| Formato                 | dprint (TS/JSON/Markdown/TOML) + rustfmt |
| Linting                 | ESLint + Oxlint + Clippy + cargo-deny    |

### Arquitectura de backend: Patrón de inyección de dependencias Harness

El backend utiliza una arquitectura de workspace Rust con **32 crates**, siguiendo el **Patrón de arquitectura Harness**:

```
Todos los crates están desacoplados a través de interfaces trait definidas en axagent-harness.
La capa de ejecución (axagent-runtime) ensambla e inyecta dependencias en tiempo de ejecución.

Dirección de dependencia: Implementaciones concretas → harness ← Llamadores
```

**harness** es la piedra angular de la arquitectura — cero lógica de negocio, cero implementaciones concretas, contiene solo definiciones trait, DTO de datos puros, constantes y tipos de error unificados. Depende de él todos los demás crates y no depende de ningún otro crate axagent-*.

```
src-tauri/crates/
├── harness/          # Piedra angular de la arquitectura — interfaces trait, DTOs, tipos de error unificados, contratos DI
│                     #   200+ definiciones trait que cubren: Agent/Provider/Tool/RAG/Storage/
│                     #   MCP/Plugins/Security/Observability/Memory/Learning/Browser/Messaging
│
├── entities/         # Modelos de entidad SeaORM
├── dao/              # Capa de acceso a datos (CRUD)
├── migration/        # Migraciones de base de datos
│
├── crypto/           # Cifrado/descifrado AES-256-GCM y gestión de claves
├── credential/       # Almacenamiento seguro de credenciales (claves API, etc.)
├── storage/          # Abstracción de almacenamiento de archivos (Local / S3 / WebDAV), soporte de lectura/escritura ZIP
├── cache/            # Capa de caché genérica (en memoria)
├── disk-cache/       # Caché de archivos basada en disco
├── search/           # Motor de búsqueda (FTS5 + sqlite-vec + embeddings candle)
├── document-parser/  # Extracción de texto de documentos (PDF/DOCX/XLSX/PPTX)
├── kit/              # Kit de utilidades — ayudantes de ruta/codificación/hash/fecha
│
├── runtime-core/     # Tipos comunes de ejecución, constantes de configuración
├── runtime/          # Orquestación de servicios de ejecución — ensambla los 30+ crates, el contenedor DI de ejecución
│                     #   Gestiona: sesiones/terminales/webhooks/límites de velocidad/permisos/SSRF/bus de eventos/estado
├── rt-workflow/      # Motor de flujos de trabajo — orquestación DAG, ejecutores de nodos, serialización YAML
├── rt-messaging/     # Pasarela de plataformas de mensajería — DingTalk/Feishu/QQ/Slack/WeChat/WhatsApp/Telegram/Discord
├── rt-webhook/       # Servidor webhook genérico y despacho de eventos
├── rt-dashboard/     # Framework de plugins de panel
├── rt-theme/         # Motor de temas — lógica de conmutación dark/light
│
├── agent/            # Núcleo de agente de IA — 80+ módulos
│                     #   Motor ReAct/planificación jerárquica/investigación en profundidad/verificación de hechos/árbol de pensamientos/
│                     #   reflexión/autoverificación/recuperación de errores/optimización RL/ajuste fino LoRA/
│                     #   evaluación/recomendación de herramientas/pruebas A-B/coordinador/pizarra/visor de visión/
│                     #   búsqueda web/búsqueda académica/compilación de wiki y más
│
├── orchestrator/     # Orquestación de agentes — programación multi-agente, descomposición DAG, ejecución de subgrafos dinámicos
├── providers/        # Adaptadores de proveedores de modelos — OpenAI/Anthropic/Gemini/Ollama/
│                     #   OpenClaw/Hermes/Generación de imágenes (DALL-E/Flux)/Realtime/Responses
├── tools/            # Sistema de herramientas — trait Tool/registro/orquestación/streaming/sandbox/47+ herramientas integradas
├── gateway/          # Pasarela de API — servidor HTTP/WS axum, OAuth, límite de velocidad, Prometheus
├── mcp/              # Protocolo MCP — stdio + Streamable HTTP, basado en rmcp
├── trajectory/       # Sistema de aprendizaje — memoria/evolución de habilidades/perfil de usuario/integración dream
├── plugins/          # Sistema de complementos — compatible con OpenClaw, instalación de paquetes npm, mercado
├── telemetry/        # Observabilidad — OpenTelemetry, registro estructurado, métricas de ejecución
├── prompt-guard/     # Protección contra inyección de prompts — canal de detección multinivel L1-L4
├── npm/              # Cliente de registro npm
└── schema-gen/       # Herramienta de generación de esquemas de base de datos
```

### Arquitectura frontend

```
src/
├── pages/            # 22 páginas
│   ├── ChatPage          # Interfaz de chat principal
│   ├── WorkflowPage      # Editor de flujos de trabajo
│   ├── GatewayPage       # Gestión de pasarela de API
│   ├── KnowledgeHubPage  # Gestión de base de conocimiento
│   ├── MemoryPage        # Gestión de memoria
│   ├── SkillsPage        # Mercado de habilidades
│   ├── SettingsPage      # Panel de configuración
│   ├── DashboardPage     # Panel de datos
│   ├── TerminalPage      # Terminal
│   ├── FilesPage         # Gestión de archivos
│   ├── GatewayLinkPage   # Gestión de enlaces externos
│   ├── LinkPage          # Enlaces de integración
│   ├── WikiEditorPage    # Editor de Wiki
│   ├── WikiEditPage      # Edición de Wiki
│   ├── WikiGraphPage     # Grafo de conocimiento Wiki
│   ├── FineTunePage      # Ajuste fino LoRA
│   ├── PersonaPage       # Gestión de personajes
│   ├── QuickBarPage      # Barra rápida
│   ├── IngestPage        # Ingesta de documentos
│   ├── WorkflowMarketplace # Mercado de flujos de trabajo
│   ├── DynamicUIManagerPage # Gestión de UI dinámica
│   └── DynamicPageViewer    # Visor de páginas dinámicas
│
├── components/       # 24 módulos, 200+ componentes
│   ├── chat/         # UI de chat (flujo de mensajes/entrada/adjuntos/llamadas a herramientas/artefactos/bloques de pensamiento)
│   ├── workflow/     # Editor de flujos de trabajo (nodos/aristas/paneles/plantillas/asistencia IA)
│   ├── gateway/      # UI de gestión de pasarela de API
│   ├── settings/     # Panel de configuración (40+ subcomponentes)
│   ├── skill/        # Editor y renderizador de habilidades
│   ├── benchmark/    # Panel de benchmark
│   ├── decomposition/# Descomposición de habilidades y generación de herramientas
│   ├── devtools/     # Línea de tiempo Trace/Span
│   ├── layout/       # Diseño (barra de título/barra lateral/paleta de comandos)
│   └── ...
│
├── stores/           # 62 stores Zustand
│   ├── domain/       # Estado central de negocio
│   ├── feature/      # Estado de módulos de características (44)
│   └── devtools/     # Estado de DevTools
│
├── hooks/            # React Hooks
├── lib/              # Funciones de utilidad + Web Workers
├── types/            # Definiciones de tipos TypeScript
├── sdk/              # SDK de integración externa
└── i18n/             # 11 traducciones de idiomas (zh-CN/zh-TW/en-US/ja/ko/fr/de/es/ru/hi/ar)
```

---

## Directorio de datos

```
~/.axagent/                    # Configuración de la aplicación
├── axagent.db                 # Base de datos SQLite principal (SeaORM)
├── master.key                 # Clave maestra AES-256
├── vector_db/                 # Índice vectorial sqlite-vec
└── ssl/                       # Certificados SSL autofirmados

~/Documents/axagent/          # Archivos de usuario
├── images/                   # Adjuntos de imágenes
├── files/                    # Adjuntos de archivos
└── backups/                  # Copias de seguridad automáticas
```

---

## Inicio rápido

### Requisitos

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+, edición 2024
- [npm](https://www.npmjs.com/) 10+
- Windows: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (MSVC + Windows SDK)
- macOS: Xcode Command Line Tools
- Linux: `build-essential` + `libwebkit2gtk-4.1-dev` + `libssl-dev`

### Compilación (Build)

```bash
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent
npm install
npm run tauri dev      # Modo desarrollo
npm run tauri build    # Compilación de producción
```

Los artefactos de compilación se encuentran en `src-tauri/target/release/`.

### Pruebas

```bash
npm run test           # Pruebas unitarias frontend (Vitest watch)
npm run test:run       # Pruebas unitarias frontend (ejecución única)
npm run test:e2e       # Pruebas E2E (Playwright)

# Pruebas de backend Rust
cd src-tauri && cargo nextest run
cd src-tauri && cargo test

# Comprobación de tipos y Linting
npm run typecheck
cd src-tauri && cargo clippy -- -D warnings
npm run format

# Verificación completa de CI
npm run ci:check
```

---

## Soporte de plataformas

| Plataforma | Arquitectura                              |
| ---------- | ----------------------------------------- |
| Windows    | x86_64, ARM64                             |
| macOS      | Apple Silicon (arm64), Intel (x86_64)     |
| Linux      | x86_64, ARM64                             |
| Android    | arm64-v8a, armeabi-v7a, x86_64 (emulador) |
| iOS        | arm64                                     |

---

## Licencia

Este proyecto se publica como código abierto bajo la licencia [AGPL-3.0-only](LICENSE).

---

## Agradecimientos

AxAgent se construye sobre muchos proyectos de código abierto excelentes, entre ellos:

- [Tauri](https://tauri.app/) — Framework de escritorio multiplataforma
- [React](https://react.dev/) + [Ant Design](https://ant.design/) — UI frontend
- [SeaORM](https://www.sea-ql.org/SeaORM/) — ORM de Rust
- [sqlite-vec](https://github.com/asg017/sqlite-vec) — Búsqueda de vectores
- [candle](https://github.com/huggingface/candle) — Inferencia de embeddings local
- [rmcp](https://github.com/nicholasxjy/rmcp) — SDK Rust MCP
- [ReactFlow](https://reactflow.dev/) — Editor de flujos de trabajo visual
- [axum](https://github.com/tokio-rs/axum) — Framework HTTP
- [Monaco Editor](https://microsoft.github.io/monaco-editor/) — Editor de código
- [xterm.js](https://xtermjs.org/) — Emulador de terminal
