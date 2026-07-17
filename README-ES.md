# AxAgent

[![Release](https://img.shields.io/github/v/release/polite0803/AxAgent?style=flat-square)](https://github.com/polite0803/AxAgent/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/polite0803/AxAgent/release.yml?style=flat-square)](https://github.com/polite0803/AxAgent/actions)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux%20%7C%20Android%20%7C%20iOS-blue?style=flat-square)
![License](https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square)

<p align="center">
  <a href="./media/poster-axagent.svg">
    <img src="./media/poster-axagent.svg" alt="Póster de AxAgent" width="80%" />
  </a>
</p>

**AxAgent** es un cliente de escritorio de asistente IA multiplataforma basado en Tauri 2 (Windows / macOS / Linux / Android / iOS). Integra un motor de agente ReAct, orquestación visual de workflows, bases de conocimiento RAG locales, extensiones de protocolo MCP, una pasarela multi-modelo unificada, automatización de navegador y control de computadora — sirviendo como una estación de trabajo IA para desarrollo diario, investigación, gestión de conocimiento y automatización.

> **Idiomas**: [简体中文](./README.md) | [English](./README-EN.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

---

## Posicionamiento del Proyecto

AxAgent resuelve tres problemas fundamentales:

1. **Acceso Multi-Modelo Unificado y Enrutamiento Inteligente** — Usa OpenAI, Anthropic Claude, Google Gemini, modelos locales Ollama y cualquier API compatible con OpenAI en una sola interfaz, con rotación automática multi-clave por cuota, enrutamiento inteligente por tipo de tarea y comparación en streaming
2. **Ciclo Cerrado de Conversación a Ejecución con IA** — 47+ herramientas integradas + workflows visuales + extensiones MCP + navegador/control de computadora, la IA puede manipular archivos, ejecutar código, gestionar Git y programar tareas
3. **Soberanía de Datos Local-First** — Las conversaciones, bases de conocimiento, memoria y configuración se almacenan en una base de datos SQLite local, las claves API se cifran con AES-256-GCM. Las funcionalidades principales funcionan sin servicios cloud de terceros

---

## Capacidades Principales

### Motor Multi-Modelo

- **9 Adaptadores de Proveedores**: OpenAI (Chat Completions + Responses + Realtime), Anthropic Claude, Google Gemini, Ollama (con gestión de modelos locales GGUF), OpenClaw, Hermes y todas las APIs compatibles con OpenAI
- **Rotación Multi-Clave**: Múltiples claves API por proveedor, rotación automática por cuota, failover automático en límite de clave única
- **Enrutamiento Inteligente**: Selección automática de modelo por tipo de tarea (revisión de código / resumen / traducción / general), reglas personalizables
- **Monitorización de Salud de Proveedores**: Seguimiento en tiempo real de tasa de éxito, latencia y disponibilidad, con degradación automática por niveles
- **Generación de Imágenes IA**: DALL-E 3 y Flux (Replicate) con preajustes multi-tamaño
- **Voz en Tiempo Real**: Conversación de voz WebSocket basada en la API Realtime de OpenAI, con soporte de interrupción y transcripción en streaming

### Sistema de Agentes (Motor ReAct)

- **Planificador Jerárquico** (`hierarchical_planner`): Descompone tareas complejas en planes estructurados Fase → Tarea, compilados en ejecución topológica DAG
- **Investigación Profunda** (`deep_research`): Orquestación de búsqueda multi-fuente incluyendo planificación, ejecución, síntesis de contenido y seguimiento de citas
- **Verificador de Hechos** (`fact_checker`): Verificación de hechos impulsada por IA con clasificador de fuentes y evaluación de credibilidad
- **Árbol de Pensamientos** (`tree_of_thoughts`): Exploración de razonamiento multi-ruta con evaluación de ramas y backtracking
- **Reflector** (`reflector`): Autoevaluación post-ejecución y sugerencias de mejora
- **Auto-Verificador** (`self_verifier`): Validación automática de resultados de razonamiento con detección de ciclos
- **Recuperación de Errores** (`error_recovery_engine`): Clasificación de tipo de error → selección de estrategia → reintento automático o ajuste de plan, con backoff exponencial
- **Pruebas A/B** (`ab_testing`): Evaluación comparativa de diferentes estrategias de razonamiento
- **Sistema de Evaluación** (`evaluator`): Framework de benchmarks integrado
- **Fine-Tuning LoRA** (`fine_tune`): Pipeline de entrenamiento integrado con gestión de adaptadores LoRA
- **Optimizador RL** (`rl_optimizer`): Aprendizaje por refuerzo basado en feedback de experiencia

**Colaboración Multi-Agente**:

- Arquitectura de coordinación maestro-esclavo con ejecución paralela de sub-agentes y planificación sensible a dependencias
- Pizarra compartida para intercambio de información entre agentes
- Modo de debate adversarial (rondas Pro/Contra con puntuación de fuerza de argumentos)
- Modo Swarm para clústeres de agentes multi-proceso
- Modo proactivo: los agentes pueden iniciar sugerencias y operaciones

**Control de Computadora**: Clics de ratón, entrada de teclado, desplazamiento de pantalla impulsados por IA, con tres niveles de permisos (predeterminado / aceptar ediciones / acceso completo) y aislamiento de rutas en sandbox

**Automatización de Navegador**: Control de navegador mediante protocolo CDP, con navegación, capturas de pantalla, clics, relleno de formularios y extracción de texto

### Sistema de Habilidades

- **Mercado de Habilidades**: Explorar e instalar habilidades de la comunidad
- **Creación Asistida por IA**: Creación automática de estructuras de habilidades desde propuestas en lenguaje natural (`skill:create`)
- **Evolución de Habilidades** (`evolution_engine`): Análisis y mejora automática de habilidades basados en feedback de ejecución
- **Coincidencia Semántica**: Recomendación semántica de habilidades según contexto
- **Descomposición de Habilidades** (`skill_decomposition`): Descomposición automática de tareas complejas en combinaciones atómicas de habilidades
- **Herramientas Generadas**: Nuevas herramientas generadas y registradas por IA
- **Ejecución en Sandbox**: Las habilidades se ejecutan en entornos sandbox aislados

### Workflow Visual

Editor de workflow DAG por arrastrar y soltar basado en ReactFlow 12:

- **17 Tipos de Nodos**: Disparador, Agente, Llamada LLM, Rama Condicional, Bifurcación Paralela, Bucle, Fusión, Retardo, Llamada de Herramienta, Ejecución de Código, Sub-Workflow, Búsqueda Vectorial, Análisis de Documento, Validación, Fin, Regla de Negocio, Rol de Agente
- **Ejecución por Orden Topológico de Kahn**: Detección automática de ciclos, planificación paralela en pipeline
- **Plantillas Integradas**: Revisión de código, corrección de bugs, documentación, pruebas, refactorización, exploración, análisis de rendimiento, auditoría de seguridad, desarrollo de funcionalidades
- **Serialización YAML**: Importación/exportación de definiciones de workflow
- **Gestión de Versiones**: Control de versiones de plantillas
- **Diseño Asistido por IA**: Diseño de workflow y recomendación de nodos asistidos por IA

### Gestión del Conocimiento

- **RAG Multi-Base de Conocimiento**: Carga de documentos → análisis automático (PDF/DOCX/XLSX/PPTX/TXT) → segmentación → indexación vectorial
- **Búsqueda Híbrida**: Similitud vectorial (sqlite-vec + embeddings locales candle) + búsqueda de texto completo BM25 (FTS5), ranking híbrido
- **Self-RAG**: Reflexión y validación automática de resultados de búsqueda
- **Re-Ranking**: Reordenación de resultados por cross-encoder
- **Grafo de Conocimiento**: Extracción de entidades → construcción de relaciones → grafo visual
- **Monitorización de Archivos**: Monitorización en tiempo real de cambios mediante `notify`, indexación incremental automática
- **LLM Wiki**: Compilador y validador Wiki asistido por IA

### Sistema de Memoria

- **Memoria Multi-Espacio de Nombres**: Aislamiento por proyecto/tema, entrada manual y extracción automática por IA
- **Integración Persistente**: Memoria de ciclo cerrado Honcho y Mem0
- **Perfil de Usuario**: Aprendizaje automático de estilo de código, preferencias de stack tecnológico y estilo de comunicación
- **Transferencia de Estilo**: Extracción de características de estilo de código → aplicación al código generado por IA
- **Integración Dream**: Consolidación automática en segundo plano de fragmentos de memoria y patrones de comportamiento en conocimiento estructurado
- **Memoria de Proyecto**: Persistencia de contexto por proyecto

### Pasarela API

Pasarela HTTP + WebSocket integrada basada en `axum`:

- **Endpoints Compatibles**: OpenAI `/v1/chat/completions`, API Claude Messages, API Gemini, además de OpenAI Responses y Realtime WebSocket
- **Gestión de Claves**: Generación, revocación, activación/desactivación de claves de acceso con soporte de caducidad
- **Seguimiento de Uso**: Estadísticas de solicitudes y consumo de tokens por clave/proveedor/fecha, exportación de métricas Prometheus
- **Limitación de Velocidad**: Algoritmo de cubo de tokens mediante `governor`
- **SSL/TLS**: Certificados autofirmados integrados (`rcgen`), soporte de certificados personalizados
- **Vinculación Externa**: Integración en un clic con Claude CLI, OpenCode y otras herramientas externas, sincronización automática de claves API
- **Tickets en Tiempo Real**: Tickets de autenticación temporal basados en HMAC para transferencia segura de conexiones WebSocket

### Integración de Plataformas de Mensajería

Pasarela multi-plataforma mediante `rt-messaging`, con recepción de mensajes, análisis de comandos y respuesta automática con IA para **DingTalk, Feishu, QQ, Slack, WeChat, WhatsApp, Telegram y Discord**.

### Sistema de Herramientas

47+ herramientas integradas, registradas uniformemente mediante el trait `Tool`:

| Categoría               | Herramientas                                                                                                                                                                                               |
| ----------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Operaciones de Archivos | `file_read`, `file_write`, `file_edit`, `file_system`                                                                                                                                                      |
| Ejecución de Código     | `bash`, `repl`                                                                                                                                                                                             |
| Búsqueda                | `grep`, `glob`                                                                                                                                                                                             |
| Navegador               | `browser` (CDP)                                                                                                                                                                                            |
| Control de Computadora  | `computer_use` (ratón/teclado/captura)                                                                                                                                                                     |
| Web                     | `web_search`, `web_fetch`                                                                                                                                                                                  |
| Base de Conocimiento    | `knowledge`, `document`                                                                                                                                                                                    |
| Git                     | `git` (commit/push/branch/diff)                                                                                                                                                                            |
| Herramientas Dev        | `lsp`, `workspace`                                                                                                                                                                                         |
| Gestión de Tareas       | `plan`, `task_system`, `todo_write`, `cron`                                                                                                                                                                |
| Mensajería              | `push_notification`, `messaging`                                                                                                                                                                           |
| Base de Datos           | `database`                                                                                                                                                                                                 |
| Almacenamiento          | `storage`                                                                                                                                                                                                  |
| Otros                   | `agent`, `agent_memory`, `context`, `export`, `integration`, `media`, `media_delivery`, `migration_tool`, `monitor`, `obsidian`, `ocr`, `personality`, `shared_path`, `system_info`, `testing`, `worktree` |

### Protocolo MCP

Implementación completa del protocolo MCP (Model Context Protocol) basada en `rmcp`:

- **Transporte**: subproceso stdio + HTTP Streamable + WebSocket
- **Autenticación OAuth**: Flujo de autorización OAuth para servidores MCP
- **Descubrimiento de Herramientas**: Descubrimiento y registro automático de herramientas expuestas por servidores MCP
- **Gestor MCP**: Gestión del ciclo de vida del servidor, verificaciones de salud, reconexión automática

### Sistema de Plugins

Arquitectura de plugins de tres niveles compatible con OpenClaw (integrado / empaquetado / externo):

- Instalación de paquetes npm con UI de mercado para búsqueda e instalación
- Definición de manifiesto de plugin, declaración de permisos, ejecución en sandbox
- Registro de herramientas personalizadas, proveedores de agentes, intercepción de Hooks
- Instalador de habilidades: instalación de habilidades desde paquetes de plugins

### Seguridad

- **Cifrado AES-256-GCM**: Almacenamiento local cifrado de claves API y configuración sensible (crate `crypto`)
- **Protección contra Inyección de Prompts**: Pipeline de defensa de cuatro niveles (`prompt-guard`) — detección de patrones → escape de delimitadores → envoltura XML → etiquetas de confianza, integrado en toda la cadena de conversación/construcción de prompts/Git/RAG
- **Protección SSRF**: Verificación de seguridad de URL para bloquear solicitudes a direcciones de red interna
- **Filtrado de Contenido**: Filtrado de seguridad multi-tipo
- **Limitación de Velocidad**: Limitación por cubo de tokens para llamadas de herramientas y solicitudes API
- **Disyuntor**: Interrupción automática en fallos consecutivos
- **Control de Acceso**: Control de permisos de acceso a herramientas basado en políticas
- **Aislamiento Sandbox**: Aislamiento del entorno de ejecución para agentes y habilidades

### Herramientas de Desarrollo

- **Trazado Distribuido** (`telemetry`): Integración OpenTelemetry con visualización Span/Trace
- **Logging Estructurado**: tracing-subscriber + marcas de tiempo chrono
- **Depuración por Reproducción**: Grabación de trayectoria de ejecución de agentes (`trajectory_recorder`) y reproducción
- **Panel DevTools**: Visor de línea de tiempo Trace Explorer, Benchmark Runner, Tool Recommender
- **Benchmarks**: Benchmarks Criterion (tool_exec / llm_call / search)
- **Verificaciones CI**: `npm run ci:check` integrando verificación de tipos, linting y validación de formato

### Experiencia de Escritorio y Móvil

- **Diseño Responsive**: Adaptación por puntos de ruptura CSS para escritorio/tableta/móvil (3 niveles: `desktop` / `tablet` / `mobile`)
- **11 Idiomas**: Chino simplificado, Chino tradicional, Inglés, Japonés, Coreano, Francés, Alemán, Español, Ruso, Hindi, Árabe
- **Motor de Temas** (`rt-theme`): Temas oscuro/claro + múltiples preajustes (incluyendo tema monoespacio 21th), personalización profunda con Ant Design 6
- **Editor Monaco**: Resaltado de sintaxis, vista previa de diferencias, soporte multi-idioma
- **Terminal xterm.js**: WebLinks, Unicode 11, búsqueda
- **Desplazamiento Virtual**: @tanstack/react-virtual + react-virtuoso
- **Renderizado de Gráficos**: D2 + Mermaid + Recharts
- **Menú de Copia Global**: Menú de copia personalizado, supresión del menú contextual nativo
- **Paleta de Comandos**: Paleta de comandos global Ctrl+K
- **Bandeja del Sistema + Atajos Globales + Inicio Automático**: Funcionamiento en segundo plano no intrusivo
- **Actualización Automática**: Verificación de versiones en GitHub Releases a intervalos configurables
- **Soporte Proxy**: Configuración de proxy HTTP / SOCKS5
- **Espacio de Trabajo en la Nube**: Sincronización de almacenamiento S3 y WebDAV con detección de conflictos y sincronización bidireccional

### Móvil

- Android APK/AAB (arm64-v8a, armeabi-v7a, x86_64)
- iOS IPA (arm64)
- Adaptaciones específicas para móvil: insets de área segura, navegación inferior, navegación por cajón

---

## Arquitectura Técnica

### Stack Tecnológico

| Capa                    | Tecnología                               | Versión |
| ----------------------- | ---------------------------------------- | ------- |
| Framework de Escritorio | Tauri                                    | 2.11    |
| Framework Frontend      | React                                    | 19      |
| Sistema de Tipos        | TypeScript                               | 7       |
| Biblioteca UI           | Ant Design                               | 6       |
| Framework CSS           | TailwindCSS                              | 4       |
| Gestión de Estado       | Zustand                                  | 5       |
| Enrutamiento            | React Router                             | 7       |
| Editor de Código        | Monaco Editor                            | 0.55    |
| Terminal                | xterm.js                                 | 6       |
| Editor de Workflow      | ReactFlow                                | 12      |
| Gráficos                | D2 + Mermaid + Recharts                  |         |
| Animación               | Framer Motion                            | 12      |
| Desplazamiento Virtual  | @tanstack/react-virtual + react-virtuoso |         |
| Arrastrar y Soltar      | @dnd-kit                                 | 6       |
| Renderizado Markdown    | markstream-react + stream-markdown       |         |
| i18n                    | i18next + react-i18next                  |         |
| Herramienta de Build    | Vite                                     | 8       |
| Pruebas                 | Vitest + Playwright                      |         |
| Formateo                | dprint (TS/JSON/Markdown/TOML) + rustfmt |         |
| Linting                 | ESLint + Oxlint + Clippy                 |         |

### Arquitectura Backend: Inyección de Dependencias Harness

Arquitectura de workspace Rust con **32 crates**, siguiendo el patrón **Harness DI**:

> Todos los crates están desacoplados mediante las interfaces trait definidas por axagent-harness, y axagent-runtime ensambla e inyecta dependencias en tiempo de ejecución.
> Dirección de dependencias: `implementaciones concretas → harness ← llamadores`

**harness** es la piedra angular arquitectónica — cero lógica de negocio, cero implementaciones concretas, conteniendo solo definiciones de traits, DTOs de datos puros, constantes y tipos de error unificados. Es dependido por todos los demás crates y no depende de ningún crate axagent-* (200+ definiciones de traits que cubren Agent/Provider/Tool/RAG/Almacenamiento/MCP/Plugins/Seguridad/Observabilidad/Memoria/Aprendizaje/Navegador/Mensajería, etc.).

```
src-tauri/crates/
├── harness/          # Piedra angular arquitectónica — interfaces trait, DTOs, tipos de error, contratos DI
├── entities/         # Modelos de entidad SeaORM
├── dao/              # Capa de acceso a datos (CRUD)
├── migration/        # Migraciones de base de datos
├── crypto/           # Cifrado/descifrado AES-256-GCM y gestión de claves
├── credential/       # Almacenamiento seguro de credenciales
├── storage/          # Abstracción de almacenamiento de archivos (local/S3/WebDAV), lectura/escritura ZIP
├── cache/            # Capa de caché en memoria
├── disk-cache/       # Caché de archivos en disco
├── search/           # Motor de búsqueda (FTS5 + sqlite-vec + embeddings locales candle)
├── document-parser/  # Extracción de texto de documentos (PDF/DOCX/XLSX/PPTX)
├── kit/              # Utilidades generales (rutas/codificación/hash/fechas)
├── runtime-core/     # Tipos comunes de tiempo de ejecución, constantes de configuración
├── runtime/          # Orquestación de servicios en tiempo de ejecución — contenedor DI que ensambla los 30+ crates
├── rt-workflow/      # Motor de workflow — orquestación DAG, ejecutores de nodos, serialización YAML
├── rt-messaging/     # Pasarela de plataformas de mensajería — DingTalk/Feishu/QQ/Slack/WeChat/WhatsApp/Telegram/Discord
├── rt-webhook/       # Servidor webhook genérico
├── rt-dashboard/     # Framework de plugins de panel
├── rt-theme/         # Motor de temas
├── agent/            # Núcleo de agente IA — 80+ módulos
│                     #   MotorReAct/PlanificaciónJerárquica/InvestigaciónProfunda/VerificaciónHechos/ÁrbolPensamientos/
│                     #   Reflexión/AutoVerificación/RecuperaciónErrores/OptimizaciónRL/FineTuningLoRA/
│                     #   Evaluación/RecomendaciónHerramientas/PruebasAB/Coordinador/Pizarra/PipelineVisión/
│                     #   BúsquedaWeb/BúsquedaAcadémica/CompilaciónWiki, etc.
├── orchestrator/     # Orquestación de agentes — planificación multi-agente, descomposición DAG, ejecución dinámica de subgrafos
├── providers/        # Adaptadores de proveedores de modelos
├── tools/            # Sistema de herramientas — trait Tool/registro/orquestación/streaming/sandbox/47+ herramientas integradas
├── gateway/          # Pasarela API — servidor HTTP/WS axum, OAuth, limitación de velocidad, Prometheus
├── mcp/              # Protocolo MCP — stdio + HTTP Streamable, basado en rmcp
├── trajectory/       # Sistema de aprendizaje — memoria/evolución de habilidades/perfiles de usuario/integración dream
├── plugins/          # Sistema de plugins — compatible con OpenClaw, instalación de paquetes npm, mercado
├── telemetry/        # Observabilidad — OpenTelemetry, logging estructurado, métricas de tiempo de ejecución
├── prompt-guard/     # Protección contra inyección de prompts — pipeline de detección multinivel L1-L4
├── npm/              # Cliente de registro npm
└── schema-gen/       # Herramienta de generación de esquema de base de datos
```

### Arquitectura Frontend

```
src/
├── pages/            # Páginas (23+ incluyendo subpáginas)
│   ├── ChatPage           # Interfaz de chat — barra lateral/flujo de mensajes/panel Agent/multi-pestaña
│   ├── DashboardPage      # Panel — estadísticas de uso/distribución de modelos/gráficos de tendencia
│   ├── WorkflowPage       # Editor de workflow — visualización DAG ReactFlow
│   ├── KnowledgeHubPage   # Gestión de base de conocimiento — carga/indexación/búsqueda
│   ├── MemoryPage         # Gestión de memoria
│   ├── SkillsPage         # Mercado de habilidades
│   ├── SettingsPage       # Panel de configuración — 40+ elementos de configuración
│   ├── TerminalPage       # Terminal integrado — xterm.js
│   ├── FilesPage          # Gestión de archivos
│   ├── GatewayLinkPage    # Pasarela API y gestión de enlaces externos
│   ├── QuickBarPage       # Barra rápida (ventana independiente)
│   ├── DynamicUIManagerPage / DynamicPageViewer  # Motor UI dinámico
│   ├── WikiGraphPage / WikiEditPage / IngestPage # LLM Wiki
│   ├── LearningGraphPage  # Grafo de aprendizaje
│   ├── FineTunePage       # Fine-tuning LoRA
│   ├── PersonaPage        # Gestión de personas
│   ├── WorkflowMarketplace # Mercado de workflows
│   ├── DevTools/          # TraceExplorer / BenchmarkRunner / ToolRecommender
│   └── Workflow/          # WorkflowListPage
│
├── components/       # 28 módulos, 450+ componentes
│   ├── chat/         # Chat (flujo de mensajes/entrada/ChatView/TabBar/RightPanel/adjuntos/renderizado de llamadas de herramientas)
│   ├── layout/       # Layout — 17 componentes
│   │                 #   AppInitializer / Sidebar / ContentArea / TitleBar /
│   │                 #   CommandPalette / GlobalCopyMenu / GlobalErrorBoundary /
│   │                 #   GlobalStatusBar / ErrorNotificationToast / AppHeader /
│   │                 #   BackendStatusIndicator / IpcReconnectBanner /
│   │                 #   ModuleErrorBoundary / NotificationBell / UserProfileModal etc.
│   ├── agent/        # Panel Agent/entrada/mini-panel
│   ├── workflow/     # Editor de workflow (nodos/aristas/paneles/plantillas/asistencia IA)
│   ├── settings/     # Panel de configuración (40+ subcomponentes)
│   ├── skill/        # Editor de habilidades/renderizador/paneles flotantes
│   ├── dynamicUI/    # Registro de componentes UI dinámicos (26 componentes integrados)
│   ├── gateway/      # Gestión de pasarela API
│   ├── files/        # Gestión de archivos
│   ├── terminal/     # Componentes de terminal
│   ├── search/       # Interfaz de búsqueda
│   ├── benchmark/    # Panel de benchmarks
│   ├── decomposition/# Descomposición de habilidades y generación de herramientas
│   ├── devtools/     # Línea de tiempo Trace/Span + panel RL Training
│   ├── approval/     # UI de workflow de aprobación
│   ├── recommendation/ # Recomendación de herramientas/modelos
│   ├── onboarding/   # WelcomeWizard / InteractiveTutorial
│   ├── help/         # Panel de ayuda
│   ├── notification/ # Componentes de notificación
│   ├── proactive/    # Sugerencias proactivas
│   ├── llm-wiki/     # Componentes LLM Wiki
│   ├── wiki/         # Componentes Wiki
│   ├── fine-tune/    # UI de fine-tuning
│   ├── trace/        # Componentes Trace
│   ├── style/        # Estilo/tema
│   ├── shared/       # Componentes compartidos (ErrorBoundary / PageContextProvider)
│   └── common/       # Componentes comunes (Icon, etc.)
│
├── stores/           # Gestión de estado Zustand
│   ├── domain/       # 10 stores de negocio principales (conversación/flujo/compresión/preferencias/multi-modelo, etc.)
│   ├── feature/      # 48 stores de módulos funcionales (agente/workflow/conocimiento/habilidades/pasarela/memoria/terminal, etc.)
│   └── devtools/     # 4 stores de herramientas de desarrollo
│
├── hooks/            # React Hooks (atajos/paleta de comandos/responsive/barra de desplazamiento/tema/avatar, etc.)
├── lib/              # Biblioteca de utilidades (invoke/pageRegistry/shortcuts/skillLifecycle/
│                     #   chartGenerator/codeExecutor/tokenEstimator/workflowLayout etc. — 45+ módulos)
├── types/            # Definiciones de tipos TypeScript
├── theme/            # Motor de temas Shadcn
├── i18n/             # Archivos de traducción en 11 idiomas (zh-CN/zh-TW/en-US/ja/ko/fr/de/es/ru/hi/ar)
├── constants/        # Constantes y flags de funcionalidades
└── sdk/              # SDK de integración externa
```

### Flags de Funcionalidades

El proyecto gestiona el despliegue progresivo de funcionalidades mediante `featureFlags.ts`:

| Flag                | Estado | Descripción                                                      |
| ------------------- | ------ | ---------------------------------------------------------------- |
| `AGENT_IN_THE_LOOP` | ✅     | Panel Agent global + inyección de contexto de página             |
| `DYNAMIC_UI`        | ✅     | Motor de construcción UI dinámico                                |
| `SELF_EVOLUTION_UI` | ❌     | Panel de control de auto-evolución frontend                      |
| `NL_EXTENSION`      | ❌     | Extensiones de negocio dinámicas impulsadas por lenguaje natural |

### Plugins Tauri

| Plugin              | Propósito                             |
| ------------------- | ------------------------------------- |
| `autostart`         | Inicio automático al arrancar         |
| `clipboard-manager` | Lectura/escritura del portapapeles    |
| `dialog`            | Diálogos de selección de archivos     |
| `fs`                | Acceso al sistema de archivos         |
| `global-shortcut`   | Registro de atajos globales           |
| `notification`      | Notificaciones del sistema            |
| `opener`            | Apertura de enlaces/archivos externos |
| `process`           | Gestión de procesos                   |
| `updater`           | Actualización automática              |
| `mcp-bridge`        | Puente de protocolo MCP (no-Android)  |

---

## Directorio de Datos

```
~/.axagent/                    # Configuración de la aplicación
├── axagent.db                 # Base de datos principal SQLite (SeaORM)
├── master.key                 # Clave maestra AES-256
├── vector_db/                 # Índice vectorial sqlite-vec
└── ssl/                       # Certificados SSL autofirmados

~/Documents/axagent/          # Archivos de usuario
├── images/                   # Adjuntos de imagen
├── files/                    # Adjuntos de archivo
└── backups/                  # Copias de seguridad automáticas
```

---

## Inicio Rápido

### Requisitos Previos

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
npm run tauri dev      # Modo desarrollo (Vite HMR + ventana Tauri)
```

### Build

```bash
npm run tauri build    # Build de producción para escritorio

npm run tauri:android:build   # Build Android
npm run tauri:ios:build       # Build iOS
```

Los artefactos de build de escritorio se encuentran en `src-tauri/target/release/`.

### Pruebas

```bash
npm run test           # Pruebas unitarias frontend (Vitest watch)
npm run test:run       # Pruebas unitarias frontend (ejecución única)
npm run test:e2e       # Pruebas E2E (Playwright)

# Pruebas backend Rust
cd src-tauri && cargo nextest run
cd src-tauri && cargo test

# Verificación de tipos & Lint
npm run typecheck
cd src-tauri && cargo clippy -- -D warnings
npm run format         # Formateo dprint
npm run lint:eslint    # Verificación ESLint
npm run contracts      # Verificación de contrato API

# Verificación CI completa
npm run ci:check
```

### Scripts

| Comando                  | Propósito                             |
| ------------------------ | ------------------------------------- |
| `npm run bump`           | Actualización interactiva de versión  |
| `npm run docs`           | Generar documentación TypeDoc         |
| `npm run skill:create`   | Crear nuevo scaffold de habilidad     |
| `npm run skill:validate` | Validar definición de habilidad       |
| `npm run check:types`    | Verificación de consistencia de tipos |

---

## Soporte de Plataformas

| Plataforma | Arquitectura                          |
| ---------- | ------------------------------------- |
| Windows    | x86_64, ARM64                         |
| macOS      | Apple Silicon (arm64), Intel (x86_64) |
| Linux      | x86_64, ARM64                         |
| Android    | arm64-v8a, armeabi-v7a, x86_64        |
| iOS        | arm64                                 |

---

## Licencia

Este proyecto es de código abierto bajo la licencia [AGPL-3.0-only](LICENSE).

---

## Agradecimientos

AxAgent está construido sobre muchos proyectos de código abierto excepcionales:

- [Tauri](https://tauri.app/) — Framework de escritorio multiplataforma
- [React](https://react.dev/) + [Ant Design](https://ant.design/) — UI frontend
- [SeaORM](https://www.sea-ql.org/SeaORM/) — ORM Rust
- [sqlite-vec](https://github.com/asg017/sqlite-vec) — Búsqueda vectorial
- [candle](https://github.com/huggingface/candle) — Inferencia de embeddings locales
- [rmcp](https://github.com/nicholasxjy/rmcp) — SDK MCP para Rust
- [ReactFlow](https://reactflow.dev/) — Editor visual de workflows
- [axum](https://github.com/tokio-rs/axum) — Framework HTTP
- [Monaco Editor](https://microsoft.github.io/monaco-editor/) — Editor de código
- [xterm.js](https://xtermjs.org/) — Emulador de terminal
- [Zustand](https://zustand.docs.pmnd.rs/) — Gestión de estado
- [Framer Motion](https://www.framer.com/motion/) — Biblioteca de animación
- [Recharts](https://recharts.org/) — Biblioteca de gráficos
