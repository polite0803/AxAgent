---
layout: home
title: AxAgent — Cliente de escritorio IA open-source & Pasarela
titleTemplate: false

head:
  - - meta
    - name: description
      content: AxAgent es un cliente de escritorio IA gratuito y de código abierto con pasarela IA integrada. Conecta OpenAI, Claude, Gemini, DeepSeek y más LLMs. Soporte para servidor MCP, base de conocimiento, privacidad primero.

hero:
  name: AxAgent
  text: Tu asistente IA de escritorio
  tagline: Chat multi-modelo, herramientas MCP, pasarela API, base de conocimiento — todo en un solo cliente open-source
  image:
    src: /logo.png
    alt: AxAgent
  actions:
    - theme: brand
      text: Comenzar
      link: /es/guide/getting-started
    - theme: alt
      text: Descargar
      link: /es/download
    - theme: alt
      text: GitHub
      link: https://github.com/polite0803/AxAgent

features:
  - icon: robot
    title: Chat multi-modelo
    details: Conéctate a OpenAI, Claude, Gemini, DeepSeek, Qwen y cualquier API compatible. Rotación de claves múltiples, salida en streaming, bloques de razonamiento.
  - icon: thunderbolt
    title: AI Agent (Beta)
    details: Modo Agent para ejecución autónoma de tareas. Leer/escribir archivos, ejecutar comandos, analizar código — tres niveles de permisos + sandbox de directorio de trabajo.
  - icon: api
    title: Llamada de herramientas MCP
    details: Implementación completa del Model Context Protocol. Soporte para stdio, SSE, StreamableHTTP. Conecta herramientas externas con un clic. Herramientas MCP integradas como @axagent/fetch.
  - icon: cloud-server
    title: Pasarela API integrada
    details: Servidor API local compatible con OpenAI. Usa AxAgent como backend para Claude Code, Codex, Gemini CLI y más. Gestión de claves, limitación de velocidad, SSL/TLS.
  - icon: book
    title: Base de conocimiento & RAG
    details: Embeddings vectoriales locales con sqlite-vec. Respuestas IA basadas en tus documentos privados — los datos nunca abandonan tu máquina.
  - icon: search
    title: Búsqueda web
    details: Integración con Tavily, Zhipu WebSearch, Bocha. Resultados de búsqueda con fuentes de citas inyectados en el contexto de conversación.
  - icon: edit
    title: Renderizado de contenido rico
    details: Markdown, LaTeX, diagramas Mermaid, diagramas de arquitectura D2, editor Monaco con vista previa diff, paneles Artifact.
  - icon: desktop
    title: Experiencia de escritorio
    details: Atajos globales, bandeja del sistema, inicio automático, siempre visible, temas oscuro/claro, soporte de proxy.
  - icon: lock
    title: Privacidad & Seguridad
    details: Todos los datos almacenados localmente. Claves API cifradas con AES-256. Copia de seguridad automática local o WebDAV. Exportación de conversación como PNG/Markdown/JSON.
---
