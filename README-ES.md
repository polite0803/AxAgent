[简体中文](./README.md) | [繁體中文](./README-ZH-TW.md) | [English](./README-EN.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | **Español** | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

[![AxAgent](https://github.com/polite0803/AxAgent/blob/main/src/assets/image/logo.png?raw=true)](https://github.com/polite0803/AxAgent)

<p align="center">
    <a href="https://www.producthunt.com/products/axagent?embed=true&amp;utm_source=badge-featured&amp;utm_medium=badge&amp;utm_campaign=badge-axagent" target="_blank" rel="noopener noreferrer"><img alt="AxAgent - Lightweight, high-perf cross-platform AI desktop client | Product Hunt" width="250" height="54" src="https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=1118403&amp;theme=light&amp;t=1775627359538"></a>
</p>

## Capturas de pantalla

| Renderizado de gráficos de chat | Proveedores y modelos |
|:---:|:---:|
| ![](.github/images/s1-0412.png) | ![](.github/images/s2-0412.png) |

| Base de conocimientos | Memoria |
|:---:|:---:|
| ![](.github/images/s3-0412.png) | ![](.github/images/s4-0412.png) |

| Agent - Consulta | Acceso rápido a API Gateway |
|:---:|:---:|
| ![](.github/images/s5-0412.png) | ![](.github/images/s6-0412.png) |

| Selección de modelo de chat | Navegación de chats |
|:---:|:---:|
| ![](.github/images/s7-0412.png) | ![](.github/images/s8-0412.png) |

| Agent - Aprobación de permisos | Resumen de API Gateway |
|:---:|:---:|
| ![](.github/images/s9-0412.png) | ![](.github/images/s10-0412.png) |

## Características

### Chat y modelos

- **Soporte multi-proveedor** — Compatible con OpenAI, Anthropic Claude, Google Gemini y todas las APIs compatibles con OpenAI
- **Gestión de modelos** — Obtener listas de modelos remotos, personalizar parámetros (temperatura, tokens máximos, Top-P, etc.)
- **Rotación multi-clave** — Configurar múltiples claves API por proveedor con rotación automática para distribuir la presión del límite de velocidad
- **Salida en streaming** — Renderizado en tiempo real token a token con bloques de pensamiento plegables
- **Versiones de mensajes** — Cambiar entre múltiples versiones de respuesta por mensaje para comparar efectos de modelos o parámetros
- **Ramificación de conversación** — Crear nuevas ramas desde cualquier nodo de mensaje, con comparación de ramas en paralelo
- **Gestión de conversaciones** — Fijar, archivar, visualización agrupada por tiempo y operaciones masivas
- **Compresión de conversación** — Comprimir automáticamente conversaciones largas preservando información clave para ahorrar espacio de contexto
- **Respuesta simultánea multi-modelo** — Hacer la misma pregunta a múltiples modelos a la vez, con comparación de respuestas en paralelo

### AI Agent

- **Soporte multi-proveedor** — Compatible con OpenAI, Anthropic Claude, Google Gemini y todas las APIs compatibles con OpenAI, con soporte para Ollama para modelos locales y puertas de enlace remotas como OpenClaw/Hermes
- **Modo Agent** — Cambia al modo Agent para la ejecución autónoma de tareas de múltiples pasos: leer/escribir archivos, ejecutar comandos, analizar código y más
- **Tres niveles de permisos** — Predeterminado (escrituras necesitan aprobación), Aceptar ediciones (aprobar cambios de archivos automáticamente), Acceso completo (sin indicaciones) — seguro y controlable
- **Sandbox de directorio de trabajo** — Las operaciones del Agent están estrictamente limitadas al directorio de trabajo especificado, previniendo el acceso no autorizado
- **Panel de aprobación de herramientas** — Visualización en tiempo real de solicitudes de llamadas a herramientas con revisión individual, "permitir siempre" con un clic, o denegar
- **Seguimiento de costos** — Estadísticas de uso de tokens y costos en tiempo real por sesión

### Sistema multi-agente

- **Coordinación de sub-agentes** — Crea múltiples sub-agentes formando una arquitectura de coordinación maestro-esclavo
- **Ejecución paralela** — Procesa múltiples agentes en paralelo para mayor eficiencia en tareas complejas
- **Debate adversario** — Múltiples agentes debaten diferentes puntos de vista para producir mejores soluciones a través de la colisión de ideas
- **Motor de flujo de trabajo** — Orquestación potente de flujos de trabajo que soporta ramificaciones condicionales, bucles y ejecución paralela
- **Roles de equipo** — Asigna roles específicos a diferentes agentes (revisión de código, pruebas, documentación, etc.) para completar tareas colaborativas

### Sistema de habilidades

- **Mercado de habilidades** — Mercado de habilidades integrado para explorar e instalar habilidades contribuidas por la comunidad
- **Creación de habilidades** — Crea habilidades a partir de propuestas con editor Markdown
- **Evolución de habilidades** — La IA analiza y mejora automáticamente las habilidades existentes para un mejor rendimiento
- **Coincidencia de habilidades** — Recomendación inteligente de habilidades relevantes, aplicadas automáticamente a escenarios de conversación apropiados
- **Registro de habilidades locales** — Soporte para herramientas personalizadas locales registradas como habilidades
- **Hooks de plugin** — Soporte para hooks pre/post para inyectar lógica personalizada antes/después de la ejecución de habilidades
- **Habilidades atómicas** — Componentes de habilidades granulados que admiten la construcción de flujos de trabajo complejos
- **Descomposición de habilidades** — Descompone automáticamente tareas complejas en habilidades atómicas ejecutables
- **Herramientas generadas** — La IA genera y registra automáticamente nuevas herramientas para expandir las capacidades del agente

### Sistema de flujo de trabajo

- **Editor de flujo de trabajo** — Diseñador visual de flujos de trabajo por arrastrar y soltar con conexión y configuración de nodos
- **Plantillas de flujo de trabajo** — Preajustes integrados para iniciar rápidamente tareas comunes
- **Gestión de versiones** — Las plantillas de flujo de trabajo admiten la gestión de versiones con retroceso a versiones históricas
- **Motor de flujo de trabajo** — Potente motor de ejecución de flujos de trabajo que admite ejecución paralela, condicional y en bucle
- **Historial de ejecución** — Registro detallado del historial de ejecución del flujo de trabajo con seguimiento de estado y depuración
- **Asistencia IA** — Asistencia IA para el diseño de flujos de trabajo, generación y optimización automática

### Renderizado de contenido

- **Renderizado Markdown** — Soporte completo para resaltado de código, fórmulas matemáticas LaTeX, tablas y listas de tareas
- **Editor de código Monaco** — Monaco Editor integrado en bloques de código con resaltado de sintaxis, copia y vista previa diff
- **Renderizado de diagramas** — Renderizado integrado de diagramas de flujo Mermaid y diagramas de arquitectura D2
- **Panel Artifact** — Fragmentos de código, borradores HTML, notas Markdown e informes visualizables en un panel dedicado
- **Inspector de sesión** — Visualización en tiempo real de la estructura de sesión como vista de árbol para navegación rápida a cualquier mensaje

### Búsqueda y conocimiento

- **Búsqueda web** — Integrado con Tavily, Zhipu WebSearch, Bocha y más, con anotaciones de fuentes de cita
- **Base de conocimiento local (RAG)** — Soporta múltiples bases de conocimiento; cargar documentos para análisis automático, fragmentación e indexación vectorial, con recuperación semántica de pasajes relevantes durante las conversaciones
- **Grafo de conocimiento** — Grafo de relaciones entidad-conocimiento que visualiza las conexiones entre puntos de conocimiento
- **Sistema de memoria** — Memoria multi-espacio de nombres con entrada manual o extracción automática de información clave por IA
- **Búsqueda de texto completo** — Motor FTS5 para búsqueda rápida en conversaciones, archivos y memorias
- **Gestión de contexto** — Adjuntar de forma flexible archivos adjuntos, resultados de búsqueda, pasajes de la base de conocimiento, entradas de memoria y salidas de herramientas

### Herramientas y extensiones

- **Protocolo MCP** — Implementación completa del Model Context Protocol con soporte para transportes stdio y HTTP/WebSocket
- **Autenticación OAuth** — Soporte de flujo de autenticación OAuth para servidores MCP
- **Herramientas integradas** — Herramientas integradas listas para usar para operaciones de archivos, ejecución de código, búsqueda y más
- **Panel de ejecución de herramientas** — Visualización de solicitudes de llamadas a herramientas y resultados devueltos
- **Cliente LSP** — Soporte integrado del protocolo LSP para finalización de código inteligente y diagnóstico

### Pasarela API

- **Pasarela API local** — Servidor API local integrado con soporte nativo para interfaces compatibles con OpenAI, Claude y Gemini
- **Enlaces externos** — Integración con un clic con herramientas externas como Claude CLI y OpenCode con sincronización automática de claves API
- **Gestión de claves API** — Generar, revocar y habilitar/deshabilitar claves de acceso con notas descriptivas
- **Análisis de uso** — Análisis de volumen de solicitudes y uso de tokens por clave, proveedor y fecha
- **Herramientas de diagnóstico** — Verificaciones de salud de la pasarela, pruebas de conexión y depuración de solicitudes
- **Soporte SSL/TLS** — Generación integrada de certificados autofirmados, con soporte para certificados personalizados
- **Registros de solicitudes** — Registro completo de todas las solicitudes y respuestas de la API que pasan por la pasarela
- **Plantillas de configuración** — Plantillas de integración prediseñadas para herramientas CLI populares como Claude, Codex, OpenCode y Gemini
- **Comunicación en tiempo real** — Push de eventos WebSocket en tiempo real, compatible con la API OpenAI Realtime

### Datos y seguridad

- **Cifrado AES-256** — Las claves API y los datos sensibles se cifran localmente con AES-256-GCM
- **Directorios de datos aislados** — Estado de la aplicación en `~/.axagent/`; archivos de usuario en `~/Documents/axagent/`
- **Copia de seguridad automática** — Copias de seguridad automáticas programadas en directorios locales o almacenamiento WebDAV
- **Restauración de copia de seguridad** — Restauración con un clic desde copias de seguridad históricas
- **Exportación de conversación** — Exportar conversaciones como capturas PNG, Markdown, texto plano o JSON
- **Gestión de espacio de almacenamiento** — Visualización del uso del disco con limpieza de archivos innecesarios

### Experiencia de escritorio

- **Cambio de tema** — Temas oscuro/claro que siguen las preferencias del sistema o se pueden configurar manualmente
- **Idioma de la interfaz** — Soporte completo para chino simplificado, chino tradicional, inglés, japonés, coreano, francés, alemán, español, ruso, hindi y árabe
- **Bandeja del sistema** — Minimizar a la bandeja del sistema al cerrar la ventana sin interrumpir los servicios en segundo plano
- **Siempre visible** — Fijar la ventana principal para que permanezca sobre todas las demás ventanas
- **Atajos globales** — Atajos de teclado globales personalizables para invocar la ventana principal en cualquier momento
- **Inicio automático** — Lanzamiento opcional al iniciar el sistema
- **Soporte de proxy** — Configuración de proxy HTTP y SOCKS5
- **Actualización automática** — Verifica automáticamente nuevas versiones al inicio y solicita actualización
- **Paleta de comandos** — `Cmd/Ctrl+K` para acceso rápido a todos los comandos y configuración

## Plataformas compatibles

| Plataforma | Arquitectura |
|------------|-------------|
| macOS | Apple Silicon (arm64), Intel (x86_64) |
| Windows 10/11 | x86_64, arm64 |
| Linux | x86_64 (AppImage/deb/rpm), arm64 (AppImage/deb/rpm) |

## Primeros pasos

Ve a la página de [Releases](https://github.com/polite0803/AxAgent/releases) y descarga el instalador para tu plataforma.

## Compilar desde el código fuente

### Requisitos previos

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+
- [npm](https://www.npmjs.com/) 10+
- Windows requiere [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) y [Rust MSVC targets](https://doc.rust-lang.org/cargo/reference/config.html#cfgtarget)

### Pasos de compilación

```bash
# Clonar el repositorio
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent

# Instalar dependencias
npm install

# Ejecutar en modo desarrollo
npm run tauri dev

# Compilar solo el frontend
npm run build

# Compilar la aplicación de escritorio
npm run tauri build
```

Los artefactos de compilación están en el directorio `src-tauri/target/release/`.

### Pruebas

```bash
# Ejecutar pruebas unitarias
npm test

# Ejecutar pruebas end-to-end
npm run test:e2e

# Verificación de tipos
npm run typecheck
```

## Preguntas frecuentes

### macOS: «La app está dañada» o «No se puede verificar al desarrollador»

Dado que la aplicación no está firmada por Apple, macOS puede mostrar uno de los siguientes mensajes:

- «AxAgent» está dañado y no se puede abrir
- «AxAgent» no se puede abrir porque Apple no puede comprobar si contiene software malicioso

**Pasos para resolver el problema:**

**1. Permitir apps de «Cualquier origen»**

```bash
sudo spctl --master-disable
```

Luego ve a **Configuración del sistema → Privacidad y seguridad → Seguridad** y selecciona **Cualquier origen**.

**2. Eliminar el atributo de cuarentena**

```bash
sudo xattr -dr com.apple.quarantine /Applications/AxAgent.app
```

> Consejo: Puedes arrastrar el ícono de la app al terminal después de escribir `sudo xattr -dr com.apple.quarantine `.

**3. Paso adicional para macOS Ventura y versiones posteriores**

Después de completar los pasos anteriores, es posible que el primer lanzamiento aún esté bloqueado. Ve a **Configuración del sistema → Privacidad y seguridad** y haz clic en **Abrir igualmente** en la sección de Seguridad. Esto solo debe hacerse una vez.

## Comunidad
- [LinuxDO](https://linux.do)

## Licencia

Este proyecto está bajo la licencia [AGPL-3.0](LICENSE).