---
layout: home
title: AxAgent — Настольный ИИ-клиент с открытым исходным кодом & Шлюз
titleTemplate: false

head:
  - - meta
    - name: description
      content: AxAgent — бесплатный настольный ИИ-клиент с открытым исходным кодом и встроенным ИИ-шлюзом. Подключайте OpenAI, Claude, Gemini, DeepSeek и другие LLM. Поддержка MCP-серверов, база знаний, приоритет конфиденциальности.

hero:
  name: AxAgent
  text: Ваш ИИ-ассистент для рабочего стола
  tagline: Мультимодельный чат, инструменты MCP, API-шлюз, база знаний — всё в одном клиенте с открытым исходным кодом
  image:
    src: /logo.png
    alt: AxAgent
  actions:
    - theme: brand
      text: Начать
      link: /ru/guide/getting-started
    - theme: alt
      text: Скачать
      link: /ru/download
    - theme: alt
      text: GitHub
      link: https://github.com/polite0803/AxAgent

features:
  - icon: robot
    title: Мультимодельный чат
    details: Подключайтесь к OpenAI, Claude, Gemini, DeepSeek, Qwen и любым совместимым API. Ротация нескольких ключей, потоковый вывод, блоки размышлений.
  - icon: thunderbolt
    title: AI Agent (Beta)
    details: Режим Agent для автономного выполнения задач. Чтение/запись файлов, выполнение команд, анализ кода — три уровня разрешений + песочница рабочего каталога.
  - icon: api
    title: Вызов инструментов MCP
    details: Полная реализация Model Context Protocol. Поддержка stdio, SSE, StreamableHTTP. Подключение внешних инструментов одним кликом. Встроенные MCP-инструменты, такие как @axagent/fetch.
  - icon: cloud-server
    title: Встроенный API-шлюз
    details: Локальный API-сервер, совместимый с OpenAI. Используйте AxAgent как бэкенд для Claude Code, Codex, Gemini CLI и других. Управление ключами, ограничение частоты запросов, SSL/TLS.
  - icon: book
    title: База знаний & RAG
    details: Локальные векторные эмбеддинги с sqlite-vec. Ответы ИИ на основе ваших приватных документов — данные никогда не покидают вашу машину.
  - icon: search
    title: Веб-поиск
    details: Интеграция с Tavily, Zhipu WebSearch, Bocha. Результаты поиска с источниками цитирования внедряются в контекст разговора.
  - icon: edit
    title: Богатый рендеринг контента
    details: Markdown, LaTeX, диаграммы Mermaid, архитектурные диаграммы D2, редактор кода Monaco с предпросмотром diff, панели Artifact.
  - icon: desktop
    title: Опыт рабочего стола
    details: Глобальные горячие клавиши, системный трей, автозапуск, поверх всех окон, тёмная/светлая темы, поддержка прокси.
  - icon: lock
    title: Конфиденциальность & Безопасность
    details: Все данные хранятся локально. Ключи API зашифрованы AES-256. Автоматическое резервное копирование локально или на WebDAV. Экспорт разговора в PNG/Markdown/JSON.
---
