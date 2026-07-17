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

**AxAgent** ist ein Tauri 2-basierter plattformübergreifender KI-Assistent-Desktop-Client (Windows / macOS / Linux / Android / iOS). Er integriert eine ReAct-Agent-Engine, visuelle Workflow-Orchestrierung, lokale RAG-Wissensdatenbanken, MCP-Protokollerweiterungen, ein vereinheitlichtes Multi-Modell-Gateway, Browser-Automatisierung und Computersteuerung — eine KI-gestützte Workstation für tägliche Entwicklung, Forschung, Wissensmanagement und Automatisierung.

> **Sprachen**: [简体中文](./README.md) | [English](./README-EN.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

---

## Projektpositionierung

AxAgent löst drei Kernprobleme:

1. **Vereinheitlichter Multi-Modell-Zugriff & Intelligentes Routing** — OpenAI, Anthropic Claude, Google Gemini, Ollama-Lokalmodelle und beliebige OpenAI-kompatible APIs in einer einzigen Oberfläche, mit Multi-Key-Kontingent-Rotation, aufgabentypspezifischem intelligentem Routing und Streaming-Vergleich
2. **Geschlossene KI-Schleife von Konversation zu Ausführung** — 47+ integrierte Tools + visuelle Workflows + MCP-Erweiterungen + Browser/Computer-Steuerung, KI kann Dateien bearbeiten, Code ausführen, Git verwalten und Aufgaben planen
3. **Local-First-Datenhoheit** — Konversationen, Wissensdatenbanken, Speicher und Konfiguration werden in einer lokalen SQLite-Datenbank gespeichert, API-Schlüssel mit AES-256-GCM verschlüsselt. Kernfunktionen arbeiten ohne Cloud-Dienste von Drittanbietern

---

## Kernfähigkeiten

### Multi-Modell-Engine

- **9 Provider-Adapter**: OpenAI (Chat Completions + Responses + Realtime), Anthropic Claude, Google Gemini, Ollama (mit GGUF-Lokalmodellverwaltung), OpenClaw, Hermes und alle OpenAI-kompatiblen APIs
- **Multi-Key-Rotation**: Mehrere API-Schlüssel pro Provider, kontingentbasierte automatische Rotation, automatisches Failover bei Einzelschlüssel-Begrenzung
- **Intelligentes Routing**: Automatische Modellauswahl nach Aufgabentyp (Code-Review / Zusammenfassung / Übersetzung / Allgemein), mit anpassbaren Regeln
- **Provider-Gesundheitsüberwachung**: Echtzeitverfolgung von Erfolgsrate, Latenz und Verfügbarkeit, mit gestaffeltem automatischem Fallback
- **KI-Bildgenerierung**: DALL-E 3 und Flux (Replicate) mit Multi-Größen-Voreinstellungen
- **Echtzeit-Sprache**: WebSocket-basierte Sprachkonversation über OpenAI Realtime API, mit Unterbrechungs- und Streaming-Transkriptionsunterstützung

### Agent-System (ReAct-Engine)

- **Hierarchischer Planer** (`hierarchical_planner`): Zerlegung komplexer Aufgaben in strukturierte Phase → Task-Pläne, kompiliert zur DAG-topologischen Ausführung
- **Tiefenrecherche** (`deep_research`): Multi-Quellen-Suchorchestrierung mit Suchplanung, -ausführung, Inhaltsynthese und Zitationsverfolgung
- **Faktenprüfer** (`fact_checker`): KI-gestützte Faktenprüfung mit Quellenklassifikator und Glaubwürdigkeitsbewertung
- **Gedankenbaum** (`tree_of_thoughts`): Multi-Pfad-Argumentationsexploration mit Zweigbewertung und Backtracking
- **Reflektor** (`reflector`): Selbstbewertung nach der Ausführung und Verbesserungsvorschläge
- **Selbstverifizierer** (`self_verifier`): Automatische Validierung der Argumentationsergebnisse mit Zykluserkennung
- **Fehlerbehebung** (`error_recovery_engine`): Fehlertypklassifizierung → Wiederherstellungsstrategie → automatischer Wiederholungsversuch oder Plananpassung, mit exponentiellem Backoff
- **A/B-Tests** (`ab_testing`): Vergleichende Bewertung verschiedener Argumentationsstrategien
- **Bewertungssystem** (`evaluator`): Integriertes Benchmark-Framework
- **LoRA-Fine-Tuning** (`fine_tune`): Integrierte Trainingspipeline mit LoRA-Adapterverwaltung
- **RL-Optimierer** (`rl_optimizer`): Erfahrungsbasiertes Policy-Reinforcement-Learning

**Multi-Agent-Zusammenarbeit**:

- Master-Slave-Koordinationsarchitektur mit paralleler Unteragentenausführung und abhängigkeitsbewusster Planung
- Gemeinsames Blackboard für agentenübergreifenden Informationsaustausch
- Adversarialer Debattenmodus (Pro/Contra-Runden mit Argumentstärkenbewertung)
- Swarm-Clustermodus für Multi-Prozess-Agentencluster
- Proaktiver Modus: Agenten können proaktiv Vorschläge und Operationen initiieren

**Computersteuerung**: KI-gesteuerte Mausklicks, Tastatureingabe, Bildschirmscrollen, mit drei Berechtigungsstufen (Standard / Änderungen akzeptieren / Vollzugriff) und Sandbox-Pfadisolierung

**Browser-Automatisierung**: Browsersteuerung über CDP-Protokoll, mit Navigation, Screenshots, Klicks, Formularausfüllung und Textextraktion

### Fähigkeitssystem

- **Fähigkeitsmarktplatz**: Community-Fähigkeiten durchsuchen und installieren
- **KI-gestützte Erstellung**: Automatische Erstellung von Fähigkeitsstrukturen aus natürlichsprachlichen Vorschlägen (`skill:create`)
- **Fähigkeitsevolution** (`evolution_engine`): Automatische Analyse und Verbesserung von Fähigkeiten basierend auf Ausführungsfeedback
- **Semantisches Matching**: Kontextabhängige semantische Fähigkeitsempfehlung
- **Fähigkeitszerlegung** (`skill_decomposition`): Automatische Zerlegung komplexer Aufgaben in atomare Fähigkeitskombinationen
- **Generierte Werkzeuge**: Von KI generierte und registrierte neue Werkzeuge
- **Sandbox-Ausführung**: Fähigkeiten werden in isolierten Sandbox-Umgebungen ausgeführt

### Visueller Workflow

Drag-and-Drop-DAG-Workflow-Editor basierend auf ReactFlow 12:

- **17 Knotentypen**: Auslöser, Agent, LLM-Aufruf, Bedingte Verzweigung, Paralleler Fork, Schleife, Zusammenführung, Verzögerung, Werkzeugaufruf, Codeausführung, Unterworkflow, Vektorsuche, Dokumentanalyse, Validierung, Ende, Geschäftsregel, Agentenrolle
- **Kahn-Topologische-Sortierungsausführung**: Automatische Zyklenerkennung, parallele Pipeline-Planung
- **Integrierte Vorlagen**: Code-Review, Bug-Fix, Dokumentation, Test, Refactoring, Exploration, Leistungsanalyse, Sicherheitsaudit, Feature-Entwicklung
- **YAML-Serialisierung**: Workflow-Definitions-Import/Export
- **Versionsverwaltung**: Vorlagenversionskontrolle
- **KI-gestütztes Design**: KI-gestütztes Workflow-Design und Knotenempfehlung

### Wissensmanagement

- **Multi-Wissensdatenbank-RAG**: Dokument-Upload → automatische Analyse (PDF/DOCX/XLSX/PPTX/TXT) → Chunking → Vektorindizierung
- **Hybride Suche**: Vektorähnlichkeit (sqlite-vec + candle lokale Embeddings) + BM25-Volltextsuche (FTS5), hybrides Ranking
- **Self-RAG**: Automatische Reflexion und Validierung von Suchergebnissen
- **Re-Ranking**: Cross-encoder-basierte Ergebnis-Neusortierung
- **Wissensgraph**: Entitätsextraktion → Beziehungsaufbau → visueller Graph
- **Dateiüberwachung**: Echtzeit-Dateiänderungsüberwachung via `notify`, automatische inkrementelle Indizierung
- **LLM Wiki**: KI-gestützter Wiki-Compiler und Validator

### Speichersystem

- **Multi-Namespace-Speicher**: Projekt-/Themenisolation, manuelle Eingabe und automatische KI-Extraktion
- **Persistente Integration**: Honcho und Mem0 Closed-Loop-Speicher
- **Benutzerprofil**: Automatisches Erlernen von Codierungsstil, Technologie-Stack-Präferenzen und Kommunikationsstil
- **Stilübertragung**: Extraktion von Codestilmerkmalen → Anwendung auf KI-generierten Code
- **Dream-Integration**: Automatische Hintergrundkonsolidierung von Speicherfragmenten und Verhaltensmustern in strukturiertes Wissen
- **Projektspeicher**: Projektbezogene Kontextpersistenz

### API-Gateway

Integriertes HTTP + WebSocket-Gateway basierend auf `axum`:

- **Kompatible Endpunkte**: OpenAI `/v1/chat/completions`, Claude Messages API, Gemini API, sowie OpenAI Responses und Realtime WebSocket
- **Schlüsselverwaltung**: Generierung, Widerruf, Aktivierung/Deaktivierung von Zugriffsschlüsseln mit Ablaufunterstützung
- **Nutzungsverfolgung**: Anfrage- und Token-Verbrauchsstatistiken pro Schlüssel/Provider/Datum, Prometheus-Metriken-Export
- **Ratenbegrenzung**: Token-Bucket-Algorithmus via `governor`
- **SSL/TLS**: Integrierte selbstsignierte Zertifikate (`rcgen`), benutzerdefinierte Zertifikatunterstützung
- **Externe Verknüpfung**: Ein-Klick-Integration mit Claude CLI, OpenCode und anderen externen Tools, automatische API-Schlüssel-Synchronisation
- **Echtzeit-Tickets**: HMAC-basierte temporäre Authentifizierungstickets für sichere WebSocket-Verbindungsübergabe

### Messaging-Plattform-Integration

Multi-Plattform-Gateway via `rt-messaging`, unterstützt Nachrichtenempfang, Befehlsanalyse und automatische KI-Antwort für **DingTalk, Feishu, QQ, Slack, WeChat, WhatsApp, Telegram und Discord**.

### Werkzeugsystem

47+ integrierte Werkzeuge, einheitlich über das `Tool`-Trait registriert:

| Kategorie          | Werkzeuge                                                                                                                                                                                                  |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Dateioperationen   | `file_read`, `file_write`, `file_edit`, `file_system`                                                                                                                                                      |
| Codeausführung     | `bash`, `repl`                                                                                                                                                                                             |
| Suche              | `grep`, `glob`                                                                                                                                                                                             |
| Browser            | `browser` (CDP)                                                                                                                                                                                            |
| Computersteuerung  | `computer_use` (Maus/Tastatur/Screenshot)                                                                                                                                                                  |
| Web                | `web_search`, `web_fetch`                                                                                                                                                                                  |
| Wissensdatenbank   | `knowledge`, `document`                                                                                                                                                                                    |
| Git                | `git` (commit/push/branch/diff)                                                                                                                                                                            |
| Dev-Werkzeuge      | `lsp`, `workspace`                                                                                                                                                                                         |
| Aufgabenverwaltung | `plan`, `task_system`, `todo_write`, `cron`                                                                                                                                                                |
| Messaging          | `push_notification`, `messaging`                                                                                                                                                                           |
| Datenbank          | `database`                                                                                                                                                                                                 |
| Speicher           | `storage`                                                                                                                                                                                                  |
| Sonstige           | `agent`, `agent_memory`, `context`, `export`, `integration`, `media`, `media_delivery`, `migration_tool`, `monitor`, `obsidian`, `ocr`, `personality`, `shared_path`, `system_info`, `testing`, `worktree` |

### MCP-Protokoll

Vollständige MCP (Model Context Protocol)-Implementierung basierend auf `rmcp`:

- **Transport**: stdio-Unterprozess + Streamable HTTP + WebSocket
- **OAuth-Authentifizierung**: OAuth-Autorisierungsfluss für MCP-Server
- **Werkzeugerkennung**: Automatische Erkennung und Registrierung von MCP-Server-exponierten Werkzeugen
- **MCP-Manager**: Server-Lebenszyklusverwaltung, Gesundheitsprüfungen, automatische Wiederverbindung

### Plugin-System

OpenClaw-kompatible dreistufige Plugin-Architektur (integriert / gebündelt / extern):

- npm-Paketinstallation mit Marketplace-UI zur Suche und Installation
- Plugin-Manifest-Definition, Berechtigungserklärung, Sandbox-isolierte Ausführung
- Benutzerdefinierte Werkzeugregistrierung, Agent-Provider, Hook-Interception
- Fähigkeitsinstaller: Installation von Fähigkeiten aus Plugin-Paketen

### Sicherheit

- **AES-256-GCM-Verschlüsselung**: Lokale verschlüsselte Speicherung von API-Schlüsseln und sensiblen Konfigurationen (`crypto`-Crate)
- **Prompt-Injection-Schutz**: Vierstufige Verteidigungspipeline (`prompt-guard`) — Mustererkennung → Delimiter-Escaping → XML-Wrapper → Vertrauenslabels, integriert in Konversationen, Prompt-Erstellung, Git und RAG
- **SSRF-Schutz**: URL-Sicherheitsprüfung zur Blockierung von Anfragen an interne Netzwerkadressen
- **Inhaltsfilterung**: Mehrtyp-Inhaltssicherheitsfilterung
- **Ratenbegrenzung**: Token-Bucket-Begrenzung für Werkzeugaufrufe und API-Anfragen
- **Schutzschalter**: Automatische Unterbrechung bei aufeinanderfolgenden Fehlern
- **Zugriffskontrolle**: Richtlinienbasierte Werkzeugzugriffsberechtigungskontrolle
- **Sandbox-Isolierung**: Isolierte Ausführungsumgebungen für Agenten und Fähigkeiten

### Entwicklerwerkzeuge

- **Verteiltes Tracing** (`telemetry`): OpenTelemetry-Integration mit Span/Trace-Visualisierung
- **Strukturiertes Logging**: tracing-subscriber + chrono-Zeitstempel
- **Replay-Debugging**: Aufzeichnung von Agentenausführungstrajektorien (`trajectory_recorder`) und Wiedergabe
- **DevTools-Panel**: Trace Explorer Timeline-Viewer, Benchmark Runner, Tool Recommender
- **Benchmarks**: Criterion-Benchmarks (tool_exec / llm_call / search)
- **CI-Prüfungen**: `npm run ci:check` integriert Typprüfung, Linting und Formatvalidierung

### Desktop- und Mobile-Erfahrung

- **Responsive Layout**: CSS-Breakpoint-basierte Anpassung für Desktop/Tablet/Mobile (3 Gerätestufen: `desktop` / `tablet` / `mobile`)
- **11 Sprachen**: Vereinfachtes Chinesisch, Traditionelles Chinesisch, Englisch, Japanisch, Koreanisch, Französisch, Deutsch, Spanisch, Russisch, Hindi, Arabisch
- **Theme-Engine** (`rt-theme`): Dunkle/helle Themes + mehrere Voreinstellungen (einschließlich 21th Monospace-Theme), tiefgehende Anpassung mit Ant Design 6
- **Monaco-Editor**: Syntaxhervorhebung, Diff-Vorschau, Mehrsprachenunterstützung
- **xterm.js-Terminal**: WebLinks, Unicode 11, Suche
- **Virtuelles Scrollen**: @tanstack/react-virtual + react-virtuoso
- **Diagramm-Rendering**: D2 + Mermaid + Recharts
- **Globales Kopiermenü**: Benutzerdefiniertes Textauswahl-Kopiermenü, natives Kontextmenü unterdrückt
- **Befehlspalette**: Ctrl+K globale Befehlspalette
- **System-Tray + Globale Tastenkürzel + Autostart**: Nicht-intrusiver Hintergrundbetrieb
- **Automatische Updates**: Konfigurierbare Intervall-basierte GitHub Releases-Versionsprüfung
- **Proxy-Unterstützung**: HTTP / SOCKS5-Proxy-Konfiguration
- **Cloud-Arbeitsbereich**: S3- und WebDAV-Speichersynchronisation mit Konflikterkennung und bidirektionaler Synchronisation

### Mobile

- Android APK/AAB (arm64-v8a, armeabi-v7a, x86_64)
- iOS IPA (arm64)
- Mobilspezifische Anpassungen: Safe Area Insets, untere Navigation, Drawer-Navigation

---

## Technische Architektur

### Tech-Stack

| Schicht             | Technologie                              | Version |
| ------------------- | ---------------------------------------- | ------- |
| Desktop-Framework   | Tauri                                    | 2.11    |
| Frontend-Framework  | React                                    | 19      |
| Typsystem           | TypeScript                               | 7       |
| UI-Bibliothek       | Ant Design                               | 6       |
| CSS-Framework       | TailwindCSS                              | 4       |
| Zustandsverwaltung  | Zustand                                  | 5       |
| Routing             | React Router                             | 7       |
| Code-Editor         | Monaco Editor                            | 0.55    |
| Terminal            | xterm.js                                 | 6       |
| Workflow-Editor     | ReactFlow                                | 12      |
| Diagramme           | D2 + Mermaid + Recharts                  |         |
| Animation           | Framer Motion                            | 12      |
| Virtuelles Scrollen | @tanstack/react-virtual + react-virtuoso |         |
| Drag & Drop         | @dnd-kit                                 | 6       |
| Markdown-Rendering  | markstream-react + stream-markdown       |         |
| i18n                | i18next + react-i18next                  |         |
| Build-Werkzeug      | Vite                                     | 8       |
| Tests               | Vitest + Playwright                      |         |
| Formatierung        | dprint (TS/JSON/Markdown/TOML) + rustfmt |         |
| Linting             | ESLint + Oxlint + Clippy                 |         |

### Backend-Architektur: Harness Dependency Injection

Rust-Workspace-Architektur mit **32 Crates**, nach dem **Harness DI-Muster**:

> Alle Crates sind durch die von axagent-harness definierten Trait-Schnittstellen entkoppelt, und axagent-runtime assembliert und injiziert Abhängigkeiten zur Laufzeit.
> Abhängigkeitsrichtung: `konkrete Implementierungen → harness ← Aufrufer`

**harness** ist der architektonische Grundstein — null Geschäftslogik, null konkrete Implementierungen, enthält nur Trait-Definitionen, reine Daten-DTOs, Konstanten und einheitliche Fehlertypen. Es wird von allen anderen Crates abhängig gemacht und hängt selbst von keinem axagent-*-Crate ab (200+ Trait-Definitionen, die Agent/Provider/Tool/RAG/Speicher/MCP/Plugins/Sicherheit/Observability/Speicher/Lernen/Browser/Messaging usw. abdecken).

```
src-tauri/crates/
├── harness/          # Architektonischer Grundstein — Trait-Schnittstellen, DTOs, Fehlertypen, DI-Verträge
├── entities/         # SeaORM-Entitätsmodelle
├── dao/              # Datenzugriffsschicht (CRUD)
├── migration/        # Datenbankmigrationen
├── crypto/           # AES-256-GCM Ver-/Entschlüsselung und Schlüsselverwaltung
├── credential/       # Sichere Anmeldeinformationsspeicherung
├── storage/          # Dateispeicherabstraktion (lokal/S3/WebDAV), ZIP-Lesen/Schreiben
├── cache/            # In-Memory-Cache-Schicht
├── disk-cache/       # Festplatten-Dateicache
├── search/           # Suchmaschine (FTS5 + sqlite-vec + candle lokale Embeddings)
├── document-parser/  # Dokumenttextextraktion (PDF/DOCX/XLSX/PPTX)
├── kit/              # Allgemeine Hilfsprogramme (Pfade/Encoding/Hashing/Daten)
├── runtime-core/     # Laufzeit-Allgemeintypen, Konfigurationskonstanten
├── runtime/          # Laufzeit-Service-Orchestrierung — DI-Container, der alle 30+ Crates assembliert
├── rt-workflow/      # Workflow-Engine — DAG-Orchestrierung, Knotenausführer, YAML-Serialisierung
├── rt-messaging/     # Messaging-Plattform-Gateway — DingTalk/Feishu/QQ/Slack/WeChat/WhatsApp/Telegram/Discord
├── rt-webhook/       # Allgemeiner Webhook-Server
├── rt-dashboard/     # Dashboard-Plugin-Framework
├── rt-theme/         # Theme-Engine
├── agent/            # KI-Agent-Kern — 80+ Module
│                     #   ReAct-Engine/HierarchischePlanung/Tiefenrecherche/Faktenprüfung/Gedankenbaum/
│                     #   Reflexion/Selbstverifizierung/Fehlerbehebung/RL-Optimierung/LoRA-Fine-Tuning/
│                     #   Bewertung/Werkzeugempfehlung/A-B-Tests/Koordinator/Blackboard/Vision-Pipeline/
│                     #   Web-Suche/AkademischeSuche/Wiki-Kompilierung usw.
├── orchestrator/     # Agentenorchestrierung — Multi-Agenten-Planung, DAG-Zerlegung, dynamische Untergraphausführung
├── providers/        # Modell-Provider-Adapter
├── tools/            # Werkzeugsystem — Tool-Trait/Registry/Orchestrierung/Streaming/Sandbox/47+ integrierte Werkzeuge
├── gateway/          # API-Gateway — axum HTTP/WS-Server, OAuth, Ratenbegrenzung, Prometheus
├── mcp/              # MCP-Protokoll — stdio + Streamable HTTP, basierend auf rmcp
├── trajectory/       # Lernsystem — Speicher/Fähigkeitsevolution/Benutzerprofile/Dream-Integration
├── plugins/          # Plugin-System — OpenClaw-kompatibel, npm-Paketinstallation, Marketplace
├── telemetry/        # Observability — OpenTelemetry, strukturiertes Logging, Laufzeitmetriken
├── prompt-guard/     # Prompt-Injection-Schutz — L1-L4 mehrstufige Erkennungspipeline
├── npm/              # npm-Registry-Client
└── schema-gen/       # Datenbankschema-Generierungswerkzeug
```

### Frontend-Architektur

```
src/
├── pages/            # Seiten (23+ einschließlich Unterseiten)
│   ├── ChatPage           # Chat-Oberfläche — Seitenleiste/Nachrichtenstrom/Agent-Panel/Multi-Tab
│   ├── DashboardPage      # Dashboard — Nutzungsstatistiken/Modellverteilung/Trenddiagramme
│   ├── WorkflowPage       # Workflow-Editor — ReactFlow DAG-Visualisierung
│   ├── KnowledgeHubPage   # Wissensdatenbankverwaltung — Dokument-Upload/Indizierung/Suche
│   ├── MemoryPage         # Speicherverwaltung
│   ├── SkillsPage         # Fähigkeitsmarktplatz
│   ├── SettingsPage       # Einstellungspanel — 40+ Konfigurationselemente
│   ├── TerminalPage       # Integriertes Terminal — xterm.js
│   ├── FilesPage          # Dateiverwaltung
│   ├── GatewayLinkPage    # API-Gateway & externe Verknüpfungsverwaltung
│   ├── QuickBarPage       # Schnellleiste (eigenständiges Fenster)
│   ├── DynamicUIManagerPage / DynamicPageViewer  # Dynamische UI-Engine
│   ├── WikiGraphPage / WikiEditPage / IngestPage # LLM Wiki
│   ├── LearningGraphPage  # Lerngraph
│   ├── FineTunePage       # LoRA-Fine-Tuning
│   ├── PersonaPage        # Persona-Verwaltung
│   ├── WorkflowMarketplace # Workflow-Marktplatz
│   ├── DevTools/          # TraceExplorer / BenchmarkRunner / ToolRecommender
│   └── Workflow/          # WorkflowListPage
│
├── components/       # 28 Module, 450+ Komponenten
│   ├── chat/         # Chat (Nachrichtenstrom/Eingabe/ChatView/TabBar/RightPanel/Anhänge/Werkzeugaufruf-Rendering)
│   ├── layout/       # Layout — 17 Komponenten
│   │                 #   AppInitializer / Sidebar / ContentArea / TitleBar /
│   │                 #   CommandPalette / GlobalCopyMenu / GlobalErrorBoundary /
│   │                 #   GlobalStatusBar / ErrorNotificationToast / AppHeader /
│   │                 #   BackendStatusIndicator / IpcReconnectBanner /
│   │                 #   ModuleErrorBoundary / NotificationBell / UserProfileModal usw.
│   ├── agent/        # Agent-Panel/Einstieg/Mini-Panel
│   ├── workflow/     # Workflow-Editor (Knoten/Kanten/Panels/Vorlagen/KI-Unterstützung)
│   ├── settings/     # Einstellungspanel (40+ Unterkomponenten)
│   ├── skill/        # Fähigkeitseditor/Renderer/Floating-Panels
│   ├── dynamicUI/    # Dynamische UI-Komponenten-Registry (26 integrierte Komponenten)
│   ├── gateway/      # API-Gateway-Verwaltung
│   ├── files/        # Dateiverwaltung
│   ├── terminal/     # Terminal-Komponenten
│   ├── search/       # Suchoberfläche
│   ├── benchmark/    # Benchmark-Panel
│   ├── decomposition/# Fähigkeitszerlegung & Werkzeuggenerierung
│   ├── devtools/     # Trace/Span-Timeline + RL Training-Panel
│   ├── approval/     # Genehmigungs-Workflow-UI
│   ├── recommendation/ # Werkzeug-/Modell-Empfehlung
│   ├── onboarding/   # WelcomeWizard / InteractiveTutorial
│   ├── help/         # Hilfe-Panel
│   ├── notification/ # Benachrichtigungskomponenten
│   ├── proactive/    # Proaktive Vorschläge
│   ├── llm-wiki/     # LLM Wiki-Komponenten
│   ├── wiki/         # Wiki-Komponenten
│   ├── fine-tune/    # Fine-Tuning-UI
│   ├── trace/        # Trace-Komponenten
│   ├── style/        # Stil/Theme
│   ├── shared/       # Gemeinsame Komponenten (ErrorBoundary / PageContextProvider)
│   └── common/       # Allgemeine Komponenten (Icon usw.)
│
├── stores/           # Zustand-Zustandsverwaltung
│   ├── domain/       # 10 Kern-Geschäfts-Stores (Konversation/Stream/Komprimierung/Einstellungen/Multi-Modell usw.)
│   ├── feature/      # 48 Funktionsmodul-Stores (Agent/Workflow/Wissen/Fähigkeiten/Gateway/Speicher/Terminal usw.)
│   └── devtools/     # 4 Entwicklerwerkzeug-Stores
│
├── hooks/            # React Hooks (Tastenkürzel/Befehlspalette/Responsive/Scrollbar/Theme/Avatar usw.)
├── lib/              # Hilfsbibliothek (invoke/pageRegistry/shortcuts/skillLifecycle/
│                     #   chartGenerator/codeExecutor/tokenEstimator/workflowLayout usw. — 45+ Module)
├── types/            # TypeScript-Typdefinitionen
├── theme/            # Shadcn-Theme-Engine
├── i18n/             # 11-Sprachen-Übersetzungsdateien (zh-CN/zh-TW/en-US/ja/ko/fr/de/es/ru/hi/ar)
├── constants/        # Konstanten & Feature-Flags
└── sdk/              # Externes Integrations-SDK
```

### Feature-Flags

Das Projekt verwaltet progressive Feature-Rollouts über `featureFlags.ts`:

| Flag                | Status | Beschreibung                                           |
| ------------------- | ------ | ------------------------------------------------------ |
| `AGENT_IN_THE_LOOP` | ✅     | Globales Agent-Panel + Seitenkontextinjektion          |
| `DYNAMIC_UI`        | ✅     | Dynamische UI-Builder-Engine                           |
| `SELF_EVOLUTION_UI` | ❌     | Selbstentwicklungs-Frontend-Steuerpanel                |
| `NL_EXTENSION`      | ❌     | Natürlichsprachliche dynamische Geschäftserweiterungen |

### Tauri-Plugins

| Plugin              | Zweck                               |
| ------------------- | ----------------------------------- |
| `autostart`         | Automatischer Start beim Booten     |
| `clipboard-manager` | Zwischenablage Lesen/Schreiben      |
| `dialog`            | Dateiauswahldialoge                 |
| `fs`                | Dateisystemzugriff                  |
| `global-shortcut`   | Globale Tastenkürzel-Registrierung  |
| `notification`      | Systembenachrichtigungen            |
| `opener`            | Externe Links/Dateien öffnen        |
| `process`           | Prozessverwaltung                   |
| `updater`           | Automatische Updates                |
| `mcp-bridge`        | MCP-Protokollbrücke (nicht-Android) |

---

## Datenverzeichnis

```
~/.axagent/                    # Anwendungskonfiguration
├── axagent.db                 # SQLite-Hauptdatenbank (SeaORM)
├── master.key                 # AES-256-Master-Schlüssel
├── vector_db/                 # sqlite-vec Vektorindex
└── ssl/                       # Selbstsignierte SSL-Zertifikate

~/Documents/axagent/          # Benutzerdateien
├── images/                   # Bildanhänge
├── files/                    # Dateianhänge
└── backups/                  # Automatische Backups
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
npm run tauri dev      # Entwicklungsmodus (Vite HMR + Tauri-Fenster)
```

### Build

```bash
npm run tauri build    # Desktop-Produktions-Build

npm run tauri:android:build   # Android-Build
npm run tauri:ios:build       # iOS-Build
```

Desktop-Build-Artefakte befinden sich in `src-tauri/target/release/`.

### Tests

```bash
npm run test           # Frontend-Unit-Tests (Vitest Watch)
npm run test:run       # Frontend-Unit-Tests (Einzelausführung)
npm run test:e2e       # E2E-Tests (Playwright)

# Rust-Backend-Tests
cd src-tauri && cargo nextest run
cd src-tauri && cargo test

# Typprüfung & Linting
npm run typecheck
cd src-tauri && cargo clippy -- -D warnings
npm run format         # dprint-Formatierung
npm run lint:eslint    # ESLint-Prüfung
npm run contracts      # API-Vertragsprüfung

# Vollständige CI-Prüfung
npm run ci:check
```

### Skripte

| Befehl                   | Zweck                             |
| ------------------------ | --------------------------------- |
| `npm run bump`           | Interaktive Versionserhöhung      |
| `npm run docs`           | TypeDoc-Dokumentation generieren  |
| `npm run skill:create`   | Neues Fähigkeits-Gerüst erstellen |
| `npm run skill:validate` | Fähigkeitsdefinition validieren   |
| `npm run check:types`    | Typkonsistenzprüfung              |

---

## Plattformunterstützung

| Plattform | Architektur                           |
| --------- | ------------------------------------- |
| Windows   | x86_64, ARM64                         |
| macOS     | Apple Silicon (arm64), Intel (x86_64) |
| Linux     | x86_64, ARM64                         |
| Android   | arm64-v8a, armeabi-v7a, x86_64        |
| iOS       | arm64                                 |

---

## Lizenz

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
