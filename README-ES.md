[English](./README-EN.md) | [简体中文](./README.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | **Español** | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

[![AxAgent](https://github.com/polite0803/AxAgent/blob/main/src/assets/image/logo.png?raw=true)](https://github.com/polite0803/AxAgent)

<p align="center">
  <a href="https://www.producthunt.com/products/axagent?embed=true&amp;amp;utm_source=badge-featured&amp;amp;utm_medium=badge&amp;amp;utm_campaign=badge-axagent" target="_blank" rel="noopener noreferrer"><img alt="AxAgent - Lightweight, high-perf cross-platform AI desktop client | Product Hunt" width="250" height="54" src="https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=1118403&amp;theme=light&amp;t=1775627359538"></a>
</p>

<p align="center">
  <strong>Cliente IA multiplataforma de escritorio/móvil | Colaboración multi-agente | Local primero</strong>
</p>

<p align="center">
  <a href="https://github.com/polite0803/AxAgent/releases" target="_blank">
    <img src="https://img.shields.io/github/v/release/polite0803/AxAgent?style=flat-square" alt="Release">
  </a>
  <a href="https://github.com/polite0803/AxAgent/actions" target="_blank">
    <img src="https://img.shields.io/github/actions/workflow/status/polite0803/AxAgent/release.yml?style=flat-square" alt="Build">
  </a>
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux%20%7C%20Android%20%7C%20iOS-blue?style=flat-square" alt="Platform">
  <img src="https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square" alt="License">
</p>

---

## ¿Qué es AxAgent?

**AxAgent v2.0** es una aplicación IA multiplataforma de escritorio/móvil completa, que integra capacidades avanzadas de agentes IA y herramientas de desarrollo ricas. Soporta múltiples proveedores de modelos, ejecución autónoma de pipelines, orquestación visual de flujos de trabajo, gestión local de conocimientos, pasarela API integrada, cubriendo las cinco plataformas **Windows / macOS / Linux / Android / iOS**.

---

## Capturas de pantalla

| Conversación y selección de modelo |       Panel multi-agente        |
| :--------------------------------: | :-----------------------------: |
|  ![](.github/images/s1-0412.png)   | ![](.github/images/s5-0412.png) |

|    Base de conocimientos RAG    |       Memoria y contexto        |
| :-----------------------------: | :-----------------------------: |
| ![](.github/images/s3-0412.png) | ![](.github/images/s4-0412.png) |

|   Editor de flujo de trabajo    |           Pasarela API           |
| :-----------------------------: | :------------------------------: |
| ![](.github/images/s9-0412.png) | ![](.github/images/s10-0412.png) |

---

## Funcionalidades principales

### 🤖 Soporte de modelos IA

- **Soporte multi-proveedor** — Integración nativa de OpenAI, Anthropic Claude, Google Gemini, Ollama, OpenClaw, Hermes y todas las APIs compatibles con OpenAI
- **Rotación multi-clave** — Configure múltiples claves API por proveedor con rotación automática para distribuir la presión de límites de tasa
- **Soporte de modelos locales** — Soporte completo para modelos locales Ollama, incluyendo gestión de archivos GGUF/GGML
- **Motor de inferencia Candle** — Inferencia local Candle integrada, soporte de interfaces rerank/judge, descarga GGUF bajo demanda
- **Gestión de modelos** — Obtención de listas de modelos remotos, personalización de parámetros (temperatura, tokens máximos, top-p, etc.)
- **Salida en streaming** — Renderizado en tiempo real token a token con bloques de pensamiento plegables (pensamiento extendido de Claude)
- **Comparación multi-modelo** — Pregunte a múltiples modelos simultáneamente con comparación lado a lado
- **Llamadas de funciones** — Llamadas de funciones estructuradas en todos los proveedores soportados
- **API Responses de OpenAI** — Soporte para transporte en formato OpenAI Responses
- **API Realtime** — Push de eventos WebSocket compatible con la API Realtime de OpenAI
- **Generación de imágenes IA** — DALL-E 3 y Flux (Replicate), múltiples preajustes de tamaño (1:1/16:9/9:16/4:3), prompts negativos
- **Enrutamiento inteligente de modelos** — Enrutamiento automático por tipo de tarea (revisión de código/resumen/traducción), reglas de enrutamiento personalizadas
- **Llamada de voz** — Conversación por voz en tiempo real vía OpenAI Realtime API, conmutación de estados conectar/hablando/escuchando

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
- **Panel del pool de agentes** — Visualización del estado en tiempo real de sub-agente/Worker/paso de flujo de trabajo
- **Panel de reflexión del agente** — Puntuación de calidad post-tarea, análisis de eficiencia, patrones de error, sugerencias de mejora
- **Selector de expertos** — Importar/exportar/personalizar roles de expertos, filtrado por categoría, preajustes integrados
- **Árbol jerárquico de agentes** — Visualización de la jerarquía de agentes y topología de colaboración
- **Clasificador de intenciones** — Identificación automática del tipo de intención de la entrada del usuario
- **Gestión del estado de creencias** — Mantenimiento del estado de comprensión del contexto del agente
- **Evaluador de objetivos** — Evaluación de la finalización y calidad de los objetivos de tarea
- **Gestión de ventana de contexto** — Gestión inteligente de la ventana de contexto, optimización del uso de tokens
- **Memoria de proyecto** — Persistencia de conocimiento a nivel de proyecto entre sesiones
- **Gestión de base de conocimientos** — Operaciones CRUD de base de conocimientos
- **Sistema de notas** — Almacenamiento y recuperación estructurados de notas dentro de los agentes

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
- **Panel de colaboración** — Gestión de sesiones de colaboración en tiempo real, compartir código de invitación, roles de participantes (Propietario/Editor/Lector)
- **Compartir sesión** — Enlace para compartir con un clic, configuración de permisos de acceso a terminal/archivo/modelo

### ⭐ Sistema de habilidades

- **Mercado de habilidades** — Mercado integrado para explorar e instalar habilidades contribuidas por la comunidad
- **Creación de habilidades** — Creación automática de habilidades a partir de propuestas, con editor Markdown
- **Evolución de habilidades** — Análisis y mejora automáticos impulsados por IA de habilidades existentes basados en retroalimentación de ejecución
- **Panel de evolución de habilidades** — Visualización de la generación de evolución, mejor aptitud promedio, estado de convergencia
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
- **Self-RAG** — Generación aumentada por recuperación automática, determinación inteligente de la necesidad de recuperación y relevancia de resultados
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
- **Sistema de plugins** — Arquitectura de plugins de tres niveles compatible con OpenClaw (integrado/empaquetado/externo), con instalación de paquetes npm, registro de herramientas, hooks y gestión del ciclo de vida
- **Mercado de plugins** — UI de mercado integrada, con búsqueda e instalación npm, diálogos de confirmación
- **Herramientas integradas** — Operaciones de archivos completas (lectura/escritura/edición), ejecución de código, búsqueda (Grep/Glob), Bash, búsqueda web, extracción web, gestión de planes, planificación Cron, REPL, LSP, gestión de contexto, control informático, envío de mensajes, lista de tareas, etc.
- **Sistema de permisos de herramientas** — Clasificación de permisos de herramientas, gestión de reglas y seguimiento de uso
- **Seguridad Bash** — Análisis de comandos, validación de rutas y control de seguridad sandbox
- **Cliente LSP** — Protocolo Language Server integrado, completación de código y diagnósticos
- **Índice AST** — Análisis e indexación AST de archivos de código
- **Backend de terminal** — Soporte para conexiones de terminal locales, Docker y SSH
- **Automatización de navegador** — Control de navegador vía integración CDP (navegación, capturas, clics, relleno, extracción de texto, etc.)
- **Automatización UI** — Identificación y control de elementos UI multiplataforma
- **Herramientas Git** — Operaciones Git con detección de ramas y sensibilidad a conflictos
- **Panel de commit Git** — Estadísticas diff Git visuales, mensajes de commit generados por IA, staging y commit con un clic
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
- **Intérprete de gráficos** — Análisis y visualización de datos de gráficos por IA (barras/líneas/circular/dispersión/área), insights automáticos
- **Visor de diff** — Comparación de versiones de conversación, Aceptar/Rechazar por archivo, detección automática de idioma
- **Barra de clasificación de contexto** — Visualización segmentada del uso de tokens de contexto por categoría
- **Grafo de contexto** — Visualización ReactFlow de relaciones de contexto
- **Sugerencia de comandos** — Sugerencia automática de comandos durante la entrada
- **Gestor de citas** — Seguimiento/clasificación de fuentes de citas con puntuación de credibilidad
- **Insignia de credibilidad** — Visualización de credibilidad de cinco estrellas

### 🛡️ Datos y seguridad

- **Cifrado AES-256** — Claves API y datos sensibles cifrados con AES-256-GCM
- **Almacenamiento aislado** — Estado de la aplicación en `~/.axagent/`, archivos de usuario en `~/Documents/axagent/`
- **Copia de seguridad automática** — Copias de seguridad programadas a directorio local o almacenamiento WebDAV
- **Espacio de trabajo en la nube** — Sincronización de almacenamiento en la nube S3 y WebDAV, detección/resolución de conflictos, sincronización bidireccional
- **Restauración de copia de seguridad** — Restauración en un clic desde copias de seguridad históricas
- **Opciones de exportación** — Capturas PNG, Markdown, texto plano, JSON
- **Gestión de almacenamiento** — Visualización del uso del disco y herramientas de limpieza
- **Autorización de archivos** — Gestión de autorización y revocación de acceso a archivos
- **Auditoría de operaciones** — Registro de auditoría de operaciones críticas

### 🖥️ Experiencia de escritorio

- **Diseño responsivo** — Adaptación automática de tres niveles escritorio/tablet/móvil (puntos de ruptura 600px/900px), conmutación en tiempo real al redimensionar
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
- **Indicador de estado de sueño** — Visualización en tiempo real del estado y resultados de la consolidación de sueños
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

### 🛡️ Protección contra inyección de prompts (Prompt-Guard)

- **Sistema de protección de cuatro niveles** — L1 detección de patrones (intercepción de alto riesgo + marcado de riesgo medio) → L2 escape de delimitadores → L3 envoltorio XML → L4 etiquetas de confianza
- **Orquestador Pipeline** — Canalización de detección multinivel en serie, soporte de umbrales de riesgo personalizables
- **Detección de Token Smuggling** — Detección especializada contra ofuscación de codificación y ataques de contrabando de tokens
- **Modo Strict** — Pruebas en modo estricto + nombramiento de causas de riesgo medio + documentación de modo personalizado
- **Integración completa de pipeline** — Integrado en session / prompt / git / RAG

### 📱 Soporte móvil

- **Android nativo** — Compilación APK/AAB, soporte para arm64-v8a / armeabi-v7a / x86_64
- **iOS nativo** — Compilación IPA, soporte para arm64
- **Diseño adaptativo** — Adaptación automática en tres niveles: escritorio/tableta/teléfono (puntos de ruptura CSS 600px/900px, conmutación en tiempo real al redimensionar la ventana)
- **Navegación móvil** — Navegación Drawer deslizante + barra de navegación inferior + botón flotante flash
- **Adaptación de zona segura** — Adaptación CSS env() para barra de estado/barra de navegación del sistema Android
- **Optimización CSP** — Lista blanca de protocolo CSP para Android WebView

---

## Arquitectura técnica

### Pila de tecnología

| Capa                        | Tecnología                                             |
| --------------------------- | ------------------------------------------------------ |
| **Framework**               | Tauri 2 + React 19 + TypeScript 6                      |
| **UI**                      | Ant Design 6 + TailwindCSS 4                           |
| **Gestión de estado**       | Zustand 5                                              |
| **Enrutamiento**            | React Router 7                                         |
| **i18n**                    | i18next + react-i18next                                |
| **Backend**                 | Rust + SeaORM 2 + SQLite                               |
| **Base de datos vectorial** | sqlite-vec                                             |
| **Editor de código**        | Monaco Editor                                          |
| **Diagramas**               | Mermaid + D2 + ECharts (CDN)                           |
| **Terminal**                | xterm.js 6                                             |
| **Flujo de trabajo**        | ReactFlow 11                                           |
| **Infografías**             | @antv/infographic                                      |
| **Iconos**                  | Iconify + Lucide                                       |
| **Arrastrar y soltar**      | @dnd-kit                                               |
| **Build**                   | Vite 8 + npm                                           |
| **Testing**                 | Vitest + Playwright + cargo-nextest                    |
| **Formateo**                | dprint (TS/JSON) + rustfmt                             |
| **Lint**                    | TS: eslint + oxlint / Rust: clippy + cargo-deny        |
| **Móvil**                   | Compilación nativa Tauri Android + iOS                 |
| **Escritorio**              | Windows (MSI) · macOS (DMG) · Linux (AppImage/deb/rpm) |

### Soporte de plataformas

| Plataforma | Arquitectura                              |
| ---------- | ----------------------------------------- |
| Windows    | x86_64, ARM64                             |
| macOS      | Apple Silicon (arm64), Intel (x86_64)     |
| Linux      | x86_64, ARM64                             |
| Android    | arm64-v8a, armeabi-v7a, x86_64 (emulador) |
| iOS        | arm64                                     |

### Arquitectura Backend Rust

El backend está organizado como un workspace de Rust con **18** crates especializadas:

```
src-tauri/crates/
├── agent/            # Núcleo del agente IA (motor ReAct, coordinación, planificación, investigación profunda, verificación de hechos, etc.)
├── core/             # Herramientas principales (base de datos, RAG, cifrado, MCP, automatización de navegador, índice AST, etc.)
├── providers/        # Adaptadores de proveedores de modelos (OpenAI, Anthropic, Gemini, Ollama, OpenClaw, etc.)
├── runtime-core/     # Capa de abstracción del runtime (tipos comunes, definiciones de traits, configuración)
├── runtime/          # Servicios del runtime (gestión de sesiones, MCP, terminal, limitador de tasa, Webhook, permisos, etc.)
├── rt-workflow/      # Motor de flujo de trabajo (orquestación DAG, ejecutores de nodos, planificador)
├── rt-messaging/     # Pasarela de mensajería (integración DingTalk/Feishu/QQ/Slack/WeChat/WhatsApp/Telegram/Discord)
├── rt-webhook/       # Servidor y despacho de Webhooks
├── rt-dashboard/     # Sistema de plugins del panel
├── rt-theme/         # Motor de temas
├── gateway/          # Pasarela API (servidor HTTP, autenticación, rutas, interfaz compatible con OpenAI)
├── tools/            # Sistema de herramientas (registro, orquestación, salida en streaming, 40+ herramientas integradas)
├── trajectory/       # Sistema de aprendizaje (memoria, habilidades, RL, perfil de usuario, integración onírica)
├── telemetry/        # Telemetría y rastreo distribuido
├── plugins/          # Sistema de plugins (compatible con OpenClaw, instalación de paquetes npm)
├── prompt-guard/     # Protección contra inyección de prompts (detección y defensa multinivel L1-L4)
├── migration/        # Migraciones de base de datos
├── npm/              # Análisis de paquetes npm y registro
└── code_engine/      # Motor de inferencia local Candle (obsoleto, funcionalidad integrada en core)
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
│   ├── feature/               # Estado de módulos funcionales (44 stores)
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
npm run test          # Vitest watch
npm run test:run      # Vitest ejecución única

# Pruebas E2E
npm run test:e2e      # Playwright
npm run test:e2e:ui   # Playwright modo UI

# Pruebas del backend Rust
cd src-tauri && cargo nextest run   # cargo-nextest (2-3x más rápido)
cd src-tauri && cargo test          # Pruebas estándar

# Verificación de tipos
npm run typecheck     # TypeScript
cd src-tauri && cargo clippy -- -D warnings  # Rust

# Formateo de código
npm run format        # dprint
cd src-tauri && cargo fmt

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
│   ├── stores/                  # Gestión de estado Zustand (62 stores)
│   │   ├── domain/            # Estado de negocio principal (9 stores)
│   │   ├── feature/           # Estado de módulos funcionales (44 stores)
│   │   ├── devtools/          # Estado de herramientas de desarrollo (5 stores)
│   │   └── shared/            # Estado compartido (4 stores)
│   ├── hooks/                   # React hooks
│   ├── lib/                     # Funciones utilitarias (incluyendo Web Worker)
│   ├── types/                   # Definiciones de tipos TypeScript
│   ├── sdk/                     # SDK (incluyendo SDK Python)
│   └── i18n/                    # Traducciones en 11 idiomas
│
├── src-tauri/                    # Código fuente backend (Rust)
│   ├── crates/                  # Workspace Rust (18 crates)
│   │   ├── agent/             # Núcleo del agente IA
│   │   ├── core/              # Base de datos, cifrado, RAG, MCP
│   │   ├── providers/         # Adaptadores de proveedores de modelos
│   │   ├── runtime-core/      # Capa de abstracción del runtime
│   │   ├── runtime/           # Servicios del runtime
│   │   ├── rt-workflow/       # Motor de flujo de trabajo
│   │   ├── rt-messaging/      # Pasarela de mensajería
│   │   ├── rt-webhook/        # Servidor de Webhooks
│   │   ├── rt-dashboard/      # Plugin de panel
│   │   ├── rt-theme/          # Motor de temas
│   │   ├── gateway/           # Servidor pasarela API
│   │   ├── tools/             # Sistema de herramientas
│   │   ├── trajectory/        # Memoria y aprendizaje
│   │   ├── telemetry/         # Rastreo y métricas
│   │   ├── plugins/           # Sistema de plugins
│   │   ├── prompt-guard/      # Protección contra inyección de prompts
│   │   ├── migration/         # Migraciones de base de datos
│   │   └── npm/               # Análisis de paquetes npm
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
