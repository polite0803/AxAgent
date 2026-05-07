[English](./README-EN.md) | [简体中文](./README.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | **Español** | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

[![AxAgent](https://github.com/polite0803/AxAgent/blob/main/src/assets/image/logo.png?raw=true)](https://github.com/polite0803/AxAgent)

<p align="center">
  <a href="https://www.producthunt.com/products/axagent?embed=true&amp;amp;utm_source=badge-featured&amp;amp;&amp;#10;&amp;amp;amp;;utm_medium=badge&amp;amp;#10&amp;amp;;utm_campaign=badge-axagent" target="_blank" rel="noopener noreferrer"><img alt="AxAgent - Lightweight, high-perf cross-platform AI desktop client | Product Hunt" width="250" height="54" src="https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=1118403&amp;theme=light&amp;t=1775627359538"></a>
</p>

<p align="center">
  <strong>Cliente desktop IA multiplataforma | Colaboración multi-agente | Local primero</strong>
</p>

<p align="center">
  <a href="https://github.com/polite0803/AxAgent/releases" target="_blank">
    <img src="https://img.shields.io/github/v/release/polite0803/AxAgent?style=flat-square" alt="Release">
  </a>
  <a href="https://github.com/polite0803/AxAgent/actions" target="_blank">
    <img src="https://img.shields.io/github/actions/workflow/status/polite0803/AxAgent/release.yml?style=flat-square" alt="Build">
  </a>
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-blue?style=flat-square" alt="Platform">
  <img src="https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square" alt="License">
</p>

---

## ¿Qué es AxAgent?

AxAgent es una aplicación desktop IA multiplataforma completa, que integra capacidades avanzadas de agentes IA y herramientas de desarrollo ricas. Soporta múltiples proveedores de modelos, ejecución autónoma de pipelines, orquestación visual de flujos de trabajo, gestión local de conocimientos y una pasarela API integrada.

---

## Capturas de pantalla

| Conversación y selección de modelo | Panel multi-agente |
|:---:|:---:|
| ![](.github/images/s1-0412.png) | ![](.github/images/s5-0412.png) |

| Base de conocimientos RAG | Memoria y contexto |
|:---:|:---:|
| ![](.github/images/s3-0412.png) | ![](.github/images/s4-0412.png) |

| Editor de flujo de trabajo | Pasarela API |
|:---:|:---:|
| ![](.github/images/s9-0412.png) | ![](.github/images/s10-0412.png) |

---

## Funcionalidades principales

### 🤖 Soporte de modelos IA

- **Soporte multi-proveedor** — Integración nativa de OpenAI, Anthropic Claude, Google Gemini, Ollama, OpenClaw, Hermes y todas las APIs compatibles con OpenAI
- **Rotación multi-clave** — Configure múltiples claves API por proveedor con rotación automática para distribuir la presión de límites de tasa
- **Soporte de modelos locales** — Soporte completo para modelos locales Ollama, incluyendo gestión de archivos GGUF/GGML
- **Gestión de modelos** — Obtención de listas de modelos remotos, personalización de parámetros (temperatura, tokens máximos, top-p, etc.)
- **Salida en streaming** — Renderizado en tiempo real token a token con bloques de pensamiento plegables (pensamiento extendido de Claude)
- **Comparación multi-modelo** — Pregunte a múltiples modelos simultáneamente con comparación lado a lado
- **Llamadas de funciones** — Llamadas de funciones estructuradas en todos los proveedores soportados
- **API Responses de OpenAI** — Soporte para transporte en formato OpenAI Responses
- **API Realtime** — Push de eventos WebSocket compatible con la API Realtime de OpenAI

### 🔐 Sistema de agentes IA

El sistema de agentes está construido sobre una arquitectura sofisticada con las siguientes características:

- **Motor de razonamiento ReAct** — Fusión de razonamiento y acción, con autoverificación integrada para ejecución fiable de tareas
- **Planificador jerárquico** — Descomposición de tareas complejas en planes estructurados con fases y dependencias
- **Descomponedor de tareas** — Descomposición automática de tareas complejas en subtareas ejecutables
- **Investigación profunda** — Orquestación de búsqueda multi-fuente, seguimiento de citas y evaluación de credibilidad
- **Verificación de hechos** — Verificación de hechos impulsada por IA y clasificación de fuentes
- **Orquestación de búsqueda** — Coordinación de múltiples proveedores de búsqueda, con planificación y síntesis de resultados
- **Búsqueda académica** — Búsqueda de literatura académica y análisis de citas
- **Control informático** — Clics de ratón, entrada de teclado, desplazamiento de pantalla controlados por IA, con análisis de modelo visual
- **Percepción de pantalla** — Captura de pantalla y análisis por modelo visual para identificación de elementos UI
- **Tres niveles de permisos** — Predeterminado (aprobación requerida), Aceptar ediciones (aprobación automática), Acceso completo (sin indicaciones)
- **Aislamiento sandbox** — Las operaciones del agente están estrictamente limitadas al directorio de trabajo especificado
- **Panel de aprobación de herramientas** — Visualización en tiempo real de solicitudes de llamadas a herramientas con aprobación individual
- **Seguimiento de costos** — Visualización en tiempo real del uso de tokens y estadísticas de costos por sesión
- **Pausa/Reanudación** — Suspenda la ejecución del agente en cualquier momento y reanude más tarde
- **Sistema de puntos de control** — Puntos de control persistentes para recuperación tras fallos y reconexión de sesiones
- **Motor de recuperación de errores** — Clasificación automática de errores, análisis de causas raíz y ejecución de estrategias de recuperación
- **Detección de bucles** — Detección e interrupción automáticas de comportamientos de bucle en el razonamiento del agente
- **Cadena de pensamiento** — Visualización del razonamiento decisional del agente, descomposición paso a paso
- **Modo proactivo** — El agente puede ofrecer sugerencias y ejecutar acciones proactivamente
- **Gestión de propósitos** — Mantenimiento y seguimiento de los propósitos de ejecución y contexto del agente

### 👥 Colaboración multi-agente

- **Coordinación de sub-agentes** — Arquitectura maestro-esclavo con soporte para múltiples agentes colaborativos
- **Ejecución paralela** — Procesamiento paralelo por múltiples agentes con planificación consciente de dependencias
- **Debate adversarial** — Rondas de debate Pro/Con con puntuación de fuerza de argumentos y seguimiento de refutaciones
- **Roles de agentes** — Roles predefinidos (investigador, planificador, desarrollador, revisor, sintetizador) para colaboración en equipo
- **Orquestador de agentes** — Enrutamiento centralizado de mensajes y gestión de estado para equipos multi-agente
- **Grafo de comunicación** — Visualización de interacciones y flujos de mensajes entre agentes
- **Clúster Swarm** — Clúster de agentes multi-proceso con sincronización de permisos y reconexión automática
- **Sistema Buddy** — Agentes compañeros configurables con definición de especies y atributos
- **Memoria compartida** — Espacio de memoria compartido entre agentes con estadísticas y consultas
- **Cron de equipo** — Planificación de tareas cron a nivel de equipo

### ⭐ Sistema de habilidades

- **Mercado de habilidades** — Mercado integrado para explorar e instalar habilidades contribuidas por la comunidad
- **Creación de habilidades** — Creación automática de habilidades a partir de propuestas, con editor Markdown
- **Evolución de habilidades** — Análisis y mejora automáticos impulsados por IA de habilidades existentes basados en retroalimentación de ejecución
- **Coincidencia de habilidades** — Coincidencia semántica, recomendación de habilidades relevantes al contexto de conversación
- **Descomposición de habilidades** — Descomposición automática de tareas complejas en habilidades atómicas ejecutables (asistida por LLM/multi-ronda/validación por flujo de trabajo)
- **Herramientas generadas** — Generación y registro automáticos por IA de nuevas herramientas para expandir las capacidades del agente
- **Hub de habilidades** — Interfaz centralizada de descubrimiento y gestión de configuración de habilidades
- **Cliente del hub de habilidades** — Integración con hub de habilidades remoto, con compartir comunitario
- **Verificación de dependencias de habilidades** — Detección automática de dependencias de habilidades y disponibilidad de herramientas
- **Contenedor sandbox de habilidades** — Ejecución segura de habilidades en un entorno aislado

### 🔄 Sistema de flujo de trabajo

El motor de flujo de trabajo implementa un sistema de orquestación de tareas basado en DAG:

- **Editor de flujo de trabajo visual** — Diseñador de flujos de trabajo por arrastrar y soltar con conexión y configuración de nodos
- **Tipos de nodos ricos** — 15 tipos de nodos: disparador, agente, LLM, condición, paralelo, bucle, fusión, retraso, herramienta, código, sub-flujo de trabajo, búsqueda vectorial, análisis de documento, validación, fin
- **Plantillas de flujo de trabajo** — Preajustes integrados: revisión de código, corrección de bugs, documentación, pruebas, refactoring, exploración, rendimiento, seguridad, desarrollo de funcionalidades
- **Ejecución DAG** — Ordenamiento topológico por algoritmo de Kahn, con detección de ciclos
- **Planificación paralela** — Ejecución en pipeline, los pasos rápidos no esperan a los lentos
- **Estrategia de reintento** — Backoff exponencial, número máximo de reintentos configurable por paso
- **Completado parcial** — Los pasos fallidos no bloquean los pasos descendentes independientes
- **Gestión de versiones** — Control de versiones de plantillas de flujo de trabajo con rollback
- **Historial de ejecución** — Registro detallado con seguimiento de estado y depuración
- **Asistencia IA** — Diseño de flujo de trabajo asistido por IA, recomendación de nodos y optimización de prompts de agentes
- **Verificación semántica** — Validación semántica de flujos de trabajo, detección de problemas potenciales
- **Importación n8n** — Soporte para importación de flujos de trabajo desde directorio n8n
- **Panel de depuración** — Depuración en tiempo real y visualización de estado durante la ejecución del flujo de trabajo

### 📚 Conocimiento y memoria

- **Base de conocimientos (RAG)** — Soporte multi-base de conocimientos, carga de documentos, análisis automático, fragmentación e indexación vectorial
- **Búsqueda híbrida** — Combinación de búsqueda por similitud vectorial y ranking BM25 de texto completo
- **Reranking** — Reranking por cross-encoder para mejorar la precisión de recuperación
- **Pipeline de recall de tres niveles** — Mecanismo de recall multinivel con índice AST + búsqueda vectorial + FTS5
- **Grafo de conocimientos** — Visualización de relaciones entidad-conocimiento (entidades, atributos, relaciones, flujos, interfaces)
- **Sistema Wiki** — Compilador y validador LLM Wiki, con visualización de grafo de conocimientos y sincronización incremental
- **Notas Wiki** — Sistema de notas con enlaces bidireccionales, vista de grafo y sincronización automática de enlaces
- **Sistema de memoria** — Memoria multi-espacio de nombres, con entrada manual o extracción automática por IA
- **Memoria de bucle cerrado** — Integración de proveedores de memoria persistente Honcho y Mem0
- **Búsqueda de texto completo FTS5** — Búsqueda rápida en conversaciones, archivos y memorias
- **Búsqueda de sesiones** — Búsqueda avanzada en todas las sesiones de conversación
- **Gestión de contexto** — Adjuntar de forma flexible archivos, resultados de búsqueda, pasajes de conocimientos, memorias, salidas de herramientas
- **Parser de documentos** — Análisis automático y extracción de contenido de documentos multi-formato
- **Indexación incremental** — Actualización incremental del índice ante cambios de archivos

### 🌐 Pasarela API

- **Servidor API local** — Servidor integrado compatible con OpenAI, Claude y Gemini
- **Enlaces externos** — Integración en un clic con Claude CLI, OpenCode, sincronización automática de claves API y modelos
- **Gestión de claves** — Generación, revocación, activación/desactivación de claves de acceso con descripciones
- **Análisis de uso** — Volumen de solicitudes y uso de tokens por clave, proveedor y fecha
- **Soporte SSL/TLS** — Certificados autofirmados integrados, soporte para certificados personalizados
- **Registros de solicitudes** — Registro completo de todas las solicitudes y respuestas API
- **Plantillas de configuración** — Plantillas preconstruidas para Claude, Codex, OpenCode, Gemini
- **API Realtime** — Push de eventos WebSocket compatible con la API Realtime de OpenAI
- **Integración de plataforma** — Soporte para DingTalk, Feishu, QQ, Slack, WeChat, WhatsApp, Telegram, Discord
- **Diagnóstico de pasarela** — Diagnóstico de conexión y gestión de políticas de programa
- **Limitador de tasa** — Limitación de tasa de solicitudes API y control de flujo
- **Cola persistente** — Gestión de cola de solicitudes persistente

### 🔧 Herramientas y extensiones

- **Protocolo MCP** — Implementación completa del Model Context Protocol, soporte de transportes stdio y HTTP/WebSocket
- **Autenticación OAuth** — Soporte de flujo OAuth para servidores MCP
- **Inicio automático MCP** — Inicio automático y gestión del ciclo de vida de servidores MCP
- **Puente de herramientas MCP** — Puente entre herramientas MCP y el sistema de herramientas del agente
- **Sistema de plugins** — Arquitectura de plugins de tres niveles (integrado/empaquetado/externo), con registro de herramientas, hooks y gestión del ciclo de vida
- **Herramientas integradas** — Operaciones de archivos completas (lectura/escritura/edición), ejecución de código, búsqueda (Grep/Glob), Bash, búsqueda web, extracción web, gestión de planes, planificación Cron, REPL, LSP, gestión de contexto, control informático, envío de mensajes, lista de tareas, etc.
- **Sistema de permisos de herramientas** — Clasificación de permisos de herramientas, gestión de reglas y seguimiento de uso
- **Seguridad Bash** — Análisis de comandos, validación de rutas y control de seguridad sandbox
- **Cliente LSP** — Protocolo Language Server integrado, completación de código y diagnósticos
- **Índice AST** — Análisis e indexación AST de archivos de código
- **Backend de terminal** — Soporte para conexiones de terminal locales, Docker y SSH
- **Automatización de navegador** — Control de navegador vía integración CDP (navegación, capturas, clics, relleno, extracción de texto, etc.)
- **Automatización UI** — Identificación y control de elementos UI multiplataforma
- **Herramientas Git** — Operaciones Git con detección de ramas y sensibilidad a conflictos
- **Recomendación de herramientas** — Motor de recomendación inteligente de herramientas basado en contexto
- **Orquestación de herramientas** — Coordinación y ejecución multi-herramienta con salida en streaming
- **Estadísticas de herramientas** — Estadísticas de frecuencia de uso y rendimiento de herramientas

### 📊 Renderizado de contenido

- **Renderizado Markdown** — Soporte completo para resaltado de código, fórmulas matemáticas LaTeX, tablas, listas de tareas
- **Editor de código Monaco** — Editor integrado con resaltado de sintaxis, copia, vista previa diff
- **Renderizado de diagramas** — Diagramas de flujo Mermaid, diagramas de arquitectura D2, gráficos interactivos ECharts
- **Panel de artefactos** — Fragmentos de código, borradores HTML, componentes React, notas Markdown, con vista previa en tiempo real
- **Cuatro modos de vista previa** — Código (editor), Dividido (lado a lado), Vista previa (solo renderizado), Vista previa de componente React
- **Inspector de sesión** — Vista de árbol de la estructura de sesión, navegación rápida
- **Panel de citas** — Seguimiento y visualización de citas fuente con puntuación de credibilidad
- **Renderizado de infografías** — Soporte para visualización de infografías

### 🛡️ Datos y seguridad

- **Cifrado AES-256** — Claves API y datos sensibles cifrados con AES-256-GCM
- **Almacenamiento aislado** — Estado de la aplicación en `~/.axagent/`, archivos de usuario en `~/Documents/axagent/`
- **Copia de seguridad automática** — Copias de seguridad programadas a directorio local o almacenamiento WebDAV
- **Restauración de copia de seguridad** — Restauración en un clic desde copias de seguridad históricas
- **Opciones de exportación** — Capturas PNG, Markdown, texto plano, JSON
- **Gestión de almacenamiento** — Visualización del uso del disco y herramientas de limpieza
- **Autorización de archivos** — Gestión de autorización y revocación de acceso a archivos
- **Auditoría de operaciones** — Registro de auditoría de operaciones críticas

### 🖥️ Experiencia de escritorio

- **Motor de temas** — Temas oscuro/claro, seguimiento del sistema o preferencia manual
- **Idioma de interfaz** — 11 idiomas: chino simplificado, chino tradicional, inglés, japonés, coreano, francés, alemán, español, ruso, hindi, árabe
- **Bandeja del sistema** — Minimización a la bandeja del sistema sin interrumpir servicios en segundo plano
- **Ventana siempre visible** — Ventana fijada sobre todas las demás ventanas
- **Atajos globales** — Atajos de teclado globales personalizables para invocar la ventana principal
- **QuickBar** — Barra flotante de acceso rápido, invocación en un clic
- **Inicio automático** — Lanzamiento opcional al iniciar el sistema
- **Soporte de proxy** — Configuración de proxy HTTP y SOCKS5
- **Actualización automática** — Verificación automática de versiones, notificación de actualización
- **Paleta de comandos** — `Cmd/Ctrl+K` para acceso rápido a comandos
- **Asistente de incorporación** — Guía interactiva de primer uso y detección de Ollama
- **Centro de notificaciones** — Gestión unificada de notificaciones en la aplicación

### 🔬 Funcionalidades avanzadas

- **Investigación profunda** — Búsqueda multi-fuente, seguimiento de citas, evaluación de credibilidad y síntesis de contenido
- **Verificación de hechos** — Verificación de hechos impulsada por IA y clasificación de fuentes
- **Planificador Cron** — Planificación de tareas automatizada con plantillas diarias/semanales/mensuales y expresiones cron personalizadas
- **Sistema Webhook** — Suscripción a eventos, notificaciones de completado de herramientas, errores de agentes, fin de sesión
- **Perfil de usuario** — Aprendizaje automático de estilo de código, convenciones de nomenclatura, indentación, estilo de comentarios, preferencias de comunicación
- **Optimizador RL** — Optimización por aprendizaje por refuerzo de selección de herramientas y estrategias de tareas
- **Ajuste fino LoRA** — Adaptación de modelo personalizada con ajuste fino LoRA local
- **Sugerencias proactivas** — Indicaciones contextuales basadas en contenido de conversación y patrones de usuario
- **Predicción de contexto** — Predicción de las próximas acciones del usuario y precarga de recursos relevantes
- **Integración onírica** — Integración automática en segundo plano de memorias y patrones, optimización del conocimiento a largo plazo
- **Recuperación de errores** — Clasificación automática de errores, análisis de causas raíz y sugerencias de recuperación
- **Herramientas de desarrollo** — Trace, Span, visualización de timeline para depuración y análisis de rendimiento
- **Sistema de benchmark** — Evaluación de rendimiento SWE-bench / Terminal-bench con scorecards
- **Transferencia de estilo** — Aplicación de preferencias de estilo de código aprendidas al código generado
- **Plugins de panel** — Panel extensible con paneles y widgets personalizados
- **Colaboración y compartir** — Colaboración en tiempo real CRDT y compartir sesión en un clic
- **Extensión de navegador** — Extensión de navegador Wiki Clipper para recorte rápido de páginas web al Wiki LLM
- **SDK Python** — SDK Python para integración con AxAgent
- **Enrutador inteligente** — Enrutamiento y clasificación inteligentes de solicitudes
- **Caché semántico** — Caché de respuestas basado en semántica, reducción de cálculo redundante
- **Compresión de contexto** — Compresión automática de contextos largos, optimización del uso de tokens
- **Procesamiento por lotes de mensajes** — Envío y optimización por lotes de mensajes
- **Pool de conexiones** — Gestión del pool de conexiones de base de datos y API
- **Feature flags** — Sistema de feature flags configurable
- **Motor de políticas** — Gestión centralizada de políticas de permisos y operaciones
- **Gobernador de recursos** — Limitación y gobernanza del uso de recursos por agentes
- **Transferencia LAN** — Capacidad de transferencia de archivos en red local

---

## Arquitectura técnica

### Pila de tecnología

| Capa | Tecnología |
|------|------------|
| **Framework** | Tauri 2 + React 19 + TypeScript 6 |
| **UI** | Ant Design 6 + TailwindCSS 4 |
| **Gestión de estado** | Zustand 5 |
| **Enrutamiento** | React Router 7 |
| **i18n** | i18next + react-i18next |
| **Backend** | Rust + SeaORM 2 + SQLite |
| **Base de datos vectorial** | sqlite-vec |
| **Editor de código** | Monaco Editor |
| **Diagramas** | Mermaid + D2 + ECharts (CDN) |
| **Terminal** | xterm.js 6 |
| **Flujo de trabajo** | ReactFlow 11 |
| **Build** | Vite 8 + npm |

### Arquitectura Backend Rust

El backend está organizado como un workspace de Rust con 10 crates especializadas:

```
src-tauri/crates/
├── agent/         # Núcleo del agente IA
│   ├── react_engine.rs          # Motor de razonamiento ReAct
│   ├── coordinator.rs           # Coordinación de agentes
│   ├── hierarchical_planner.rs  # Descomposición de tareas
│   ├── task_decomposer.rs       # Descomposición de subtareas
│   ├── self_verifier.rs         # Verificación de salidas
│   ├── verification_agent.rs    # Agente de verificación
│   ├── error_recovery_engine.rs # Motor de recuperación de errores
│   ├── error_classifier.rs      # Clasificación de errores
│   ├── recovery_strategies.rs   # Estrategias de recuperación
│   ├── loop_detector.rs         # Detección de bucles
│   ├── vision_pipeline.rs       # Percepción de pantalla
│   ├── deep_research.rs         # Investigación profunda
│   ├── fact_checker.rs          # Verificación de hechos
│   ├── research_agent.rs        # Agente de investigación
│   ├── search_planner.rs        # Planificación de búsqueda
│   ├── search_orchestrator.rs   # Orquestación de búsqueda
│   ├── academic_search.rs       # Búsqueda académica
│   ├── source_validator.rs      # Validación de fuentes
│   ├── source_classifier.rs     # Clasificación de fuentes
│   ├── credibility_evaluator.rs # Evaluación de credibilidad
│   ├── citation_tracker.rs      # Seguimiento de citas
│   ├── content_synthesizer.rs   # Síntesis de contenido
│   ├── outline_builder.rs       # Construcción de esquemas
│   ├── reference_builder.rs     # Construcción de referencias
│   ├── proactive_mode.rs        # Modo proactivo
│   ├── purpose_manager.rs       # Gestión de propósitos
│   ├── graph_insights.rs        # Insights de grafos
│   ├── insight_generator.rs     # Generación de insights
│   ├── schema_manager.rs        # Gestión de esquemas
│   ├── ingest_pipeline.rs       # Pipeline de ingesta de datos
│   ├── session_manager.rs       # Gestión de sesiones
│   ├── health_checker.rs        # Verificación de salud
│   ├── metrics.rs               # Recopilación de métricas
│   ├── evaluator/               # Evaluación de benchmarks
│   ├── fine_tune/               # Ajuste fino LoRA
│   ├── rl_optimizer/            # Optimización de estrategias RL
│   └── tool_recommender/        # Motor de recomendación de herramientas
│
├── core/          # Utilidades principales
│   ├── db.rs                   # Base de datos SeaORM
│   ├── vector_store.rs         # Integración sqlite-vec
│   ├── rag.rs                  # Capa de abstracción RAG
│   ├── hybrid_search.rs        # Búsqueda vectorial + FTS5
│   ├── recall_pipeline.rs      # Pipeline de recall de tres niveles
│   ├── crypto.rs               # Cifrado AES-256
│   ├── mcp_client.rs           # Cliente protocolo MCP
│   ├── browser_automation.rs   # Automatización de navegador
│   ├── computer_control.rs     # Control informático
│   ├── screen_vision.rs        # Visión de pantalla
│   ├── screen_capture.rs       # Captura de pantalla
│   ├── ui_automation.rs        # Automatización UI
│   ├── ast_index.rs            # Índice AST
│   ├── incremental_indexer.rs  # Indexación incremental
│   ├── document_parser.rs      # Parser de documentos
│   ├── markdown_parser.rs      # Parser Markdown
│   ├── text_chunker.rs         # Fragmentación de texto
│   ├── token_counter.rs        # Conteo de tokens
│   ├── token_budget.rs         # Presupuesto de tokens
│   ├── file_index.rs           # Índice de archivos
│   ├── file_authorizer.rs      # Autorización de archivos
│   ├── file_store.rs           # Almacenamiento de archivos
│   ├── cache.rs                # Gestión de caché
│   ├── disk_cache.rs           # Caché de disco
│   ├── cache_persister.rs      # Persistencia de caché
│   ├── cache_snapshot.rs       # Instantánea de caché
│   ├── vector_cache.rs         # Caché vectorial
│   ├── marketplace_service.rs  # Servicio de mercado
│   ├── marketplace.rs          # Abstracción de mercado
│   ├── operation_audit.rs      # Auditoría de operaciones
│   ├── unified_config.rs       # Configuración unificada
│   ├── platform_config.rs      # Configuración de plataforma
│   ├── command_validator.rs    # Validación de comandos
│   ├── shell_parser.rs         # Parser Shell
│   ├── output_processor.rs     # Procesamiento de salidas
│   ├── storage_inventory.rs    # Inventario de almacenamiento
│   ├── storage_migration.rs    # Migración de almacenamiento
│   ├── storage_paths.rs        # Rutas de almacenamiento
│   ├── s3_backup.rs            # Backup S3
│   ├── webdav.rs               # Sincronización WebDAV
│   ├── git_tools.rs            # Herramientas Git
│   ├── sandbox_runner.rs       # Ejecutor sandbox
│   ├── search.rs               # Abstracción de búsqueda
│   ├── reranker.rs             # Reranking
│   ├── model_knowledge.rs      # Conocimiento de modelos
│   ├── prompt_template.rs      # Plantillas de prompts
│   ├── preset_templates.rs     # Plantillas predefinidas
│   ├── workflow_types.rs       # Tipos de flujo de trabajo
│   ├── workflow_version.rs     # Versión de flujo de trabajo
│   ├── path_vars.rs            # Variables de ruta
│   ├── entity/                 # Entidades SeaORM (40+ tablas)
│   └── repo/                   # Repositorios de datos (30+ repos)
│
├── gateway/       # Pasarela API
│   ├── server.rs               # Servidor HTTP
│   ├── handlers.rs             # Gestores API
│   ├── routes.rs               # Definición de rutas
│   ├── auth.rs                 # Autenticación
│   ├── middleware.rs           # Middleware
│   ├── metrics.rs              # Recopilación de métricas
│   ├── native.rs               # Integración nativa
│   ├── marketplace_handlers.rs # Interfaz de mercado
│   └── realtime.rs             # Soporte WebSocket
│
├── plugins/       # Sistema de plugins
│   ├── hooks.rs                # Ejecutor de hooks
│   ├── agent_provider.rs       # Proveedor de agentes
│   ├── test_isolation.rs       # Aislamiento de pruebas
│   └── lib.rs                  # Registro de plugins y ciclo de vida
│
├── providers/     # Adaptadores de modelos
│   ├── adapter.rs              # Interfaz de adaptador
│   ├── registry.rs             # Registro de proveedores
│   ├── openai.rs               # API OpenAI
│   ├── openai_responses.rs     # API Responses OpenAI
│   ├── anthropic.rs            # API Claude
│   ├── gemini.rs               # API Gemini
│   ├── ollama.rs               # Ollama local
│   ├── openclaw.rs             # OpenClaw
│   ├── hermes.rs               # Hermes
│   ├── image_gen.rs            # Generación de imágenes
│   ├── realtime_client.rs      # Cliente API Realtime
│   └── transport/              # Capa de transporte (Chat Completions / Responses / Anthropic)
│
├── runtime/       # Servicios de ejecución
│   ├── session.rs              # Gestión de sesiones
│   ├── workflow_engine.rs      # Orquestación DAG
│   ├── work_engine/            # Motor de trabajo (ejecutores de nodos + planificador + capa de caché)
│   ├── mcp.rs                  # Servidor MCP
│   ├── mcp_client.rs           # Cliente MCP
│   ├── mcp_server.rs           # Implementación del servidor MCP
│   ├── mcp_stdio.rs            # Transporte MCP stdio
│   ├── mcp_autostart.rs        # Inicio automático MCP
│   ├── mcp_lifecycle_hardened.rs # Gestión del ciclo de vida MCP
│   ├── mcp_tool_bridge.rs      # Puente de herramientas MCP
│   ├── cron/                   # Planificación de tareas
│   ├── terminal/               # Backends de terminal (local/Docker/SSH)
│   ├── benchmarks/             # SWE-bench / Terminal-bench
│   ├── collaboration/          # Colaboración CRDT y compartir sesión
│   ├── tool_generator/         # Generación de herramientas IA
│   ├── message_gateway/        # Integraciones de plataforma (DingTalk/Feishu/QQ/Slack/WeChat/WhatsApp/Telegram/Discord)
│   ├── buddy/                  # Sistema Buddy (especies/atributos/gestor)
│   ├── swarm/                  # Clúster Swarm (backend de procesos/sincronización de permisos/reconexión)
│   ├── tasks/                  # Tareas en segundo plano (sueños/agentes remotos/compañeros in-process)
│   ├── adversarial_debate.rs   # Debate adversarial
│   ├── agent_orchestrator.rs   # Orquestación multi-agente
│   ├── agent_roles.rs          # Roles de agentes
│   ├── webhook_dispatcher.rs   # Despacho de webhooks
│   ├── webhook_server.rs       # Servidor de webhooks
│   ├── session_search.rs       # Búsqueda de sesiones
│   ├── dashboard_plugin.rs     # Plugin de panel
│   ├── dashboard_registry.rs   # Registro de panel
│   ├── permissions.rs          # Gestión de permisos
│   ├── permission_enforcer.rs  # Aplicación de permisos
│   ├── policy_engine.rs        # Motor de políticas
│   ├── trust_resolver.rs       # Resolución de confianza
│   ├── resource_governor.rs    # Gobernador de recursos
│   ├── green_contract.rs       # Contrato verde
│   ├── feature_flags.rs        # Feature flags
│   ├── module_switch.rs        # Interruptor de módulos
│   ├── mode_selector.rs        # Selector de modo
│   ├── config.rs               # Configuración de ejecución
│   ├── config_validate.rs      # Validación de configuración
│   ├── prompt.rs               # Gestión de prompts
│   ├── prompt_cache.rs         # Caché de prompts
│   ├── compact.rs              # Compresión de contexto
│   ├── summary_compression.rs  # Compresión de resumen
│   ├── compact_thresholds.rs   # Umbrales de compresión
│   ├── compact_warning.rs      # Advertencia de compresión
│   ├── reactive_compact.rs     # Compresión reactiva
│   ├── session_memory_compact.rs # Compresión de memoria de sesión
│   ├── message_importance.rs   # Evaluación de importancia de mensajes
│   ├── message_batching.rs     # Procesamiento por lotes de mensajes
│   ├── rate_limiter.rs         # Limitador de tasa
│   ├── connection_pool.rs      # Pool de conexiones
│   ├── persistent_queue.rs     # Cola persistente
│   ├── persistent_queue_manager.rs # Gestor de cola
│   ├── health_check.rs         # Verificación de salud
│   ├── cache_guard.rs          # Guardia de caché
│   ├── checkpoint.rs           # Punto de control
│   ├── branch_lock.rs          # Bloqueo de rama
│   ├── stale_base.rs           # Detección de base obsoleta
│   ├── watch_patterns.rs       # Patrones de vigilancia
│   ├── lan_transfer.rs         # Transferencia LAN
│   ├── tls_config.rs           # Configuración TLS
│   ├── sse.rs                  # Flujo de eventos SSE
│   ├── api_server.rs           # Servidor API
│   ├── gateway_auth.rs         # Autenticación de pasarela
│   ├── gateway_metrics.rs      # Métricas de pasarela
│   ├── bash.rs                 # Ejecución Bash
│   ├── bash_validation.rs      # Validación Bash
│   ├── shell_hooks.rs          # Hooks Shell
│   ├── shell_completer.rs      # Autocompletado Shell
│   ├── terminal_analyzer.rs    # Analizador de terminal
│   ├── git_context.rs          # Contexto Git
│   ├── git_tools.rs            # Herramientas Git
│   ├── file_ops.rs             # Operaciones de archivos
│   ├── hooks.rs                # Gestión de hooks
│   ├── hook_chain.rs           # Cadena de hooks
│   ├── hook_config.rs          # Configuración de hooks
│   ├── plugin_hooks.rs         # Hooks de plugins
│   ├── plugin_lifecycle.rs     # Ciclo de vida de plugins
│   ├── profile.rs              # Perfil
│   ├── profile_manager.rs      # Gestor de perfiles
│   ├── oauth.rs                # Autenticación OAuth
│   ├── usage.rs                # Estadísticas de uso
│   ├── bootstrap.rs            # Arranque
│   ├── worker_boot.rs          # Arranque de worker
│   ├── fork_bridge.rs          # Puente de fork
│   ├── task_packet.rs          # Paquete de tareas
│   ├── task_router.rs          # Enrutador de tareas
│   ├── task_registry.rs        # Registro de tareas
│   ├── transform_pipeline.rs   # Pipeline de transformación
│   ├── transport_handlers.rs   # Gestores de transporte
│   ├── general_engine.rs       # Motor general
│   ├── engine_bridge.rs        # Puente de motor
│   ├── conversation.rs         # Gestión de conversación
│   ├── session_control.rs      # Control de sesión
│   ├── shared_memory.rs        # Memoria compartida
│   ├── validation_executor.rs  # Ejecutor de validación
│   ├── recovery_recipes.rs     # Recetas de recuperación
│   ├── error_recovery.rs       # Recuperación de errores
│   ├── theme_engine.rs         # Motor de temas
│   ├── token_budget_predictor.rs # Predicción de presupuesto de tokens
│   ├── team_cron_registry.rs   # Registro Cron de equipo
│   ├── module_dream.rs         # Módulo onírico
│   ├── json.rs                 # Utilidades JSON
│   └── lane_events.rs          # Eventos Lane
│
├── telemetry/     # Telemetría y rastreo
│   ├── tracer.rs              # Rastreo distribuido
│   ├── metrics.rs             # Recopilación de métricas
│   ├── span.rs                # Gestión de Spans
│   ├── event.rs               # Definición de eventos
│   ├── collector.rs           # Recopilación de datos
│   ├── exporter.rs            # Exportación de datos
│   └── storage.rs             # Backend de almacenamiento
│
├── tools/         # Sistema de herramientas
│   ├── registry.rs             # Registro de herramientas
│   ├── builtin_tools.rs        # Definiciones de herramientas integradas
│   ├── builtin_handlers.rs     # Gestores de herramientas integradas
│   ├── orchestration.rs        # Orquestación de herramientas
│   ├── streaming.rs            # Salida en streaming
│   ├── stats.rs                # Estadísticas de uso
│   ├── recorder.rs             # Registro de ejecución
│   ├── agent_def_loader.rs     # Cargador de definiciones de agentes
│   ├── agent_def_types.rs      # Tipos de definiciones de agentes
│   ├── bash/                   # Herramienta Bash (parser/sandbox/seguridad/validación de rutas)
│   ├── hooks/                  # Hooks (registro/ejecutor)
│   ├── mcp/                    # Herramientas MCP (registro/OAuth/wrapper)
│   ├── permissions/            # Permisos (clasificador/reglas/seguidor)
│   └── tools/                  # Implementaciones de herramientas específicas
│       ├── agent.rs            # Herramienta de agente
│       ├── bash.rs             # Ejecución Bash
│       ├── context.rs          # Gestión de contexto
│       ├── cron.rs             # Planificación Cron
│       ├── glob.rs             # Glob de archivos
│       ├── grep.rs             # Búsqueda de contenido
│       ├── lsp.rs              # Herramienta LSP
│       ├── monitor.rs          # Herramienta de monitor
│       ├── plan.rs             # Herramienta de plan
│       ├── repl.rs             # Herramienta REPL
│       ├── skill.rs            # Herramienta de habilidad
│       ├── web_fetch.rs        # Extracción web
│       ├── web_search.rs       # Búsqueda web
│       ├── file_read.rs        # Lectura de archivo
│       ├── file_write.rs       # Escritura de archivo
│       ├── file_edit.rs        # Edición de archivo
│       ├── computer_use.rs     # Control informático
│       ├── messaging.rs        # Envío de mensajes
│       ├── push_notification.rs # Notificación push
│       ├── task_system.rs      # Sistema de tareas
│       ├── todo_write.rs       # Lista de tareas
│       └── batch_missing.rs    # Detección de lotes faltantes
│
├── trajectory/    # Sistema de aprendizaje
│   ├── memory.rs              # Gestión de memoria
│   ├── memory_provider.rs     # Interfaz de proveedor de memoria
│   ├── auto_memory.rs         # Extracción automática de memoria
│   ├── skill.rs               # Sistema de habilidades
│   ├── skill_manager.rs       # Gestor de habilidades
│   ├── skill_evolution.rs     # Evolución de habilidades
│   ├── skill_matcher.rs       # Coincidencia de habilidades
│   ├── skill_proposal.rs      # Propuesta de habilidades
│   ├── skills_hub_adapter.rs  # Adaptador del hub de habilidades
│   ├── skills_hub_client.rs   # Cliente del hub de habilidades
│   ├── skill_decomposition/   # Descomposición de habilidades (asistida por LLM/multi-ronda/validación flujo de trabajo/análisis de herramientas)
│   ├── rl.rs                  # Señales de recompensa RL
│   ├── rl_trainer.rs          # Entrenador RL
│   ├── training_env.rs        # Entorno de entrenamiento
│   ├── behavior_learner.rs    # Aprendizaje comportamental
│   ├── behavior_tracker.rs    # Seguimiento comportamental
│   ├── pattern.rs             # Reconocimiento de patrones
│   ├── pattern_analyzer.rs    # Análisis de patrones
│   ├── user_profile.rs        # Perfil de usuario
│   ├── preference_learner.rs  # Aprendizaje de preferencias
│   ├── adaptation.rs          # Ajuste adaptativo
│   ├── dream_consolidation.rs # Integración onírica
│   ├── parallel_execution.rs  # Servicio de ejecución paralela
│   ├── style_extractor.rs     # Extracción de estilo
│   ├── style_applier.rs       # Aplicación de estilo
│   ├── style_vectorizer.rs    # Vectorización de estilo
│   ├── style_migrator.rs      # Migración de estilo
│   ├── suggestion_engine.rs   # Motor de sugerencias
│   ├── proactive_assistant.rs # Asistente proactivo
│   ├── context_predictor.rs   # Predicción de contexto
│   ├── task_prefetcher.rs     # Prebúsqueda de tareas
│   ├── reminder_manager.rs    # Gestión de recordatorios
│   ├── nudge.rs               # Sistema de nudges
│   ├── insight.rs             # Generación de insights
│   ├── compactor.rs           # Compresión de datos
│   ├── trajectory.rs          # Gestión de trayectoria
│   ├── trajectory_compressor.rs # Compresión de trayectoria
│   ├── sub_agent.rs           # Sub-agente
│   ├── batch.rs               # Procesamiento por lotes
│   ├── context.rs             # Gestión de contexto
│   ├── fts5.rs                # Búsqueda FTS5
│   ├── hooks.rs               # Hooks
│   ├── storage.rs             # Almacenamiento
│   ├── scheduled_task.rs      # Tarea programada
│   └── memory_providers/      # Proveedores de memoria (Honcho/Mem0/bucle cerrado/servicios)
│
└── migration/     # Migraciones de base de datos
    └── m20240101_000001~000010  # 10 archivos de migración
```

### Arquitectura Frontend

```
src/
├── stores/                    # Gestión de estado Zustand
│   ├── domain/               # Estado de negocio principal
│   │   ├── conversationStore.ts
│   │   ├── messageStore.ts
│   │   ├── streamStore.ts
│   │   ├── multiModelStore.ts
│   │   ├── preferenceStore.ts
│   │   └── compressStore.ts
│   ├── feature/               # Estado de módulos funcionales (30+ stores)
│   │   ├── agentStore.ts
│   │   ├── agentProfileStore.ts
│   │   ├── appConfigStore.ts
│   │   ├── backupStore.ts
│   │   ├── buddyStore.ts
│   │   ├── categoryStore.ts
│   │   ├── decompositionStore.ts
│   │   ├── dreamStore.ts
│   │   ├── executionStore.ts
│   │   ├── expertStore.ts
│   │   ├── fileStore.ts
│   │   ├── gatewayStore.ts
│   │   ├── gatewayLinkStore.ts
│   │   ├── generatedToolStore.ts
│   │   ├── helpStore.ts
│   │   ├── knowledgeStore.ts
│   │   ├── llmWikiStore.ts
│   │   ├── localToolStore.ts
│   │   ├── memoryStore.ts
│   │   ├── mcpStore.ts
│   │   ├── nudgeStore.ts
│   │   ├── onboardingStore.ts
│   │   ├── planStore.ts
│   │   ├── platformStore.ts
│   │   ├── proactiveStore.ts
│   │   ├── promptTemplateStore.ts
│   │   ├── providerStore.ts
│   │   ├── searchStore.ts
│   │   ├── settingsStore.ts
│   │   ├── skillExtensionStore.ts
│   │   ├── skillStore.ts
│   │   ├── styleStore.ts
│   │   ├── terminalStore.ts
│   │   ├── themeStore.ts
│   │   ├── trajectoryStore.ts
│   │   ├── userProfileStore.ts
│   │   ├── wikiStore.ts
│   │   ├── workEngineStore.ts
│   │   └── workflowEditorStore.ts
│   ├── devtools/              # Estado de herramientas de desarrollo
│   │   ├── tracerStore.ts
│   │   ├── evaluatorStore.ts
│   │   ├── rlStore.ts
│   │   ├── fineTuneStore.ts
│   │   └── recommendationStore.ts
│   └── shared/                # Estado compartido
│       ├── artifactStore.ts
│       ├── chatWorkspaceStore.ts
│       ├── tabStore.ts
│       └── uiStore.ts
│
├── components/                # Componentes React (24 módulos)
│   ├── chat/                # Interfaz de chat (90+ componentes)
│   ├── workflow/            # Editor de flujo de trabajo (nodos/paneles/plantillas/asistencia IA)
│   ├── gateway/             # UI Pasarela API
│   ├── settings/            # Paneles de configuración (40+ componentes)
│   ├── terminal/            # UI Terminal
│   ├── skill/               # Editor y renderizador de habilidades
│   ├── benchmark/           # Panel de benchmarks
│   ├── decomposition/       # Descomposición de habilidades y generación de herramientas
│   ├── files/               # Página de gestión de archivos
│   ├── fine-tune/           # Configuración de ajuste fino LoRA
│   ├── link/                # Gestión de enlaces externos
│   ├── llm-wiki/            # Editor LLM Wiki
│   ├── proactive/           # Sistema de sugerencias proactivas
│   ├── recommendation/      # Panel de recomendación de herramientas
│   ├── wiki/                # Gestión Wiki
│   ├── devtools/            # Timeline Trace/Span
│   ├── style/               # Transferencia de estilo de código
│   ├── layout/              # Componentes de diseño (barra de título/barra lateral/paleta de comandos)
│   ├── help/                # Panel de ayuda
│   ├── onboarding/          # Asistente de incorporación
│   ├── notification/        # Centro de notificaciones
│   ├── search/              # Búsqueda de sesiones
│   ├── common/              # Componentes comunes
│   └── shared/              # Componentes compartidos
│
├── pages/                    # Componentes de página (22 páginas)
│   ├── ChatPage.tsx
│   ├── KnowledgePage.tsx
│   ├── KnowledgeHubPage.tsx
│   ├── MemoryPage.tsx
│   ├── WorkflowPage.tsx
│   ├── WorkflowMarketplace.tsx
│   ├── GatewayPage.tsx
│   ├── GatewayLinkPage.tsx
│   ├── LinkPage.tsx
│   ├── FilesPage.tsx
│   ├── FineTunePage.tsx
│   ├── SkillsPage.tsx
│   ├── WikiPage.tsx
│   ├── WikiEditorPage.tsx
│   ├── WikiGraphPage.tsx
│   ├── LlmWikiPage.tsx
│   ├── LlmWikiEditorPage.tsx
│   ├── IngestPage.tsx
│   ├── QuickBarPage.tsx
│   ├── SettingsPage.tsx
│   └── DevTools/
│       ├── TraceExplorer.tsx
│       ├── BenchmarkRunner.tsx
│       └── ToolRecommender.tsx
│
├── hooks/                    # React hooks (10)
├── lib/                      # Funciones utilitarias (incluyendo Web Worker)
├── types/                    # Definiciones de tipos TypeScript (22)
├── sdk/                      # SDK (incluyendo SDK Python)
└── i18n/                     # Traducciones en 11 idiomas
```

### Soporte de plataformas

| Plataforma | Arquitectura |
|------------|-------------|
| macOS | Apple Silicon (arm64), Intel (x86_64) |
| Windows | x86_64, ARM64 |
| Linux | x86_64, ARM64 (AppImage/deb/rpm) |

## Inicio rápido

### Descargar versiones preconstruidas

Visite la página [Releases](https://github.com/polite0803/AxAgent/releases) para descargar el instalador de su plataforma.

### Compilar desde el código fuente

#### Requisitos previos

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+
- [npm](https://www.npmjs.com/) 10+
- Windows: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) + objetivos Rust MSVC

#### Pasos de compilación

```bash
# Clonar el repositorio
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent

# Instalar dependencias
npm install

# Modo desarrollo
npm run tauri dev

# Compilar solo el frontend
npm run build

# Compilar la aplicación de escritorio
npm run tauri build
```

Los artefactos de compilación están en `src-tauri/target/release/`.

### Pruebas

```bash
# Pruebas unitarias
npm run test

# Pruebas E2E
npm run test:e2e

# Verificación de tipos
npm run typecheck

# Formateo de código
npm run format

# Verificación CI
npm run ci:check
```

---

## Estructura del proyecto

```
AxAgent/
├── src/                         # Código fuente frontend (React + TypeScript)
│   ├── components/              # Componentes React (24 módulos)
│   │   ├── chat/               # Interfaz de chat (90+ componentes)
│   │   ├── workflow/           # Componentes del editor de flujo de trabajo
│   │   ├── gateway/            # Componentes de la pasarela API
│   │   ├── settings/           # Paneles de configuración (40+ componentes)
│   │   ├── terminal/           # Componentes de terminal
│   │   ├── skill/              # Editor y renderizador de habilidades
│   │   ├── benchmark/          # Benchmarks
│   │   ├── decomposition/      # Descomposición de habilidades
│   │   ├── files/              # Gestión de archivos
│   │   ├── fine-tune/          # Ajuste fino LoRA
│   │   ├── link/               # Enlaces externos
│   │   ├── llm-wiki/           # LLM Wiki
│   │   ├── proactive/          # Sugerencias proactivas
│   │   ├── recommendation/     # Recomendación de herramientas
│   │   ├── wiki/               # Gestión Wiki
│   │   ├── devtools/           # Herramientas de desarrollo
│   │   ├── style/              # Estilo de código
│   │   ├── layout/             # Componentes de diseño
│   │   ├── help/               # Panel de ayuda
│   │   ├── onboarding/         # Asistente de incorporación
│   │   ├── notification/       # Centro de notificaciones
│   │   ├── search/             # Búsqueda de sesiones
│   │   ├── common/             # Componentes comunes
│   │   └── shared/             # Componentes compartidos
│   ├── pages/                   # Componentes de página (22 páginas)
│   ├── stores/                  # Gestión de estado Zustand
│   │   ├── domain/            # Estado de negocio principal (6 stores)
│   │   ├── feature/           # Estado de módulos funcionales (30+ stores)
│   │   ├── devtools/          # Estado de herramientas de desarrollo (5 stores)
│   │   └── shared/            # Estado compartido (4 stores)
│   ├── hooks/                   # React hooks (10)
│   ├── lib/                     # Funciones utilitarias (incluyendo Web Worker)
│   ├── types/                   # Definiciones de tipos TypeScript (22)
│   ├── sdk/                     # SDK (incluyendo SDK Python)
│   └── i18n/                    # Traducciones en 11 idiomas
│
├── src-tauri/                    # Código fuente backend (Rust)
│   ├── crates/                  # Workspace Rust (10 crates)
│   │   ├── agent/             # Núcleo del agente IA
│   │   ├── core/              # Base de datos, cifrado, RAG
│   │   ├── gateway/           # Servidor pasarela API
│   │   ├── plugins/           # Sistema de plugins
│   │   ├── providers/         # Adaptadores de proveedores de modelos
│   │   ├── runtime/           # Servicios de ejecución
│   │   ├── tools/             # Sistema de herramientas
│   │   ├── trajectory/        # Memoria y aprendizaje
│   │   ├── telemetry/         # Rastreo y métricas
│   │   └── migration/         # Migraciones de base de datos
│   └── src/                    # Punto de entrada Tauri (70+ módulos de comandos)
│
├── extension/                  # Extensión de navegador (Wiki Clipper)
├── e2e/                        # Pruebas E2E Playwright
├── scripts/                    # Scripts de build y herramientas
└── website/                    # Sitio web del proyecto (VitePress)
```

## Directorio de datos

```
~/.axagent/                      # Directorio de configuración
├── axagent.db                   # Base de datos SQLite
├── master.key                   # Clave maestra AES-256
├── vector_db/                   # Base de datos vectorial (sqlite-vec)
└── ssl/                         # Certificados SSL

~/Documents/axagent/            # Directorio de archivos de usuario
├── images/                     # Imágenes adjuntas
├── files/                      # Archivos adjuntos
└── backups/                    # Archivos de copia de seguridad
```

---

## Preguntas frecuentes

### macOS: «La app está dañada» o «No se puede verificar al desarrollador»

Como la aplicación no está firmada por Apple:

**1. Permitir apps de «Cualquier origen»**
```bash
sudo spctl --master-disable
```

Luego ve a **Configuración del sistema → Privacidad y seguridad → Seguridad** y selecciona **Cualquier origen**.

**2. Eliminar el atributo de cuarentena**
```bash
sudo xattr -dr com.apple.quarantine /Applications/AxAgent.app
```

**3. Paso adicional para macOS Ventura+**
Ve a **Configuración del sistema → Privacidad y seguridad** y haz clic en **Abrir igualmente**.

---

## Comunidad

- [LinuxDO](https://linux.do)

## Licencia

Este proyecto está bajo la licencia [AGPL-3.0](LICENSE).
