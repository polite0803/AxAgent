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

**AxAgent** ist ein plattformübergreifender KI-Desktop-Client auf Basis von Tauri 2 (Windows / macOS / Linux / Android / iOS), positioniert als KI-gesteuerte Arbeitsstation für tägliche Entwicklung, Forschung, Wissensmanagement und Automatisierung. Es integriert eine ReAct-Agent-Engine, kognitives Routing (dreistufiges hierarchisches Routing + Retrieval-Augmented Routing RAR), visuelle Workflow-Orchestrierung, lokale RAG-Wissensdatenbanken, MCP-Protokollerweiterungen, ein vereinheitlichtes Multi-Modell-Gateway, Browser-Automatisierung und Computersteuerung — und bringt die KI von der „Konversation" zur „Ausführung".

> **Sprachen**: [简体中文](./README.md) | [English](./README-EN.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

---

## Projektpositionierung

AxAgent löst drei Kernprobleme:

1. **Vereinheitlichter Multi-Modell-Zugriff und intelligentes Routing** — Eine einzige Oberfläche für OpenAI, Anthropic Claude, Google Gemini, DeepSeek, Qwen, GLM, Kimi, Wenxin, lokale Ollama-Modelle und beliebige OpenAI-kompatible APIs, mit automatischer Rotation mehrerer API-Schlüssel nach Kontingent, aufgabentypbasiertem intelligentem Routing und Streaming-Vergleich
2. **Geschlossener Kreislauf der KI von der Konversation zur Ausführung** — 163+ integrierte Werkzeuge + visuelle Workflows + MCP-Erweiterungen + Browser-/Computersteuerung: Die KI kann Dateien bearbeiten, Code ausführen, Git verwalten und Aufgaben planen
3. **Local-First-Datenhoheit** — Konversationen, Wissensdatenbanken, Speicher und Konfiguration werden in einer lokalen SQLite-Datenbank gespeichert; API-Schlüssel werden mit AES-256-GCM verschlüsselt. Die Kernfunktionen laufen ohne Cloud-Dienste von Drittanbietern

---

## Kernfähigkeiten

### Kognitives Routing-System (Cognitive Router)

AxAgent nutzt `cognitive_query` als einheitlichen Einstiegspunkt für alle Konversationen und bildet Benutzerabsichten über **dreistufiges hierarchisches Routing** auf konkrete Fähigkeiten ab:

- **L1-Domänen-Routing** (`domain_router`): Regeln + LLM-Fallback, erkennt 9 Geschäftsdomänen (Datenanalyse / Content-Erstellung / Kommunikation / Betrieb / KI-Medien / Finanzen / Automatisierung / Allgemein usw.)
- **L2-Cluster-Routing** (`cluster_router`): Lokalisiert Fähigkeitscluster innerhalb der Domäne (27 Cluster, die 8 Geschäftsdomänen abdecken)
- **L3-Fähigkeits-Routing**: **Retrieval-Augmented Routing (RAR)** — ruft die Top-K ähnlichen Workflows aus dem Fähigkeits-Vektorindex ab und injiziert sie in den Prompt, kombiniert mit der Pfadsuche im Workflow-DAG; gibt Pfadadressen (z. B. `/finance/stock_analysis/tech`) und Ausführungsmodi aus
- **Ausführungsmodi**: `Ask / Plan / Act / Workflow / Direct / Delegate / ParameterExtract / Clarify`, automatische Auswahl anhand der Konfidenz
- **Fähigkeitensystem**: Einheitliche Registrierung (`CapabilityRegistry`) + Vektorindex (`CapabilityIndexer`) + hybride Suche (`CapabilityRetriever`, Vektor + BM25 + exaktes Tag-Matching + Ausschluss negativer Stichproben)
- **Isolierung der Systemfähigkeiten**: Der kognitive Orchestrator ist physisch von Geschäfts-Workflows getrennt; Systemfähigkeiten tragen das Sichtbarkeitskennzeichen `SYSTEM_ONLY`; die Routing-Ebene verfügt über eine integrierte Selbstreferenz-Unterbrechung, um Selbstreferenz-Paradoxien zu verhindern
- **Dreistufiges Routing als Workflow-DAG umgesetzt**: 4 vordefinierte Routing-Workflow-Vorlagen (Hauptorchestrierung mit ~20 Knoten + L1/L2/L3-Unterrouting), ausgeführt von der `rt-workflow`-Engine

### Multi-Modell-Engine

- **13 Provider-Adapter**: OpenAI (Chat Completions + Responses + Realtime), Anthropic Claude, Google Gemini, DeepSeek, Qwen, GLM, Kimi, Wenxin Yiyan, Ollama, Llama.cpp (lokale GGUF-Modelle), OpenClaw, Hermes sowie alle OpenAI-kompatiblen APIs
- **Multi-Key-Rotation**: Mehrere API-Schlüssel pro Provider mit kontingentbasierter automatischer Rotation; automatisches Umschalten bei Ratenbegrenzung eines einzelnen Schlüssels
- **Intelligentes Routing**: Automatische Auswahl des optimalen Modells nach Aufgabentyp (Code-Review / Zusammenfassung / Übersetzung / Allgemein), mit anpassbaren Regeln
- **Provider-Gesundheitsüberwachung**: Echtzeitverfolgung von Erfolgsrate, Latenz und Verfügbarkeit, mit gestaffeltem automatischem Fallback
- **KI-Bildgenerierung**: DALL-E 3 und Flux mit Mehrgrößen-Voreinstellungen
- **Echtzeit-Sprache**: WebSocket-basierte Sprachkonversation über die OpenAI Realtime API, mit Unterbrechungs- und Streaming-Transkriptionsunterstützung

### Agent-System (ReAct-Engine)

- **Hierarchischer Planer** (`hierarchical_planner`): Zerlegt komplexe Aufgaben in strukturierte Phase → Task-Pläne, kompiliert zur DAG-topologischen Ausführung
- **Tiefenrecherche** (`deep_research`): Orchestrierung von Multi-Quellen-Suchen mit Suchplanung, Suchausführung, Inhaltsynthese und Zitationsverfolgung
- **Faktenprüfung** (`fact_checker`): KI-gestützte Faktenverifizierung mit Quellenklassifikator und Glaubwürdigkeitsbewertung
- **Gedankenbaum** (`tree_of_thoughts`): Multi-Pfad-Argumentationserkundung mit Zweigbewertung und Backtracking
- **Reflektor** (`reflector`): Selbstbewertung nach der Aufgabenausführung mit Verbesserungsvorschlägen
- **Selbstverifizierung** (`self_verifier`): Automatische Validierung von Argumentationsergebnissen, einschließlich Zykluserkennung
- **Fehlerwiederherstellung** (`error_recovery_engine`): Fehlertypklassifizierung → Auswahl der Wiederherstellungsstrategie → automatischer Wiederholungsversuch oder Plananpassung, mit exponentiellem Backoff
- **A/B-Tests** (`ab_testing`): Vergleichende Bewertung verschiedener Argumentationsstrategien
- **Bewertungssystem** (`evaluator`): Integriertes Benchmark-Framework
- **LoRA-Fine-Tuning** (`fine_tune`): Integrierte Trainingspipeline mit Verwaltung von LoRA-Adaptern
- **RL-Optimierer** (`rl_optimizer`): Erfahrungsbasiertes Policy-Reinforcement-Learning

**Multi-Agent-Zusammenarbeit**:

- Master-Slave-Koordinationsarchitektur, parallele Ausführung von Unteragenten, abhängigkeitsbewusste Planung
- Gemeinsames Blackboard für den Informationsaustausch zwischen Agenten
- Adversarialer Debattenmodus (Pro/Contra-Runden mit Bewertung der Argumentstärke)
- Swarm-Clustermodus für Multi-Prozess-Agentencluster
- Proaktiver Modus: Agenten können proaktiv Vorschläge und Aktionen initiieren

**Computersteuerung**: KI-gesteuerte Mausklicks, Tastatureingabe und Bildschirmscrollen, mit drei Berechtigungsstufen (Standard / Änderungen akzeptieren / Vollzugriff) und Sandbox-Pfadisolierung

**Browser-Automatisierung**: Browsersteuerung über das CDP-Protokoll mit Navigation, Screenshots, Klicks, Formularausfüllung und Textextraktion

### Fähigkeitensystem

- **Fähigkeitsmarktplatz**: Community-Fähigkeiten durchsuchen und installieren
- **KI-gestützte Erstellung**: Automatische Erstellung von Fähigkeitsstrukturen aus natürlichsprachlichen Vorschlägen (`skill:create`)
- **Fähigkeitsevolution** (`evolution_engine`): Automatische Analyse und Verbesserung von Fähigkeiten auf Basis von Ausführungsfeedback
- **Semantisches Matching**: Automatische Empfehlung relevanter Fähigkeiten anhand der Semantik des Konversationskontexts
- **Fähigkeitszerlegung** (`skill_decomposition`): Automatische Zerlegung komplexer Aufgaben in atomare Fähigkeitskombinationen
- **Generierte Werkzeuge**: Von der KI generierte und registrierte neue Werkzeuge
- **Sandbox-Ausführung**: Fähigkeiten werden sicher in isolierten Sandboxes ausgeführt

### Visueller Workflow

Drag-and-Drop-DAG-Workflow-Editor auf Basis von ReactFlow 12:

- **32 Knotentypen**: Auslöser, Agent, LLM-Aufruf, Bedingte Verzweigung, Paralleler Fork, Schleife, Zusammenführung, Verzögerung, Werkzeugaufruf, Codeausführung, Unterworkflow, Vektorsuche, Dokumentanalyse, Validierung, Ende, HTTP-Anfrage, Switch, Datenbankabfrage, Benachrichtigung, Genehmigung, Dateioperation, Datentransformation, Webhook-Senden, Logging, LLM-Klassifikator, Aggregator, E-Mail, Debatte, Swarm, Multi-Agent, Speicher, Geschäftsregel
- **Kahn-Topologische-Sortierungsausführung**: Automatische Erkennung zyklischer Abhängigkeiten, parallele Pipeline-Planung
- **Integrierte Vorlagen**: Code-Review, Bug-Fix, Dokumentgenerierung, Tests, Refactoring, Exploration, Leistungsanalyse, Sicherheitsaudit, Feature-Entwicklung
- **YAML-Serialisierung**: Import/Export von Workflow-Definitionen
- **Versionsverwaltung**: Versionskontrolle von Vorlagen
- **KI-gestütztes Design**: KI-unterstütztes Workflow-Design, Knotenempfehlung und -diagnose

### Wissensmanagement

- **Multi-Wissensdatenbank-RAG**: Dokument-Upload → automatische Analyse (PDF/DOCX/XLSX/PPTX/TXT) → Chunking → Vektorindizierung
- **Hybride Suche**: Vektorähnlichkeit (sqlite-vec + lokale candle-Embeddings) + BM25-Volltextsuche (FTS5), hybrides Ranking
- **Self-RAG**: Automatische Reflexion und Validierung der Suchergebnisse
- **Re-Ranking**: Neusortierung der Ergebnisse mit Cross-Encoder
- **Wissensgraph**: Entitätsextraktion → Beziehungsaufbau → visueller Graph
- **Dateiüberwachung**: Echtzeitüberwachung von Dateiänderungen auf Basis von `notify`, automatische inkrementelle Indizierung
- **LLM Wiki**: KI-gestützter Wiki-Compiler und -Validator

### Speichersystem

- **Multi-Namespace-Speicher**: Isolierung nach Projekt/Thema, mit manueller Eingabe und automatischer KI-Extraktion
- **Persistente Integration**: Closed-Loop-Speicher mit Honcho und Mem0
- **Benutzerprofil**: Automatisches Erlernen von Codestil, Technologie-Stack-Präferenzen und Kommunikationsstil
- **Stilübertragung**: Extraktion von Codestilmerkmalen → Anwendung auf KI-generierten Code
- **Dream-Integration**: Automatische Konsolidierung von Speicherfragmenten und Verhaltensmustern im Hintergrund, um strukturiertes Wissen zu erzeugen
- **Projektspeicher**: Kontextpersistenz auf Projektebene

### API-Gateway

Integriertes HTTP + WebSocket-Gateway auf Basis von `axum`:

- **Kompatible Endpunkte**: OpenAI `/v1/chat/completions`, Claude Messages API, Gemini API sowie OpenAI Responses und Realtime WebSocket
- **Schlüsselverwaltung**: Generierung, Widerruf, Aktivierung/Deaktivierung von Zugriffsschlüsseln, mit Unterstützung für Ablaufzeiten
- **Nutzungsverfolgung**: Statistik von Anfragen und Token-Verbrauch nach Schlüssel/Provider/Datum, Prometheus-Metriken-Export
- **Ratenbegrenzung**: Token-Bucket-Algorithmus auf Basis von `governor`
- **SSL/TLS**: Integrierte selbstsignierte Zertifikate (`rcgen`), mit Unterstützung für benutzerdefinierte Zertifikate
- **Externe Verknüpfung**: Ein-Klick-Integration externer Werkzeuge wie Claude CLI und OpenCode, mit automatischer API-Schlüssel-Synchronisation
- **Echtzeit-Tickets**: HMAC-basierte temporäre Authentifizierungstickets für die sichere Übergabe von WebSocket-Verbindungen
- **Server-Modus**: Optionale `axagent-server`-Binärdatei, die die Fähigkeiten der Desktop-Anwendung als Dienst bereitstellt

### Messaging-Plattform-Integration

Über `rt-messaging` wird ein Multi-Plattform-Gateway realisiert, das Nachrichtenempfang, Befehlsanalyse und automatische KI-Antworten für **DingTalk, Feishu, QQ, Slack, WeChat, WhatsApp, Telegram und Discord** unterstützt.

### Werkzeugsystem

**163+ integrierte Werkzeuge**, einheitlich über das `Tool`-Trait registriert, die 15 Hauptkategorien abdecken:

| Kategorie          | Werkzeugbeispiele                                                                                                                                                        |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Dateioperationen   | `file_read`, `file_write`, `file_edit`, `glob`, `grep`, Verzeichnis/Löschen/Verschieben usw. — 11                                                                        |
| Shell/Web          | `bash`, `web_fetch`, `web_search`                                                                                                                                        |
| Netzwerk           | `http_request`, `ping`, `dns_lookup`, `json_api`, `rss_reader`, `graphql`, `websocket`                                                                                   |
| Browser            | `browser_navigate`, `browser_click`, `browser_fill`, `browser_screenshot` usw. — 10 (CDP)                                                                                |
| Computersteuerung  | `computer_use` (Maus/Tastatur/Screenshot)                                                                                                                                |
| Git                | `git_status`, `git_diff`, `git_commit`, `git_log`, `git_branch`, `git_review`                                                                                            |
| Wissensdatenbank   | `list_knowledge_bases`, `search_knowledge`, `add_knowledge_document` usw. — 6                                                                                            |
| Aufgabenverwaltung | `todo_write`, `task_*` (6), `cron_*` (3), `plan`-bezogen                                                                                                                 |
| Nachrichten-Push   | `push_notification`, `send_message`, Team-Kollaborationswerkzeuge                                                                                                        |
| Datenbank          | `database_query`, `database_list_tables`, `database_migration_status`                                                                                                    |
| Speicher           | `get_storage_info`, `upload_storage_file`, `download_storage_file` usw. — 5                                                                                              |
| Export/Format      | `export_word`, `export_pdf`, `export_xlsx`, `export_pptx`, `render_markdown` usw. — 9                                                                                    |
| OCR                | `ocr_image`, `ocr_detect_langs`                                                                                                                                          |
| Obsidian           | `obsidian_search`, `obsidian_read`, `obsidian_backlinks` usw. — 9                                                                                                        |
| Sonstige           | `agent`, `delegate_task`, `skills_*`, `lsp`, `repl`, `monitor`, `workspace_*`, `session_search`, `generate_image`, `sequential_thinking`, CI/CD, DevOps, RPC, Tests usw. |

### MCP-Protokoll

Vollständige MCP (Model Context Protocol)-Implementierung auf Basis von `rmcp`:

- **Transportebene**: stdio-Unterprozess + Streamable HTTP + SSE
- **OAuth-Authentifizierung**: Unterstützung des OAuth-Autorisierungsflusses für MCP-Server
- **Werkzeugerkennung**: Automatische Erkennung und Registrierung der von MCP-Servern bereitgestellten Werkzeuge
- **MCP-Manager**: Lebenszyklusverwaltung der Server, Gesundheitsprüfungen, automatische Wiederverbindung

### Plugin-System

OpenClaw-kompatible dreistufige Plugin-Architektur (integriert / gebündelt / extern):

- npm-Paketinstallation, mit integrierter Marketplace-UI für Suche und Installation
- Plugin-Manifest-Definition, Berechtigungsdeklaration, sandbox-isolierte Ausführung
- Registrierung benutzerdefinierter Werkzeuge, Agent-Provider, Hook-Interception
- Fähigkeitsinstaller: Installation von Fähigkeiten aus Plugin-Paketen in das Fähigkeitensystem

### Dynamische UI-Engine

- **Schema-gesteuert**: Deklarativer Aufbau von Oberflächen über JSON Schema, ohne Code schreiben zu müssen
- **31 integrierte Komponenten**: Container (7) / Datenanzeige (6) / Formulare (9) / Medien (4) / Sonstige (5)
- **Datenbindung**: Deklarative Datenquellenbindung und bedingtes Rendering
- **NL2UI**: Direkte Generierung dynamischer UI-Oberflächen aus natürlicher Sprache

### ACP-Client-SDK

- **ACP (Agent Client Protocol)**: Zweisprachiges SDK (TypeScript + Python), null Abhängigkeiten von Drittanbietern
- Sitzungsverwaltung, Prompt-Versand, Werkzeugaufruf-Protokollierung, WebSocket-Ereignisströme
- Kommunikation mit dem AxAgent-Dienst über die `/acp/v1/*`-Endpunkte

### Sicherheit

- **AES-256-GCM-Verschlüsselung**: Lokale verschlüsselte Speicherung von API-Schlüsseln und sensiblen Konfigurationen (`crypto`-Crate)
- **Prompt-Injection-Schutz**: Vierstufige Verteidigungspipeline (`prompt-guard`) — Mustererkennung → Delimiter-Escaping → XML-Wrapper → Vertrauenslabels, integriert in den gesamten Pfad von Konversation, Prompt-Erstellung, Git und RAG
- **SSRF-Schutz**: URL-Sicherheitsprüfung, die Anfragen an interne Netzwerkadressen blockiert
- **Inhaltsfilterung**: Sicherheitsfilterung für mehrere Inhaltstypen
- **Ratenbegrenzung**: Token-Bucket-Begrenzung für Werkzeugaufrufe und API-Anfragen
- **Schutzschalter**: Automatische Unterbrechung bei aufeinanderfolgenden Fehlern
- **Zugriffskontrolle**: Richtlinienbasierte Zugriffsberechtigungskontrolle für Werkzeuge
- **Sandbox-Isolierung**: Isolierte Ausführungsumgebungen für Agenten und Fähigkeiten

### Entwicklerwerkzeuge

- **Verteiltes Tracing** (`telemetry`): OpenTelemetry-Integration mit Span/Trace-Visualisierung
- **Strukturiertes Logging**: tracing-subscriber + chrono-Zeitstempel
- **Replay-Debugging**: Aufzeichnung und Wiedergabe von Agenten-Ausführungstrajektorien (`trajectory_recorder`)
- **DevTools-Panel**: Trace-Explorer-Timeline-Viewer, Benchmark Runner, Tool Recommender
- **Benchmarks**: Criterion-Benchmarks (tool_exec / llm_call / search)
- **CI-Prüfungen**: `npm run ci:check` integriert Typprüfung, Linting und Formatvalidierung

### Desktop- und Mobile-Erfahrung

- **Responsives Layout**: CSS-Breakpoint-basierte Anpassung für Desktop/Tablet/Mobile (3 Gerätelayouts: `desktop` / `tablet` / `mobile`)
- **11 Sprachen**: Vereinfachtes Chinesisch, Traditionelles Chinesisch, Englisch, Japanisch, Koreanisch, Französisch, Deutsch, Spanisch, Russisch, Hindi, Arabisch
- **Theme-Engine** (`rt-theme`): Dunkle/helle Themes + mehrere Voreinstellungen, tiefgehende Anpassung mit Ant Design 6
- **Monaco-Editor**: Syntaxhervorhebung, Diff-Vorschau, Mehrsprachenunterstützung
- **xterm.js-Terminal**: WebLinks, Unicode 11, Suche
- **Virtuelles Scrollen**: @tanstack/react-virtual + react-virtuoso
- **Diagramm-Rendering**: D2 + Mermaid + Recharts + Sigma (Graphen)
- **Command Palette**: Ctrl+K globale Befehlspalette
- **System-Tray + globale Tastenkürzel + Autostart**: Nicht-intrusiver Hintergrundbetrieb
- **Automatische Updates**: Versionsprüfung über GitHub Releases mit konfigurierbarem Intervall
- **Proxy-Unterstützung**: HTTP / SOCKS5-Proxy-Konfiguration
- **Cloud-Arbeitsbereich**: S3- und WebDAV-Speichersynchronisation mit Konflikterkennung und bidirektionaler Synchronisation

### Mobile

- Android APK/AAB (arm64-v8a, armeabi-v7a, x86_64)
- iOS IPA (arm64)
- Mobilspezifische Anpassungen: Safe-Area-Anpassung, untere Navigation, Drawer-Navigation

---

## Technische Architektur

### Tech-Stack

| Schicht               | Technologie                              | Version |
| --------------------- | ---------------------------------------- | ------- |
| Desktop-Framework     | Tauri                                    | 2.11    |
| Frontend-Framework    | React                                    | 19      |
| Typsystem             | TypeScript                               | 7       |
| UI-Bibliothek         | Ant Design                               | 6       |
| CSS-Framework         | TailwindCSS                              | 4       |
| Zustandsverwaltung    | Zustand                                  | 5       |
| Routing               | React Router                             | 7       |
| Code-Editor           | Monaco Editor                            | 0.55    |
| Terminal              | xterm.js                                 | 6       |
| Workflow-Editor       | ReactFlow                                | 12      |
| Diagramme             | D2 + Mermaid + Recharts + Sigma          |         |
| Animation             | Framer Motion                            | 12      |
| Virtuelles Scrollen   | @tanstack/react-virtual + react-virtuoso |         |
| Drag & Drop           | @dnd-kit                                 | 6       |
| Markdown-Rendering    | markstream-react + stream-markdown       |         |
| Internationalisierung | i18next + react-i18next                  |         |
| Build-Werkzeug        | Vite                                     | 8       |
| Tests                 | Vitest + Playwright                      |         |
| Formatierung          | dprint (TS/JSON/Markdown/TOML) + rustfmt |         |
| Linting               | ESLint + Oxlint + Clippy                 |         |

### Backend-Architektur: Harness-Dependency-Injection-Muster

Die Rust-Workspace-Architektur umfasst **37 Mitglieder** (Haupt-Crate + 35 Bibliotheks-Crates + schema-gen) und folgt der **Harness-Dependency-Injection-Architektur**:

> Alle Crates werden über die von axagent-harness definierten Trait-Schnittstellen entkoppelt; zur Laufzeit assembliert und injiziert axagent-runtime die Abhängigkeiten.
> Abhängigkeitsrichtung: `konkrete Implementierungen → harness ← Aufrufer`

**harness** ist der architektonische Grundstein — null Geschäftslogik, null konkrete Implementierungen, enthält nur Trait-Definitionen, reine Daten-DTOs, Konstanten und einheitliche Fehlertypen. Es wird von allen anderen Crates abhängig gemacht und hängt selbst von keinem axagent-*-Crate ab (200+ Trait-Definitionen, die Agent/Provider/Tool/RAG/Speicher/MCP/Plugins/Sicherheit/Observability/Speicher/Lernen/Browser/Messaging/Kognitives Routing usw. abdecken).

```
src-tauri/crates/
├── harness/          # 架构基石 — trait 接口、DTO、错误类型、DI 契约
├── entities/         # SeaORM 实体模型
├── dao/              # 数据访问层（CRUD）
├── migration/        # 数据库迁移
├── crypto/           # AES-256-GCM 加解密与密钥管理
├── credential/       # 凭据安全存储
├── storage/          # 文件存储抽象（本地/S3/WebDAV），ZIP 读写
├── cache/            # 内存缓存层
├── disk-cache/       # 磁盘文件级缓存
├── search/           # 检索引擎（FTS5 + sqlite-vec + candle 本地嵌入）
├── document-parser/  # 文档文本提取（PDF/DOCX/XLSX/PPTX）
├── kit/              # 通用工具集（路径/编码/哈希/日期）
├── runtime-core/     # 运行时公共类型、配置常量
├── runtime/          # 运行时服务编排 — 装配全部 crate 的 DI 容器
├── rt-workflow/      # 工作流引擎 — DAG 编排、节点执行器、YAML 序列化
├── rt-messaging/     # 消息平台网关 — 钉钉/飞书/QQ/Slack/微信/WhatsApp/Telegram/Discord
├── rt-webhook/       # 通用 Webhook 服务器
├── rt-dashboard/     # 仪表盘插件框架
├── rt-theme/         # 主题引擎
├── agent/            # AI 智能体核心 — 80+ 模块
│                     #   ReAct引擎/层级规划/深度研究/事实核查/思维树/反思/
│                     #   自验证/错误恢复/RL优化/LoRA微调/评估/工具推荐/A/B测试/
│                     #   协调器/黑板/视觉管线/Web搜索/学术搜索/Wiki编译等
├── orchestrator/     # 智能体编排 — 多智能体调度、DAG 分解、动态子图执行
├── providers/        # 模型提供商适配器（13 种）
├── tools/            # 工具体系 — Tool trait/注册表/编排/流式/沙箱/163+内置工具
├── gateway/          # API 网关 — axum HTTP/WS 服务器、OAuth、速率限制、Prometheus
├── mcp/              # MCP 协议 — stdio + Streamable HTTP + SSE，基于 rmcp
├── trajectory/       # 学习系统 — 记忆/技能进化/用户画像/梦境整合
├── plugins/          # 插件系统 — OpenClaw 兼容、npm 包安装、市场
├── telemetry/        # 可观测性 — OpenTelemetry、结构化日志、运行时指标
├── prompt-guard/     # 提示词注入防护 — L1-L4 多级检测管线
├── npm/              # npm 注册表客户端
├── crdt/             # 协同编辑数据结构
├── device/           # 设备管理
├── axagent-mobile/   # 移动端适配层
├── agent-macro/      # 智能体宏
├── agent-command-types/ # 智能体命令类型
└── schema-gen/       # 数据库 Schema 生成工具
```

### Frontend-Architektur

```
src/
├── pages/            # 页面（24 个）
│   ├── ChatPage           # 对话主界面 — 侧边栏/消息流/Agent 面板/多 Tab
│   ├── DashboardPage      # 数据仪表盘 — 用量统计/模型分布/趋势图表
│   ├── WorkflowPage       # 工作流编辑器 — ReactFlow DAG 可视化
│   ├── KnowledgeHubPage   # 知识库管理 — 文档上传/索引/检索
│   ├── MemoryPage         # 记忆管理
│   ├── SkillsPage         # 技能市场
│   ├── SettingsPage       # 设置面板 — 40+ 配置项
│   ├── TerminalPage       # 内置终端 — xterm.js
│   ├── FilesPage          # 文件管理
│   ├── GatewayLinkPage    # API 网关与外部链接管理
│   ├── QuickBarPage       # 快捷栏（独立窗口）
│   ├── DynamicUIManagerPage / DynamicPageViewer  # 动态 UI 引擎
│   ├── WikiGraphPage / WikiEditPage / IngestPage # LLM Wiki
│   ├── LearningGraphPage  # 学习图谱
│   ├── FineTunePage       # LoRA 微调
│   ├── PersonaPage        # 角色管理
│   ├── WorkflowMarketplace # 工作流市场
│   ├── DevTools/          # TraceExplorer / BenchmarkRunner / ToolRecommender
│   └── Workflow/          # WorkflowListPage
│
├── components/       # 33 个模块，500+ 组件
│   ├── chat/         # 对话（消息流/输入/ChatView/TabBar/RightPanel/附件/工具调用渲染）
│   ├── layout/       # 布局 — AppInitializer / Sidebar / ContentArea / TitleBar /
│   │                 #   CommandPalette / GlobalCopyMenu / GlobalErrorBoundary /
│   │                 #   GlobalStatusBar / ErrorNotificationToast / AppHeader 等
│   ├── agent/        # Agent 面板/入口/迷你面板
│   ├── workflow/     # 工作流编辑器（节点/连线/面板/模板/AI辅助）
│   ├── settings/     # 设置面板（40+ 子组件）
│   ├── skill/        # 技能编辑器/渲染器/浮动面板
│   ├── dynamicUI/    # 动态 UI 组件（31 个内置组件）
│   ├── gateway/      # API 网关管理
│   ├── files/        # 文件管理
│   ├── terminal/     # 终端组件
│   ├── search/       # 搜索界面
│   ├── benchmark/    # 基准测试面板
│   ├── decomposition/# 技能分解与工具生成
│   ├── devtools/     # Trace/Span 时间线 + RL Training 面板
│   ├── approval/     # 审批流程界面
│   ├── recommendation/ # 工具/模型推荐
│   ├── onboarding/   # WelcomeWizard / InteractiveTutorial
│   ├── help/         # 帮助面板
│   ├── notification/ # 通知组件
│   ├── proactive/    # 主动建议
│   ├── llm-wiki/     # LLM Wiki 组件
│   ├── wiki/         # Wiki 组件
│   ├── fine-tune/    # 微调界面
│   ├── trace/        # Trace 组件
│   ├── style/        # 样式/主题
│   ├── shared/       # 共享组件（ErrorBoundary / PageContextProvider）
│   └── common/       # 通用组件（Icon 等）
│
├── stores/           # Zustand 状态管理（82 个 store）
│   ├── domain/       # 9 个核心业务 store（对话/流/压缩/偏好/多模型等）
│   ├── feature/      # 61 个功能模块 store（智能体/工作流/知识库/技能/网关/记忆/终端等）
│   ├── shared/       # 8 个跨组件共享 store（UI/标签页/工作区/后端状态等）
│   └── devtools/     # 4 个开发者工具 store
│
├── hooks/            # React Hooks（快捷键/命令面板/响应式/滚动条/主题/Avatar 等）
├── lib/              # 工具函数库（invoke/pageRegistry/shortcuts/skillLifecycle/
│                     #   chartGenerator/codeExecutor/tokenEstimator/workflowLayout 等 45+ 模块）
├── types/            # TypeScript 类型定义
├── theme/            # Shadcn 主题引擎
├── i18n/             # 11 语言翻译文件（zh-CN/zh-TW/en-US/ja/ko/fr/de/es/ru/hi/ar）
├── constants/        # 常量与功能开关
└── sdk/              # ACP 客户端 SDK（TypeScript + Python）
```

### Feature-Flags

Das Projekt verwaltet die progressive Feature-Veröffentlichung über `featureFlags.ts`:

| Flag                | Status | Beschreibung                                                     |
| ------------------- | ------ | ---------------------------------------------------------------- |
| `AGENT_IN_THE_LOOP` | ✅     | Globales Agent-Panel + Injektion des Seitenkontexts              |
| `DYNAMIC_UI`        | ✅     | Dynamische UI-Builder-Engine                                     |
| `SELF_EVOLUTION_UI` | ❌     | Frontend-Steueroberfläche für Selbstevolution                    |
| `NL_EXTENSION`      | ❌     | Natürlichsprachlich gesteuerte dynamische Geschäftserweiterungen |

### Tauri-Plugins

| Plugin              | Zweck                               |
| ------------------- | ----------------------------------- |
| `autostart`         | Automatischer Start beim Booten     |
| `clipboard-manager` | Zwischenablage Lesen/Schreiben      |
| `dialog`            | Dateiauswahldialoge                 |
| `fs`                | Dateisystemzugriff                  |
| `global-shortcut`   | Registrierung globaler Tastenkürzel |
| `notification`      | Systembenachrichtigungen            |
| `opener`            | Externe Links/Dateien öffnen        |
| `process`           | Prozessverwaltung                   |
| `updater`           | Automatische Updates                |

---

## Datenverzeichnisse

```
~/.axagent/                    # 应用配置
├── axagent.db                 # SQLite 主数据库 (SeaORM)
├── master.key                 # AES-256 主密钥
├── vector_db/                 # sqlite-vec 向量索引
└── ssl/                       # 自签名 SSL 证书

~/Documents/axagent/          # 用户文件
├── images/                   # 图片附件
├── files/                    # 文件附件
└── backups/                  # 自动备份
```

---

## Schnellstart

### Voraussetzungen

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+ (Edition 2024)
- [npm](https://www.npmjs.com/) 10+
- Windows: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (MSVC + Windows SDK)
- macOS: Xcode Command Line Tools
- Linux: `build-essential` + `libwebkit2gtk-4.1-dev` + `libssl-dev`

### Entwicklung

```bash
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent
npm install
npm run tauri dev      # 开发模式（前端 Vite HMR + Tauri 窗口）
```

### Build

```bash
npm run tauri build    # 桌面端生产构建

npm run tauri:android:build   # Android 构建
npm run tauri:ios:build       # iOS 构建
```

Die Desktop-Build-Artefakte befinden sich in `src-tauri/target/release/`.

### Tests

```bash
npm run test           # 前端单元测试（Vitest watch）
npm run test:run       # 前端单元测试（单次运行）
npm run test:e2e       # E2E 测试（Playwright）

# Rust 后端测试
cd src-tauri && cargo test

# 类型检查 & Lint
npm run typecheck
cd src-tauri && cargo clippy -- -D warnings
npm run format         # dprint 格式化
npm run lint:eslint    # ESLint 检查
npm run contracts      # API 契约检查

# CI 全量检查
npm run ci:check
```

### Häufige Skripte

| Befehl                   | Zweck                            |
| ------------------------ | -------------------------------- |
| `npm run bump`           | Interaktive Versionserhöhung     |
| `npm run docs`           | TypeDoc-Dokumentation generieren |
| `npm run skill:create`   | Neues Fähigkeitsgerüst erstellen |
| `npm run skill:validate` | Fähigkeitsdefinition validieren  |
| `npm run check:types`    | Typkonsistenzprüfung             |

---

## Unterstützte Plattformen

| Plattform | Architektur                           |
| --------- | ------------------------------------- |
| Windows   | x86_64, ARM64                         |
| macOS     | Apple Silicon (arm64), Intel (x86_64) |
| Linux     | x86_64, ARM64                         |
| Android   | arm64-v8a, armeabi-v7a, x86_64        |
| iOS       | arm64                                 |

---

## Open-Source-Lizenz

Dieses Projekt ist unter der [AGPL-3.0-only](LICENSE)-Lizenz als Open Source veröffentlicht.

---

## Danksagungen

AxAgent basiert auf vielen herausragenden Open-Source-Projekten:

- [Tauri](https://tauri.app/) — Plattformübergreifendes Desktop-Framework
- [React](https://react.dev/) + [Ant Design](https://ant.design/) — Frontend-UI
- [SeaORM](https://www.sea-ql.org/SeaORM/) — Rust ORM
- [sqlite-vec](https://github.com/asg017/sqlite-vec) — Vektorsuche
- [candle](https://github.com/huggingface/candle) — Lokale Embedding-Inferenz
- [rmcp](https://github.com/nicholasxjy/rmcp) — Rust MCP SDK
- [ReactFlow](https://reactflow.dev/) — Visueller Workflow-Editor
- [axum](https://github.com/tokio-rs/axum) — HTTP-Framework
- [Monaco Editor](https://microsoft.github.io/monaco-editor/) — Code-Editor
- [xterm.js](https://xtermjs.org/) — Terminal-Emulator
- [Zustand](https://zustand.docs.pmnd.rs/) — Zustandsverwaltung
- [Framer Motion](https://www.framer.com/motion/) — Animationsbibliothek
- [Recharts](https://recharts.org/) — Diagrammbibliothek
