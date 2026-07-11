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

**AxAgent** ist ein Open-Source-Desktopclient für KI-Assistenten, der plattformübergreifend **Windows / macOS / Linux / Android / iOS** unterstützt. Er geht weit über eine Chat-Oberfläche hinaus – er integriert eine ReAct-Agent-Engine, visuelle Workflow-Orchestrierung, lokale RAG-Wissensdatenbanken, MCP-Protokollerweiterungen, ein einheitliches Multi-Modell-Gateway, Browser-Automatisierung und Computersteuerung und dient als KI-Workstation für tägliche Entwicklung, Forschung, Wissensmanagement und Automatisierung.

> **Sprachen**: [简体中文](./README.md) | [English](./README-EN.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

---

## Was AxAgent löst

AxAgent adressiert drei Kernprobleme:

1. **Einheitliche Multi-Modell-Orchestrierung**: OpenAI, Anthropic Claude, Google Gemini, lokale Ollama-Modelle und jede OpenAI-kompatible API in einer einzigen Oberfläche nutzen, mit Multi-Key-Rotation, intelligenter Modellweiterleitung und Streaming-Vergleich
2. **Operationalisierung von KI-Fähigkeiten**: KI von „Konversation" auf „Ausführung" erweitern – durch 47+ eingebaute Werkzeuge, visuelle Workflows, MCP-Erweiterungen, Browser-Automatisierung und Computersteuerung kann KI direkt Dateien bearbeiten, Code ausführen, Git verwalten und Aufgaben planen
3. **Local-First-Datensouveränität**: KI-Gespräche, Wissensdatenbanken, Erinnerungen und Konfigurationsdateien werden alle in einer lokalen SQLite-Datenbank gespeichert. API-Schlüssel werden mit AES-256-GCM verschlüsselt. Die Kernfunktionen laufen ohne Drittanbieter-Cloud-Dienste.

---

## Kernfunktionen

### Multi-Modell-Engine

- **9 Provider-Adapter**: OpenAI (Chat Completions + Responses + Realtime), Anthropic Claude, Google Gemini, Ollama (mit GGUF-Verwaltung), OpenClaw, Hermes sowie alle OpenAI-kompatiblen APIs
- **Multi-Key-Rotation**: Mehrere API-Schlüssel pro Provider konfigurieren, mit automatischer, kontingentbasierter Rotation, um unterbrechungsfreie Anfragen bei Ratenbegrenzungen zu gewährleisten
- **Intelligentes Routing**: Automatische Auswahl des passendsten Modells nach Aufgabentyp (Code-Review / Zusammenfassung / Übersetzung / Allgemein), mit anpassbaren Routing-Regeln
- **Provider-Gesundheitsüberwachung**: Echtzeit-Verfolgung von Erfolgsrate, Latenz und Verfügbarkeit pro Provider, mit gestufter automatischer Fallback (ProviderTier)
- **KI-Bildgenerierung**: DALL-E 3 und Flux (Replicate) mit Mehrfachgrößen-Voreinstellungen
- **Echtzeit-Sprache**: WebSocket-Sprachkonversation basierend auf der OpenAI-Realtime-API, mit Unterbrechung und Streaming-Transkription

### Agentensystem

Das gesamte Agentensystem ist auf einer **ReAct-Engine (Reasoning + Acting)** aufgebaut und umfasst folgende implementierte Subsysteme:

- **Hierarchischer Planer** (`hierarchical_planner`): Zerlegt komplexe Aufgaben in strukturierte Phase → Task-Pläne mit Abhängigkeitsbeziehungen, kompiliert zu DAG-topologischer Ausführung
- **Tiefgehende Recherche** (`deep_research`): Multi-Quellen-Such-Orchestrierung einschließlich Suchplanung (`search_planner`), Suchexecution (`search_orchestrator`), Inhaltsynthese (`content_synthesizer`) und Zitatverfolgung (`citation_tracker`)
- **Faktenprüfer** (`fact_checker`): KI-gestützte Faktenverifizierung mit Quellenklassifizierer (`source_classifier`), Quellenvalidierer (`source_validator`) und Glaubwürdigkeitsbewerter (`credibility_evaluator`)
- **Tree of Thoughts** (`tree_of_thoughts`): Multi-Pfad-Inferenzexploration mit Zweigbewertung und Backtracking
- **Reflektor** (`reflector`): Selbsteinschätzung und Verbesserungsvorschläge nach der Aufgabe
- **Selbstverifizierer** (`self_verifier`): Automatische Validierung von Inferenzergebnissen mit Zykluserkennung (`cycle_detector`) zur Vermeidung von Endlosschleifen
- **Fehlerwiederherstellung** (`error_recovery_engine`): Fehlertypen klassifizieren → Wiederherstellungsstrategie wählen → automatischer Retry oder Plananpassung, mit exponentiellem Backoff
- **A/B-Testing** (`ab_testing`): Vergleichende Evaluierung verschiedener Inferenzstrategien
- **Evaluierungssystem** (`evaluator`): Eingebauchtes Benchmark-Framework mit Unterstützung für Datensätze, Metriken und Berichtserstellung
- **LoRA-Feinabstimmung** (`fine_tune`): Eingebauchte Trainingspipeline mit LoRA-Adapter-Verwaltung
- **RL-Optimierer** (`rl_optimizer`): erfahrungsfeedbackbasierte Policy-Verstärkung durch Reinforcement Learning mit Experience Replay und Policy-Gradienten
- **Werkzeugempfehlung** (`tool_recommender`): kontextbasierte Analyse und Empfehlung von Werkzeugnutzungsmustern

**Multi-Agent-Kollaboration**:

- Master-Slave-Koordinationsarchitektur (`coordinator`) mit paralleler Sub-Agent-Ausführung und abhängigkeitsbewusster Planung
- Gemeinsame Blackboard (`shared_blackboard`) für den Informationsaustausch zwischen Agenten
- Adversarialer Debattenmodus mit Pro/Contra-Runden und Argumentstärke-Scoring
- Swarm-Cluster-Modus mit Multi-Prozess-Agent-Clustern, unterstützt Berechtigungssynchronisation und Auto-Reconnect
- Proaktiver Modus (`proactive_mode`): Agenten können proaktiv Vorschläge und Aktionen unterbreiten

**Computersteuerung**: KI-gesteuerte Mausklicks, Tastatureingaben, Bildschirmscrollen mit dreistufigen Berechtigungen (Default / Accept Edits / Full Access) und Sandbox-Pfadisolation

**Browser-Automatisierung**: Browsersteuerung über das CDP-Protokoll mit Unterstützung für Navigation, Screenshots, Klicks, Formularausfüllung, Textextraktion und Seitenzustandsüberwachung

### Fähigkeitssystem (Skill System)

- **Skill-Marktplatz**: Community-Skills durchsuchen und installieren
- **KI-gestützte Erstellung**: Automatische Erzeugung von Skill-Strukturen aus natürlichsprachlichen Vorschlägen
- **Skill-Evolution** (`evolution_engine`): Automatische Analyse und Verbesserung von Skills basierend auf Ausführungsfeedback
- **Semantisches Matching** (`skill`): Semantische Zuordnung und automatische Empfehlung relevanter Skills basierend auf dem Gesprächskontext
- **Skill-Zerlegung** (`skill_decomposition`): Automatische Zerlegung komplexer Aufgaben in atomare Skill-Kombinationen
- **Generierte Werkzeuge** (`generated_tool`): Von KI generierte und registrierte neue Werkzeuge
- **Sandbox-Ausführung** (`sandbox`): Skills werden sicher in isolierten Sandbox-Umgebungen ausgeführt

### Visueller Workflow

Drag-and-Drop-DAG-Workflow-Editor basierend auf ReactFlow 12:

- **17 Knotentypen**: Trigger, Agent, LLM-Call, Conditional Branch, Parallel Fork, Loop, Merge, Delay, Tool Call, Code Execution, Sub-workflow, Vector Retrieval, Document Parsing, Validation, End, Business Rule, Agent Role
- **Kahn-Topologie-Sortierungsausführung**: Automatische zyklische Abhängigkeitserkennung mit paralleler Pipeline-Planung
- **Eingebauchte Vorlagen**: Code-Review, Bug-Fix, Dokumentgenerierung, Tests, Refactoring, Exploration, Performance-Analyse, Security-Audit, Feature-Entwicklung
- **YAML-Serialisierung**: Workflow-Definitionen unterstützen YAML-Import/-Export
- **Versionsverwaltung**: Versionskontrolle für Workflow-Vorlagen
- **KI-Unterstützung**: KI-gestütztes Workflow-Design und Knotenempfehlungen

### Wissensverwaltung

- **Multi-KB-RAG**: Dokumentupload → automatisches Parsing (PDF/DOCX/XLSX/PPTX/TXT) → Chunking → Vektorindexierung
- **Hybride Suche**: Vektorähnlichkeit (sqlite-vec + candle lokale Embeddings) + BM25-Volltextsuche (FTS5) mit hybrider Rangfolge
- **Self-RAG**: selbst-abrufende, auf Augmentierung basierende Generierung mit automatischer Reflexion und Verifizierung von Abrufergebnissen
- **Re-Ranking**: Cross-Encoder-Ergebnis-Re-Ranking zur Präzisionsverbesserung
- **Wissensgraph**: Entitätsextraktion (`EntityExtractor`) → Beziehungsaufbau → visueller Graph
- **Dateiüberwachung**: Echtzeit-Überwachung von Dateiänderungen über `notify` mit automatischer inkrementeller Indizierung
- **LLM-Wiki**: KI-gestützter Wiki-Compiler und -Validator mit Wiki-Clipping-Browser-Erweiterung

### Gedächtnissystem (Memory System)

- **Multi-Namespace-Gedächtnis**: Isolation nach Projekt/Thema, unterstützt manuelle Eingabe und KI-Auto-Extraktion
- **Persistenz-Integration**: Honcho- und Mem0-Gedächtnis im geschlossenen Kreislauf
- **Benutzerprofil** (`user_profile` / `profile`): Automatisches Erlernen des Codierungsstils (Einrückung/Benennung/Kommentare), Tech-Stack-Präferenzen und Kommunikationsstil
- **Stilübertragung** (`style`): Extrahieren von Code-Stilmerkmalen → Anwenden auf KI-generierten Code
- **Dream-Integration** (`dream`): Hintergrund-Auto-Konsolidierung von Gedächtnisfragmenten und Verhaltensmustern in strukturiertes Wissen
- **Projektgedächtnis** (`project_memory`): Kontextpersistenz pro Projekt

### API-Gateway

Eingebauter HTTP- + WebSocket-Gateway-Server basierend auf `axum`:

- **Kompatible Endpunkte**: OpenAI `/v1/chat/completions`, Claude Messages API, Gemini API sowie OpenAI Responses und Realtime WebSocket
- **Schlüsselverwaltung**: Generieren, Widerrufen, Aktivieren/Deaktivieren von Zugriffsschlüsseln mit Ablaufunterstützung
- **Nutzungsverfolgung**: Anfragezähler und Token-Verbrauchsstatistiken pro Schlüssel, Provider und Datum mit Prometheus-Metrik-Export
- **Ratenbegrenzung**: Token-Bucket-Algorithmus über `governor` mit konfigurierbaren Ratenbegrenzungsrichtlinien
- **SSL/TLS**: Eingebauchte selbstsignierte Zertifikate (`rcgen`) mit Unterstützung für benutzerdefinierte Zertifikate
- **Externe Verknüpfung**: Ein-Klick-Integration mit Claude CLI, OpenCode und anderen externen Tools mit automatischer API-Schlüssel-Synchronisation
- **Echtzeit-Tickets**: HMAC-basierte temporäre Authentifizierungstickets für sicheres WebSocket-Echtzeit-Verbindungs-Handoff

### Messaging-Plattform-Integration

Messaging-Plattform-Gateway über die `rt-messaging`-Crate implementiert, unterstützt:

DingTalk, Feishu, QQ, Slack, WeChat, WhatsApp, Telegram, Discord

Unterstützt Webhook-Nachrichtenempfang, Befehlsparsing und automatische KI-Antwortzustellung.

### Werkzeugsystem (Tool System)

47 eingebaute Werkzeuge, alle einheitlich über das `Tool`-Trait registriert:

| Kategorie          | Werkzeuge                                                                                                                                                                                                  |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Dateioperationen   | `file_read`, `file_write`, `file_edit`, `file_system` (list/search/metadata)                                                                                                                               |
| Code-Ausführung    | `bash`, `repl`                                                                                                                                                                                             |
| Suche              | `grep`, `glob`                                                                                                                                                                                             |
| Browser            | `browser` (CDP-Steuerung)                                                                                                                                                                                  |
| Computersteuerung  | `computer_use` (Maus/Tastatur/Screenshot)                                                                                                                                                                  |
| Web                | `web_search`, `web_fetch`                                                                                                                                                                                  |
| Wissensdatenbank   | `knowledge`, `document` (Dokumenten-Parsing)                                                                                                                                                               |
| Git                | `git` (commit/push/branch/diff)                                                                                                                                                                            |
| Entwicklungs-Tools | `lsp` (Language Server Protocol), `workspace`                                                                                                                                                              |
| Aufgabenverwaltung | `plan`, `task_system`, `todo_write`, `cron`                                                                                                                                                                |
| Benachrichtigungen | `push_notification`, `messaging`                                                                                                                                                                           |
| Datenbank          | `database`                                                                                                                                                                                                 |
| Speicher           | `storage`                                                                                                                                                                                                  |
| Sonstiges          | `agent`, `agent_memory`, `context`, `export`, `integration`, `media`, `media_delivery`, `migration_tool`, `monitor`, `obsidian`, `ocr`, `personality`, `shared_path`, `system_info`, `testing`, `worktree` |

### MCP-Protokoll

Vollständige MCP-Implementierung (Model Context Protocol) basierend auf der `rmcp`-Crate:

- **Transport-Schicht**: stdio-Subprozess + Streamable HTTP + WebSocket
- **OAuth-Authentifizierung**: OAuth-Autorisierungsfluss-Unterstützung für MCP-Server
- **Werkzeugentdeckung**: Automatische Erkennung und Registrierung von Werkzeugen, die von MCP-Servern bereitgestellt werden
- **MCP-Manager**: Server-Lebenszyklusverwaltung, Gesundheitschecks, Auto-Reconnect

### Plugin-System

OpenClaw-kompatible dreistufige Plugin-Architektur (Built-in / Bundled / External), unterstützt:

- npm-Paketinstallation mit eingebauter Marktplatz-UI zum Suchen und Installieren
- Plugin-Manifest-Definition, Berechtigungsdeklarationen, sandbox-isolierte Ausführung
- Benutzerdefinierte Werkzeugregistrierung, Agent-Provider, Hook-Interception
- Skill-Installer: Skills aus Plugin-Paketen in das Skill-System installieren

### Sicherheit

- **AES-256-GCM-Verschlüsselung**: Lokale verschlüsselte Speicherung für API-Schlüssel und sensible Konfiguration (`crypto`-Crate)
- **Prompt-Injection-Schutz**: Vierstufige Verteidigungspipeline (`prompt-guard`) – Mustererkennung → Trennzeichen-Escaping → XML-Wrapper → Vertrauenslabels, integriert in Sitzungen, Prompt-Konstruktion, Git und RAG über die gesamte Pipeline
- **SSRF-Schutz** (`ssrf_guard`): URL-Sicherheitsprüfungen, die Anfragen an interne Netzwerkadressen blockieren
- **Inhaltsfilterung** (`content_filter`): Multi-Typ-Inhaltssicherheitsfilterung
- **Ratenbegrenzung** (`rate_limiter`): Token-Bucket-Ratenbegrenzung für Werkzeugaufrufe und API-Anfragen
- **Circuit Breaker** (`circuit_breaker`): Automatische Unterbrechung bei aufeinanderfolgenden Fehlern zum Schutz der Systemstabilität
- **Zugriffskontrolle** (`tool_access`): Richtlinienbasierte Werkzeugzugriffs-Berechtigungskontrolle
- **Sandbox-Isolation**: Ausführungsumgebungsisolation für Agenten und Skills

### Entwicklererfahrung

- **Verteiltes Tracing** (`telemetry`): OpenTelemetry-Integration mit Span/Trace-Visualisierung
- **Telemetrie** (`telemetry`): Strukturiertes Logging, Laufzeitmetriken, Leistungsereignis-Sammlung
- **Replay-Debugging**: Aufzeichnung der Agenten-Ausführungsspur (`trajectory_recorder`) und Replay
- **DevTools-Panel**: Eingebauter Frontend-Trace/Span-Zeitlinien-Betrachter
- **Benchmark-Framework**: Criterion-Benchmarks (tool_exec / llm_call / search), SWE-bench- und Terminal-bench-Evaluierung

### Desktop- und Mobile-Erfahrung

- **Responsives Layout**: CSS-Breakpoints adaptiv für Desktop / Tablet / Mobil (600px / 900px)
- **11 Sprachen**: Vereinfachtes Chinesisch, Traditionelles Chinesisch, Englisch, Japanisch, Koreanisch, Französisch, Deutsch, Spanisch, Russisch, Hindi, Arabisch
- **Theme-Engine** (`rt-theme`): Dark/Light-Theme folgt der Systemeinstellung oder manueller Umschaltung, tief angepasst an Ant Design 6
- **Monaco-Editor**: Eingebauter Code-Editor mit Syntaxhervorhebung, Diff-Vorschau, Mehrsprachenunterstützung
- **xterm.js-Terminal**: Eingebauter Terminal-Emulator mit Unterstützung für WebLinks, Unicode 11, Suche
- **D2 / Mermaid / ECharts**: Architekturdiagramme, Flussdiagramme und interaktive Chart-Darstellung
- **Sitzungsfreigabe**: Ein-Klick-Freigabelink-Generierung mit konfigurierbaren Zugriffsberechtigungen
- **System-Tray + Globale Tastenkürzel + Auto-Start**: Unaufdringlicher Hintergrundbetrieb
- **Auto-Update**: Automatische Versionsupdate-Erkennung über GitHub Releases
- **Proxy-Unterstützung**: HTTP- und SOCKS5-Proxy-Konfiguration
- **Cloud-Workspace**: S3- und WebDAV-Speicher-Sync mit Konflikterkennung und bidirektionalem Sync

### Mobil

- Android APK/AAB (arm64-v8a, armeabi-v7a, x86_64)
- iOS IPA (arm64)
- Mobile-spezifische Anpassungen: Safe-Area-Anpassung, untere Navigationsleiste, Drawer-Navigation

---

## Technische Architektur

### Technologie-Stack

| Ebene                 | Technologie                              |
| --------------------- | ---------------------------------------- |
| Desktop-Framework     | Tauri 2.11                               |
| Frontend-Framework    | React 19 + TypeScript                    |
| UI-Bibliothek         | Ant Design 6 + TailwindCSS 4             |
| State-Management      | Zustand 5                                |
| Routing               | React Router 7                           |
| Code-Editor           | Monaco Editor                            |
| Terminal              | xterm.js 6                               |
| Workflow-Editor       | ReactFlow 12                             |
| Charts                | D2 + Mermaid + Recharts + ECharts        |
| Virtual Scrolling     | @tanstack/react-virtual + react-virtuoso |
| Drag & Drop           | @dnd-kit                                 |
| Markdown-Rendering    | markstream-react + stream-markdown       |
| Internationalisierung | i18next + react-i18next                  |
| Build-Tool            | Vite 8                                   |
| Testing               | Vitest + Playwright + cargo-nextest      |
| Formatierung          | dprint (TS/JSON/Markdown/TOML) + rustfmt |
| Linting               | ESLint + Oxlint + Clippy + cargo-deny    |

### Backend-Architektur: Harness-Dependency-Injection-Muster

Das Backend verwendet eine Rust-Workspace-Architektur mit **32 Crates** und folgt dem **Harness-Architekturmuster**:

```
Alle Crates sind über Trait-Interfaces entkoppelt, die in axagent-harness definiert sind.
Die Laufzeit (axagent-runtime) assembliert und injiziert Abhängigkeiten zur Laufzeit.

Abhängigkeitsrichtung: Konkrete Implementierungen → harness ← Aufrufer
```

**harness** ist der architektonische Eckstein – null Geschäftslogik, null konkrete Implementierungen, enthält nur Trait-Definitionen, reine Daten-DTOs, Konstanten und einheitliche Fehlertypen. Es wird von allen anderen Crates abhängig und hängt von keinem anderen axagent-*-Crate ab.

```
src-tauri/crates/
├── harness/          # Architektonischer Eckstein — Trait-Interfaces, DTOs, einheitliche Fehlertypen, DI-Verträge
│                     #   200+ Trait-Definitionen abdeckend: Agent/Provider/Tool/RAG/Storage/
│                     #   MCP/Plugins/Security/Observability/Memory/Learning/Browser/Messaging
│
├── entities/         # SeaORM-Entity-Modelle
├── dao/              # Datenzugriffsschicht (CRUD)
├── migration/        # Datenbankmigrationen
│
├── crypto/           # AES-256-GCM-Verschlüsselung/-Entschlüsselung und Schlüsselverwaltung
├── credential/       # Sichere Anmeldeinformationsspeicherung (API-Schlüssel usw.)
├── storage/          # Dateispeicher-Abstraktion (Local / S3 / WebDAV), ZIP-Lesen/Schreiben-Unterstützung
├── cache/            # Generische Caching-Schicht (im Speicher)
├── disk-cache/       # Festplattenbasierte Datei-Caches
├── search/           # Suchmaschine (FTS5 + sqlite-vec + candle-Embeddings)
├── document-parser/  # Dokumenttextextraktion (PDF/DOCX/XLSX/PPTX)
├── kit/              # Utility-Toolkit — Pfad/Kodierung/Hash/Datum-Helfer
│
├── runtime-core/     # Laufzeit-Grundtypen, Konfigurationskonstanten
├── runtime/          # Laufzeit-Service-Orchestrierung — assembliert alle 30+ Crates, der DI-Laufzeit-Container
│                     #   Verwaltet: Sitzungen/Terminals/Webhooks/Ratenbegrenzung/Berechtigungen/SSRF/Event-Bus/State
├── rt-workflow/      # Workflow-Engine — DAG-Orchestrierung, Knoten-Executors, YAML-Serialisierung
├── rt-messaging/     # Messaging-Plattform-Gateway — DingTalk/Feishu/QQ/Slack/WeChat/WhatsApp/Telegram/Discord
├── rt-webhook/       # Generischer Webhook-Server und Event-Dispatch
├── rt-dashboard/     # Dashboard-Plugin-Framework
├── rt-theme/         # Theme-Engine — Dark/Light-Umschaltlogik
│
├── agent/            # KI-Agenten-Kern — 80+ Module
│                     #   ReAct-Engine/hierarchische Planung/tiefgehende Recherche/Faktenprüfung/Tree of Thoughts/
│                     #   Reflexion/Selbstverifizierung/Fehlerwiederherstellung/RL-Optimierung/LoRA-Feinabstimmung/
│                     #   Evaluierung/Werkzeugempfehlung/A-B-Testing/Coordinator/Blackboard/Vision-Pipeline/
│                     #   Websuche/akademische Suche/Wiki-Kompilierung und mehr
│
├── orchestrator/     # Agenten-Orchestrierung — Multi-Agent-Planung, DAG-Zerlegung, dynamische Subgraphen-Ausführung
├── providers/        # Modell-Provider-Adapter — OpenAI/Anthropic/Gemini/Ollama/
│                     #   OpenClaw/Hermes/Bildgenerierung (DALL-E/Flux)/Realtime/Responses
├── tools/            # Werkzeugsystem — Tool-Trait/Registry/Orchestrierung/Streaming/Sandbox/47+ eingebaute Werkzeuge
├── gateway/          # API-Gateway — axum HTTP/WS-Server, OAuth, Ratenbegrenzung, Prometheus
├── mcp/              # MCP-Protokoll — stdio + Streamable HTTP, basierend auf rmcp
├── trajectory/       # Lernsystem — Gedächtnis/Skill-Evolution/Benutzerprofil/Dream-Integration
├── plugins/          # Plugin-System — OpenClaw-kompatibel, npm-Paketinstallation, Marktplatz
├── telemetry/        # Observability — OpenTelemetry, strukturiertes Logging, Laufzeitmetriken
├── prompt-guard/     # Prompt-Injection-Schutz — L1-L4 Multi-Level-Erkennungspipeline
├── npm/              # npm-Registry-Client
└── schema-gen/       # Datenbankschema-Generierungstool
```

### Frontend-Architektur

```
src/
├── pages/            # 22 Seiten
│   ├── ChatPage          # Haupt-Chat-Oberfläche
│   ├── WorkflowPage      # Workflow-Editor
│   ├── GatewayPage       # API-Gateway-Verwaltung
│   ├── KnowledgeHubPage  # Wissensdatenbank-Verwaltung
│   ├── MemoryPage        # Gedächtnisverwaltung
│   ├── SkillsPage        # Skill-Marktplatz
│   ├── SettingsPage      # Einstellungspanel
│   ├── DashboardPage     # Daten-Dashboard
│   ├── TerminalPage      # Terminal
│   ├── FilesPage         # Dateiverwaltung
│   ├── GatewayLinkPage   # Externe Link-Verwaltung
│   ├── LinkPage          # Integrationslinks
│   ├── WikiEditorPage    # Wiki-Editor
│   ├── WikiEditPage      # Wiki-Bearbeitung
│   ├── WikiGraphPage     # Wiki-Wissensgraph
│   ├── FineTunePage      # LoRA-Feinabstimmung
│   ├── PersonaPage       # Persona-Verwaltung
│   ├── QuickBarPage      # Quick-Bar
│   ├── IngestPage        # Dokumentenaufnahme
│   ├── WorkflowMarketplace # Workflow-Marktplatz
│   ├── DynamicUIManagerPage # Dynamische UI-Verwaltung
│   └── DynamicPageViewer    # Dynamischer Seitenbetrachter
│
├── components/       # 24 Module, 200+ Komponenten
│   ├── chat/         # Chat-UI (Nachrichtenstream/Eingabe/Anhänge/Werkzeugaufrufe/Artefakte/Denkblöcke)
│   ├── workflow/     # Workflow-Editor (Knoten/Kanten/Panels/Vorlagen/KI-Unterstützung)
│   ├── gateway/      # API-Gateway-Verwaltungs-UI
│   ├── settings/     # Einstellungspanel (40+ Sub-Komponenten)
│   ├── skill/        # Skill-Editor und -Renderer
│   ├── benchmark/    # Benchmark-Panel
│   ├── decomposition/# Skill-Zerlegung und Werkzeuggenerierung
│   ├── devtools/     # Trace/Span-Zeitlinie
│   ├── layout/       # Layout (Titelleiste/Seitenleiste/Befehlspalette)
│   └── ...
│
├── stores/           # 62 Zustand-Stores
│   ├── domain/       # Kern-Geschäftsstatus
│   ├── feature/      # Feature-Modul-Status (44)
│   └── devtools/     # DevTools-Status
│
├── hooks/            # React Hooks
├── lib/              # Hilfsfunktionen + Web Workers
├── types/            # TypeScript-Typdefinitionen
├── sdk/              # Externe Integrations-SDK
└── i18n/             # 11 Sprachübersetzungen (zh-CN/zh-TW/en-US/ja/ko/fr/de/es/ru/hi/ar)
```

---

## Datenverzeichnis

```
~/.axagent/                    # Anwendungskonfiguration
├── axagent.db                 # SQLite-Hauptdatenbank (SeaORM)
├── master.key                 # AES-256-Hauptschlüssel
├── vector_db/                 # sqlite-vec-Vektorindex
└── ssl/                       # Selbstsignierte SSL-Zertifikate

~/Documents/axagent/          # Benutzerdateien
├── images/                   # Bildanhänge
├── files/                    # Dateianhänge
└── backups/                  # Automatische Backups
```

---

## Schnellstart

### Anforderungen

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+, Edition 2024
- [npm](https://www.npmjs.com/) 10+
- Windows: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (MSVC + Windows SDK)
- macOS: Xcode Command Line Tools
- Linux: `build-essential` + `libwebkit2gtk-4.1-dev` + `libssl-dev`

### Erstellen (Build)

```bash
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent
npm install
npm run tauri dev      # Entwicklungsmodus
npm run tauri build    # Produktions-Build
```

Build-Artefakte befinden sich unter `src-tauri/target/release/`.

### Tests

```bash
npm run test           # Frontend-Unit-Tests (Vitest watch)
npm run test:run       # Frontend-Unit-Tests (einzelner Lauf)
npm run test:e2e       # E2E-Tests (Playwright)

# Rust-Backend-Tests
cd src-tauri && cargo nextest run
cd src-tauri && cargo test

# Typprüfung & Linting
npm run typecheck
cd src-tauri && cargo clippy -- -D warnings
npm run format

# CI-Vollprüfung
npm run ci:check
```

---

## Plattformunterstützung

| Plattform | Architektur                               |
| --------- | ----------------------------------------- |
| Windows   | x86_64, ARM64                             |
| macOS     | Apple Silicon (arm64), Intel (x86_64)     |
| Linux     | x86_64, ARM64                             |
| Android   | arm64-v8a, armeabi-v7a, x86_64 (Emulator) |
| iOS       | arm64                                     |

---

## Lizenz

Dieses Projekt ist unter der [AGPL-3.0-only](LICENSE)-Lizenz open-source verfügbar.

---

## Danksagungen

AxAgent baut auf vielen hervorragenden Open-Source-Projekten auf, unter anderem:

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
