[English](./README-EN.md) | [简体中文](./README.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | **Deutsch** | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

[![AxAgent](https://github.com/polite0803/AxAgent/blob/main/src/assets/image/logo.png?raw=true)](https://github.com/polite0803/AxAgent)

<p align="center">
  <a href="https://www.producthunt.com/products/axagent?embed=true&amp;amp;utm_source=badge-featured&amp;amp;&amp;#10;&amp;amp;amp;;utm_medium=badge&amp;amp;#10&amp;amp;;utm_campaign=badge-axagent" target="_blank" rel="noopener noreferrer"><img alt="AxAgent - Lightweight, high-perf cross-platform AI desktop client | Product Hunt" width="250" height="54" src="https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=1118403&amp;theme=light&amp;t=1775627359538"></a>
</p>

<p align="center">
  <strong>Plattformübergreifender AI-Desktop-/Mobilclient | Multi-Agenten-Kollaboration | Lokal zuerst</strong>
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

## Was ist AxAgent?

**AxAgent v2.0** ist eine funktionsreiche plattformübergreifende AI-Desktop-/Mobilanwendung, die fortschrittliche AI-Agenten-Fähigkeiten mit umfangreichen Entwicklerwerkzeugen integriert. Sie unterstützt mehrere Modellanbieter, autonome Pipeline-Ausführung, visuelle Workflow-Orchestrierung, lokales Wissensmanagement und ein integriertes API-Gateway und deckt **Windows / macOS / Linux / Android / iOS** ab.

---

## Screenshots

| Chat und Modellauswahl | Multi-Agenten-Dashboard |
|:---:|:---:|
| ![](.github/images/s1-0412.png) | ![](.github/images/s5-0412.png) |

| Wissensbasis RAG | Speicher und Kontext |
|:---:|:---:|
| ![](.github/images/s3-0412.png) | ![](.github/images/s4-0412.png) |

| Workflow-Editor | API-Gateway |
|:---:|:---:|
| ![](.github/images/s9-0412.png) | ![](.github/images/s10-0412.png) |

---

## Kernfunktionen

### 🤖 AI-Modellunterstützung

- **Multi-Anbieter-Unterstützung** — Native Integration von OpenAI, Anthropic Claude, Google Gemini, Ollama, OpenClaw, Hermes und allen OpenAI-kompatiblen APIs
- **Multi-Key-Rotation** — Mehrere API-Schlüssel pro Anbieter konfigurieren mit automatischer Rotation zur Verteilung des Rate-Limit-Drucks
- **Lokale Modellunterstützung** — Vollständige Unterstützung für Ollama lokale Modelle, einschließlich GGUF/GGML-Dateiverwaltung
- **Candle-Inferenz-Engine** — Eingebaute Candle-Lokalinferenz mit Rerank/Judge-Schnittstellen und On-Demand-GGUF-Downloads
- **Modellverwaltung** — Remote-Modelllisten abrufen, Parameter anpassen (Temperatur, maximale Tokens, Top-P usw.)
- **Streaming-Ausgabe** — Echtzeit-Token-für-Token-Rendering mit einklappbaren Denkblöcken (Claude Extended Thinking)
- **Multi-Modell-Vergleich** — Gleichzeitige Frage an mehrere Modelle mit Side-by-Side-Vergleich
- **Funktionsaufrufe** — Strukturierte Funktionsaufrufe über alle unterstützten Anbieter
- **OpenAI Responses API** — Unterstützung für das OpenAI Responses-Format-Transport
- **Realtime API** — WebSocket-Ereignis-Push kompatibel mit der OpenAI Realtime API

### 🔐 AI-Agenten-System

Das Agentensystem basiert auf einer anspruchsvollen Architektur mit folgenden Eigenschaften:

- **ReAct-Reasoning-Engine** — Verschmelzung von Reasoning und Aktion mit integrierter Selbstverifikation für zuverlässige Aufgabenausführung
- **Hierarchischer Planer** — Zerlegung komplexer Aufgaben in strukturierte Pläne mit Phasen und Abhängigkeiten
- **Aufgabenzerleger** — Automatische Zerlegung komplexer Aufgaben in ausführbare Teilaufgaben
- **Tiefenrecherche** — Multi-Quellen-Suchorchesterung, Zitationsverfolgung und Glaubwürdigkeitsbewertung
- **Faktenprüfung** — KI-gesteuerte Faktenverifikation und Quellenklassifizierung
- **Suchorchesterung** — Koordination mehrerer Suchanbieter mit Suchplanung und Ergebnissynthese
- **Akademische Suche** — Akademische Literaturrecherche und Zitationsanalyse
- **Computersteuerung** — KI-gesteuerte Mausklicks, Tastatureingaben, Bildlauf mit visuellem Modell-Analyse
- **Bildschirmwahrnehmung** — Screenshot-Erfassung und visuelle Modell-Analyse zur UI-Element-Identifikation
- **Drei Berechtigungsstufen** — Standard (Genehmigung erforderlich), Bearbeitungen akzeptieren (automatische Genehmigung), Vollzugriff (keine Abfragen)
- **Sandbox-Isolation** — Agenten-Operationen sind strikt auf das angegebene Arbeitsverzeichnis beschränkt
- **Werkzeug-Genehmigungspanel** — Echtzeit-Anzeige von Werkzeugaufruf-Anfragen mit einzelner Genehmigung
- **Kostenverfolgung** — Echtzeit-Token-Nutzung und Kostenstatistiken pro Sitzung
- **Pause/Fortsetzen** — Agenten-Ausführung jederzeit anhalten und später fortsetzen
- **Checkpoint-System** — Persistente Checkpoints für Absturzwiederherstellung und Sitzungs-Wiederverbindung
- **Fehlerwiederherstellungs-Engine** — Automatische Fehlerklassifizierung, Ursachenanalyse und Wiederherstellungsstrategie-Ausführung
- **Schleifenerkennung** — Automatische Erkennung und Unterbrechung von Schleifenverhalten im Agenten-Reasoning
- **Gedankenkette** — Visualisierung der Agenten-Entscheidungsfindung, schrittweise Zerlegung
- **Proaktiver Modus** — Agenten können proaktiv Vorschläge machen und Aktionen ausführen
- **Zweckverwaltung** — Pflege und Verfolgung der Ausführungszwecke und des Kontexts des Agenten

### 👥 Multi-Agenten-Kollaboration

- **Sub-Agenten-Koordination** — Master-Slave-Architektur mit Unterstützung mehrerer kollaborativer Agenten
- **Parallele Ausführung** — Parallele Verarbeitung durch mehrere Agenten mit abhängigkeitsbewusster Planung
- **Adversariale Debatte** — Pro/Contra-Debattenrunden mit Argumentstärke-Bewertung und Widerlegungsverfolgung
- **Agenten-Rollen** — Vordefinierte Rollen (Forscher, Planer, Entwickler, Prüfer, Synthetisierer) für Teamzusammenarbeit
- **Agenten-Orchestrator** — Zentrales Nachrichten-Routing und Zustandsverwaltung für Multi-Agenten-Teams
- **Kommunikationsgraph** — Visualisierung von Agenten-Interaktionen und Nachrichtenflüssen
- **Swarm-Cluster** — Multi-Prozess-Agenten-Cluster mit Berechtigungssynchronisation und automatischer Wiederverbindung
- **Buddy-System** — Konfigurierbare Agenten-Partner mit Spezies- und Attributdefinition
- **Gemeinsamer Speicher** — Agenten-übergreifender gemeinsamer Speicherplatz mit Statistiken und Abfragen
- **Team-Cron** — Teamweite Cron-Aufgabenplanung

### ⭐ Skill-System

- **Skill-Marktplatz** — Integrierter Marktplatz zum Durchsuchen und Installieren von Community-Skills
- **Skill-Erstellung** — Automatische Skill-Erstellung aus Vorschlägen mit Markdown-Editor
- **Skill-Evolution** — KI-gesteuerte automatische Analyse und Verbesserung bestehender Skills basierend auf Ausführungsfeedback
- **Skill-Matching** — Semantische Übereinstimmung, Empfehlung relevanter Skills zum Gesprächskontext
- **Skill-Zerlegung** — Automatische Zerlegung komplexer Aufgaben in ausführbare atomare Skills (LLM-unterstützt/Multi-Runde/Workflow-Validierung)
- **Generierte Werkzeuge** — KI-gesteuerte automatische Generierung und Registrierung neuer Werkzeuge zur Erweiterung der Agenten-Fähigkeiten
- **Skill-Hub** — Zentrale Skill-Entdeckung und Konfigurationsverwaltungsoberfläche
- **Skill-Hub-Client** — Integration mit Remote-Skill-Hub mit Community-Sharing
- **Skill-Abhängigkeitsprüfung** — Automatische Erkennung von Skill-Abhängigkeiten und Werkzeugverfügbarkeit
- **Skill-Sandbox-Container** — Sichere Ausführung von Skills in einer isolierten Umgebung

### 🔄 Workflow-System

Die Workflow-Engine implementiert ein DAG-basiertes Aufgaben-Orchestrierungssystem:

- **Visueller Workflow-Editor** — Drag-and-Drop-Workflow-Designer mit Knotenverbindung und -konfiguration
- **Umfangreiche Knotentypen** — 15 Knotentypen: Trigger, Agent, LLM, Bedingung, Parallel, Schleife, Merge, Verzögerung, Werkzeug, Code, Sub-Workflow, Vektorsuche, Dokumentanalyse, Validierung, Ende
- **Workflow-Vorlagen** — Integrierte Voreinstellungen: Code-Review, Bug-Fix, Dokumentation, Tests, Refactoring, Exploration, Performance, Sicherheit, Feature-Entwicklung
- **DAG-Ausführung** — Topologische Sortierung nach Kahn-Algorithmus mit Zykluserkennung
- **Parallele Planung** — Pipeline-Ausführung, schnelle Schritte warten nicht auf langsame
- **Wiederholungsstrategie** — Exponentielles Backoff, konfigurierbare maximale Wiederholungsversuche pro Schritt
- **Teilabschluss** — Fehlgeschlagene Schritte blockieren keine unabhängigen nachgelagerten Schritte
- **Versionsverwaltung** — Versionskontrolle von Workflow-Vorlagen mit Rollback
- **Ausführungsverlauf** — Detaillierte Aufzeichnung mit Statusverfolgung und Debugging
- **KI-Unterstützung** — KI-gestütztes Workflow-Design, Knotenempfehlung und Agenten-Prompt-Optimierung
- **Semantische Prüfung** — Semantische Workflow-Validierung, Erkennung potenzieller Probleme
- **n8n-Import** — Unterstützung für Workflow-Import aus n8n-Verzeichnis
- **Debug-Panel** — Echtzeit-Debugging und Statusanzeige während der Workflow-Ausführung

### 📚 Wissen und Speicher

- **Wissensbasis (RAG)** — Multi-Wissensbasis-Unterstützung, Dokument-Upload, automatisches Parsen, Chunking und Vektorindexierung
- **Hybride Suche** — Kombination aus Vektorähnlichkeitssuche und BM25-Volltext-Ranking
- **Reranking** — Cross-Encoder-Reranking zur Verbesserung der Abrufgenauigkeit
- **Dreistufige Recall-Pipeline** — Mehrstufiger Abrufmechanismus mit AST-Index + Vektorsuche + FTS5
- **Wissensgraph** — Wissensentitäts-Beziehungsvisualisierung (Entitäten, Attribute, Beziehungen, Flüsse, Schnittstellen)
- **Wiki-System** — LLM-Wiki-Compiler und -Validator mit Wissensgraph-Visualisierung und inkrementeller Synchronisation
- **Wiki-Notizen** — Bidirektionales Link-Notizsystem mit Graphansicht und automatischer Link-Synchronisation
- **Speichersystem** — Multi-Namespace-Speicher mit manuellem Eintrag oder KI-gesteuerter automatischer Extraktion
- **Closed-Loop-Speicher** — Integration der persistenten Speicheranbieter Honcho und Mem0
- **FTS5-Volltextsuche** — Schnelle Suche über Gespräche, Dateien und Speicher
- **Sitzungssuche** — Erweiterte Suche über alle Gesprächssitzungen
- **Kontextverwaltung** — Flexibles Anhängen von Dateien, Suchergebnissen, Wissenspassagen, Speichereinträgen, Werkzeugausgaben
- **Dokument-Parser** — Automatisches Parsen und Inhaltsextraktion von Multi-Format-Dokumenten
- **Inkrementelle Indexierung** — Inkrementelle Indexaktualisierung bei Dateiänderungen

### 🌐 API-Gateway

- **Lokaler API-Server** — Integrierter OpenAI-kompatibler, Claude- und Gemini-Schnittstellenserver
- **Externe Links** — One-Click-Integration mit Claude CLI, OpenCode, automatische API-Schlüssel- und Modellsynchronisation
- **Schlüsselverwaltung** — Generierung, Widerruf, Aktivierung/Deaktivierung von Zugriffsschlüsseln mit Beschreibungen
- **Nutzungsanalyse** — Anfragevolumen und Token-Nutzung nach Schlüssel, Anbieter und Datum
- **SSL/TLS-Unterstützung** — Integrierte selbstsignierte Zertifikate, Unterstützung für benutzerdefinierte Zertifikate
- **Anfrage-Logs** — Vollständige Aufzeichnung aller API-Anfragen und -Antworten
- **Konfigurationsvorlagen** — Vorgefertigte Vorlagen für Claude, Codex, OpenCode, Gemini
- **Realtime API** — WebSocket-Ereignis-Push kompatibel mit der OpenAI Realtime API
- **Plattform-Integration** — Unterstützung für DingTalk, Feishu, QQ, Slack, WeChat, WhatsApp, Telegram, Discord
- **Gateway-Diagnose** — Verbindungsdiagnose und Programmrichtlinienverwaltung
- **Rate-Limiter** — API-Anfragen-Ratenbegrenzung und Flusskontrolle
- **Persistente Warteschlange** — Persistente Anfragewarteschlangenverwaltung

### 🔧 Werkzeuge und Erweiterungen

- **MCP-Protokoll** — Vollständige Model Context Protocol-Implementierung mit stdio- und HTTP/WebSocket-Transporten
- **OAuth-Authentifizierung** — OAuth-Flow-Unterstützung für MCP-Server
- **MCP-Autostart** — Automatischer Start und Lebenszyklusverwaltung von MCP-Servern
- **MCP-Werkzeug-Bridge** — Brücke zwischen MCP-Werkzeugen und dem Agenten-Werkzeugsystem
- **Plugin-System** — OpenClaw-kompatible Drei-Stufen-Plugin-Architektur (integriert/gebündelt/extern) mit npm-Paketinstallation, Werkzeugregistrierung, Hooks und Lebenszyklusverwaltung
- **Plugin-Marktplatz** — Integrierte Marktplatz-UI mit npm-Suche, Installation und Bestätigungsdialogen
- **Integrierte Werkzeuge** — Umfassende Dateioperationen (Lesen/Schreiben/Bearbeiten), Codeausführung, Suche (Grep/Glob), Bash, Websuche, Web-Fetch, Planverwaltung, Cron-Planung, REPL, LSP, Kontextverwaltung, Computersteuerung, Nachrichtenversand, Todo-Liste usw.
- **Werkzeug-Berechtigungssystem** — Werkzeug-Berechtigungsklassifizierung, Regelverwaltung und Nutzungsverfolgung
- **Bash-Sicherheit** — Befehlsanalyse, Pfadvalidierung und Sandbox-Sicherheitskontrolle
- **LSP-Client** — Integriertes Language Server Protocol mit Code-Vervollständigung und Diagnose
- **AST-Index** — AST-Parsing und Indexerstellung für Codedateien
- **Terminal-Backend** — Unterstützung für lokale, Docker- und SSH-Terminalverbindungen
- **Browser-Automatisierung** — Browsersteuerung über CDP-Integration (Navigation, Screenshots, Klicks, Formularausfüllung, Textextraktion usw.)
- **UI-Automatisierung** — Plattformübergreifende UI-Element-Identifikation und -Steuerung
- **Git-Werkzeuge** — Git-Operationen mit Branch-Erkennung und Konfliktbewusstsein
- **Werkzeug-Empfehlung** — Kontextbasiertes intelligentes Werkzeug-Empfehlungssystem
- **Werkzeug-Orchestrierung** — Multi-Werkzeug-Koordinationsausführung mit Streaming-Ausgabe
- **Werkzeug-Statistiken** — Werkzeug-Nutzungshäufigkeit und Leistungsstatistiken

### 📊 Inhaltsrendering

- **Markdown-Rendering** — Vollständige Unterstützung für Code-Hervorhebung, LaTeX-Mathematikformeln, Tabellen, Aufgabenlisten
- **Monaco Code-Editor** — Integrierter Editor mit Syntaxhervorhebung, Kopieren, Diff-Vorschau
- **Diagramm-Rendering** — Mermaid-Flussdiagramme, D2-Architekturdiagramme, ECharts-interaktive Diagramme
- **Artefakt-Panel** — Codeausschnitte, HTML-Entwürfe, React-Komponenten, Markdown-Notizen mit Echtzeitvorschau
- **Vier Vorschaumodi** — Code (Editor), Split (Side-by-Side), Vorschau (nur gerendert), React-Komponentenvorschau
- **Sitzungs-Inspektor** — Baumansicht der Sitzungsstruktur, schnelle Navigation
- **Zitations-Panel** — Verfolgung und Anzeige von Quellenzitationen mit Glaubwürdigkeitsbewertung
- **Infografik-Rendering** — Unterstützung für Infografik-Visualisierung

### 🛡️ Daten und Sicherheit

- **AES-256-Verschlüsselung** — API-Schlüssel und sensible Daten mit AES-256-GCM verschlüsselt
- **Isolierte Speicherung** — Anwendungsstatus in `~/.axagent/`, Benutzerdateien in `~/Documents/axagent/`
- **Automatisches Backup** — Geplante Backups in lokale Verzeichnisse oder WebDAV-Speicher
- **Backup-Wiederherstellung** — Ein-Klick-Wiederherstellung aus historischen Backups
- **Export-Optionen** — PNG-Screenshots, Markdown, Klartext, JSON
- **Speicherverwaltung** — Visuelle Plattennutzungsanzeige und Bereinigungstools
- **Dateiautorisierung** — Dateizugriffsautorisierung und -widerrufverwaltung
- **Operations-Audit** — Audit-Log-Erfassung kritischer Operationen

### 🖥️ Desktop-Erfahrung

- **Themen-Engine** — Dunkle/helle Themen, Systemfolge oder manuelle Präferenz
- **Oberflächensprache** — 11 Sprachen: Vereinfachtes Chinesisch, Traditionelles Chinesisch, Englisch, Japanisch, Koreanisch, Französisch, Deutsch, Spanisch, Russisch, Hindi, Arabisch
- **Systemtray** — Minimierung in den Systemtray ohne Unterbrechung von Hintergrunddiensten
- **Immer im Vordergrund** — Fenster über allen anderen Fenstern anheften
- **Globale Tastenkürzel** — Anpassbare Tastenkürzel zum Aufrufen des Hauptfensters
- **QuickBar** — Schnellzugriff-Schwebelleiste, Ein-Klick-Aufruf
- **Autostart** — Optionaler Start beim Systemstart
- **Proxy-Unterstützung** — HTTP- und SOCKS5-Proxy-Konfiguration
- **Automatische Updates** — Automatische Versionsprüfung, Update-Benachrichtigung
- **Befehlspalette** — `Cmd/Ctrl+K` für schnellen Zugriff auf Befehle
- **Onboarding-Assistent** — Interaktiver Erstnutzungs-Assistent und Ollama-Erkennung
- **Benachrichtigungscenter** — Unified In-App-Benachrichtigungsverwaltung

### 🔬 Erweiterte Funktionen

- **Tiefenrecherche** — Multi-Quellen-Suche, Zitationsverfolgung, Glaubwürdigkeitsbewertung und Inhaltssynthese
- **Faktenprüfung** — KI-gesteuerte Faktenverifikation und Quellenklassifizierung
- **Cron-Planer** — Automatisierte Aufgabenplanung mit täglichen/wöchentlichen/monatlichen Vorlagen und benutzerdefinierten Cron-Ausdrücken
- **Webhook-System** — Ereignisabonnement, Werkzeugabschluss-, Agentenfehler-, Sitzungsende-Benachrichtigungen
- **Benutzerprofil** — Automatisches Lernen von Code-Stil, Namenskonventionen, Einrückung, Kommentarstil, Kommunikationspräferenzen
- **RL-Optimierer** — Reinforcement-Learning-Optimierung der Werkzeugauswahl und Aufgabenstrategien
- **LoRA-Feinabstimmung** — Benutzerdefinierte Modelladaption mit lokalem LoRA-Feintuning
- **Proaktive Vorschläge** — Kontextbewusste Hinweise basierend auf Gesprächsinhalt und Benutzermustern
- **Kontextvorhersage** — Vorhersage der nächsten Benutzeraktion und Vorabladen relevanter Ressourcen
- **Traum-Integration** — Automatische Hintergrund-Integration von Speicher und Mustern, Optimierung von Langzeitwissen
- **Fehlerwiederherstellung** — Automatische Fehlerklassifizierung, Ursachenanalyse und Wiederherstellungsvorschläge
- **Entwicklerwerkzeuge** — Trace, Span, Timeline-Visualisierung für Debugging und Performance-Analyse
- **Benchmark-System** — SWE-bench / Terminal-bench Leistungsbewertung und Metriken mit Scorecards
- **Stiltransfer** — Anwendung gelernter Code-Stil-Präferenzen auf generierten Code
- **Dashboard-Plugins** — Erweiterbares Dashboard mit benutzerdefinierten Panels und Widgets
- **Kollaboration und Freigabe** — CRDT-Echtzeit-Kollaboration und Ein-Klick-Sitzungsfreigabe
- **Browser-Erweiterung** — Wiki Clipper Browser-Erweiterung zum schnellen Clipping von Webseiten ins LLM-Wiki
- **Python SDK** — Python SDK zur Integration mit AxAgent
- **Smarter Router** — Intelligentes Routing und Klassifizierung von Anfragen
- **Semantischer Cache** — Semantikbasierter Antwort-Cache zur Reduzierung redundanter Berechnungen
- **Kontextkompression** — Automatische Kompression langer Kontexte, Optimierung der Token-Nutzung
- **Nachrichten-Batching** — Nachrichten-Stapelversand und -optimierung
- **Verbindungspool** — Datenbank- und API-Verbindungspool-Verwaltung
- **Feature Flags** — Konfigurierbares Feature-Flag-System
- **Policy-Engine** — Zentrale Verwaltung von Berechtigungs- und Operationsrichtlinien
- **Ressourcen-Governor** — Agenten-Ressourcennutzungslimitierung und -Governance
- **LAN-Transfer** — Lokale Netzwerk-Dateiübertragungsfähigkeit

### 🛡️ Prompt-Injection-Schutz (Prompt-Guard)

- **Vier-Stufen-Schutz** — L1 Mustererkennung (Hochrisiko-Blockierung + Mittleres-Risiko-Markierung) → L2 Trennzeichen-Escaping → L3 XML-Wrapper → L4 Vertrauens-Tags
- **Pipeline-Orchestrator** — Mehrstufige Erkennungspipeline mit anpassbaren Risikoschwellen
- **Token-Smuggling-Erkennung** — Spezialisierte Erkennung von Encoding-Verschleierung und Token-Smuggling-Angriffen
- **Strict-Modus** — Strict-Modus-Tests + Mittleres-Risiko-Benennung + Benutzerdefinierte-Modus-Dokumentation
- **Volle Pipeline-Integration** — Integriert in Session / Prompt / Git / RAG-Workflows

### 📱 Mobile Unterstützung

- **Android Nativ** — APK/AAB-Builds, Unterstützung für arm64-v8a / armeabi-v7a / x86_64
- **iOS Nativ** — IPA-Builds, Unterstützung für arm64
- **Adaptives Layout** — Drei-Stufen-Auto-Anpassung Desktop/Tablet/Telefon
- **Mobile Navigation** — Drawer-Slide-Navigation + untere Navigationsleiste + Flash-FAB
- **Safe-Area-Anpassung** — Android-System-Statusleiste/Navigationsleiste CSS env()-Anpassung
- **CSP-Optimierung** — Android WebView CSP-Protokoll-Whitelist

---

## Technische Architektur

### Technologie-Stack

| Schicht | Technologie |
|---------|------------|
| **Framework** | Tauri 2 + React 19 + TypeScript 6 |
| **UI** | Ant Design 6 + TailwindCSS 4 |
| **Zustandsverwaltung** | Zustand 5 |
| **Routing** | React Router 7 |
| **i18n** | i18next + react-i18next |
| **Backend** | Rust + SeaORM 2 + SQLite |
| **Vektor-DB** | sqlite-vec |
| **Code-Editor** | Monaco Editor |
| **Diagramme** | Mermaid + D2 + ECharts (CDN) |
| **Terminal** | xterm.js 6 |
| **Workflow** | ReactFlow 11 |
| **Infografik** | @antv/infographic |
| **Icons** | Iconify + Lucide |
| **Drag & Drop** | @dnd-kit |
| **Build** | Vite 8 + npm |
| **Tests** | Vitest + Playwright + cargo-nextest |
| **Formatierung** | dprint (TS/JSON) + rustfmt |
| **Lint** | TS: eslint + oxlint / Rust: clippy + cargo-deny |
| **Mobil** | Tauri Android + iOS native Builds |
| **Desktop** | Windows (MSI) · macOS (DMG) · Linux (AppImage/deb/rpm) |

### Plattformunterstützung

| Plattform | Architektur |
|-----------|------------|
| Windows | x86_64, ARM64 |
| macOS | Apple Silicon (arm64), Intel (x86_64) |
| Linux | x86_64, ARM64 |
| Android | arm64-v8a, armeabi-v7a, x86_64 (Emulator) |
| iOS | arm64 |

### Rust-Backend-Architektur

Das Backend ist als Rust-Workspace mit **18** spezialisierten Crates organisiert:

```
src-tauri/crates/
├── agent/            # KI-Agenten-Kern (ReAct-Engine, Koordination, Planung, Tiefenrecherche, Faktenprüfung usw.)
├── core/             # Kernprogramme (Datenbank, RAG, Verschlüsselung, MCP, Browser-Automatisierung, AST-Index usw.)
├── providers/        # Modellanbieter-Adapter (OpenAI, Anthropic, Gemini, Ollama, OpenClaw usw.)
├── runtime-core/     # Laufzeit-Abstraktionsschicht (gemeinsame Typen, Trait-Definitionen, Konfiguration)
├── runtime/          # Laufzeitdienste (Sitzungsverwaltung, MCP, Terminal, Rate-Limiter, Webhook, Berechtigungen usw.)
├── rt-workflow/      # Workflow-Engine (DAG-Orchestrierung, Knoten-Executors, Scheduler)
├── rt-messaging/     # Nachrichten-Gateway (DingTalk/Feishu/QQ/Slack/WeChat/WhatsApp/Telegram/Discord)
├── rt-webhook/       # Webhook-Server und Versand
├── rt-dashboard/     # Dashboard-Plugin-System
├── rt-theme/         # Themen-Engine
├── gateway/          # API-Gateway (HTTP-Server, Authentifizierung, Routing, OpenAI-kompatible Schnittstelle)
├── tools/            # Werkzeugsystem (Registry, Orchestrierung, Streaming-Ausgabe, 40+ integrierte Werkzeuge)
├── trajectory/       # Lernsystem (Speicher, Skills, RL, Benutzerprofil, Traum-Integration)
├── telemetry/        # Telemetrie und verteiltes Tracing
├── plugins/          # Plugin-System (OpenClaw-kompatibel, npm-Paketinstallation)
├── prompt-guard/     # Prompt-Injection-Schutz (L1-L4 mehrstufige Erkennung und Abwehr)
├── migration/        # Datenbankmigrationen
├── npm/              # npm-Paket-Parsing und Registry
└── code_engine/      # Candle-Lokalinferenz-Engine (veraltet, Funktionalität in core integriert)
```

### Frontend-Architektur

```
src/
├── stores/                    # Zustand State Management
│   ├── domain/               # Kerngeschäftslogik-State
│   │   ├── conversationStore.ts
│   │   ├── messageStore.ts
│   │   ├── streamStore.ts
│   │   ├── multiModelStore.ts
│   │   ├── preferenceStore.ts
│   │   └── compressStore.ts
│   ├── feature/               # Feature-Modul-State (30+ Stores)
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
│   ├── devtools/              # Entwicklerwerkzeug-State
│   │   ├── tracerStore.ts
│   │   ├── evaluatorStore.ts
│   │   ├── rlStore.ts
│   │   ├── fineTuneStore.ts
│   │   └── recommendationStore.ts
│   └── shared/                # Geteilter State
│       ├── artifactStore.ts
│       ├── chatWorkspaceStore.ts
│       ├── tabStore.ts
│       └── uiStore.ts
│
├── components/                # React-Komponenten (24 Module)
│   ├── chat/                # Chat-Interface (90+ Komponenten)
│   ├── workflow/            # Workflow-Editor (Knoten/Panels/Vorlagen/KI-Unterstützung)
│   ├── gateway/             # API-Gateway-UI
│   ├── settings/            # Einstellungs-Panels (40+ Komponenten)
│   ├── terminal/            # Terminal-UI
│   ├── skill/               # Skill-Editor und -Renderer
│   ├── benchmark/           # Benchmark-Panel
│   ├── decomposition/       # Skill-Zerlegung und Werkzeuggenerierung
│   ├── files/               # Dateiverwaltungsseite
│   ├── fine-tune/           # LoRA-Feinabstimmungs-Konfiguration
│   ├── link/                # Externe Links-Verwaltung
│   ├── llm-wiki/            # LLM-Wiki-Editor
│   ├── proactive/           # Proaktives Vorschlagssystem
│   ├── recommendation/      # Werkzeug-Empfehlungs-Panel
│   ├── wiki/                # Wiki-Verwaltung
│   ├── devtools/            # Trace/Span-Timeline
│   ├── style/               # Code-Stiltransfer
│   ├── layout/              # Layout-Komponenten (Titelleiste/Sidebar/Befehlspalette)
│   ├── help/                # Hilfe-Panel
│   ├── onboarding/          # Onboarding-Assistent
│   ├── notification/        # Benachrichtigungscenter
│   ├── search/              # Sitzungssuche
│   ├── common/              # Gemeinsame Komponenten
│   └── shared/              # Geteilte Komponenten
│
├── pages/                    # Seitenkomponenten (22 Seiten)
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
├── hooks/                    # React-Hooks (10)
├── lib/                      # Hilfsfunktionen (inkl. Web Worker)
├── types/                    # TypeScript-Typdefinitionen (22)
├── sdk/                      # SDK (inkl. Python SDK)
└── i18n/                     # 11-Sprach-Übersetzungen
```

## Erste Schritte

### Vorab-Builds herunterladen

Besuchen Sie die [Releases](https://github.com/polite0803/AxAgent/releases)-Seite und laden Sie das Installationsprogramm für Ihre Plattform herunter.

### Aus dem Quellcode erstellen

#### Voraussetzungen

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+
- [npm](https://www.npmjs.com/) 10+
- Windows: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) + Rust MSVC-Ziele

#### Build-Schritte

```bash
# Repository klonen
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent

# Abhängigkeiten installieren
npm install

# Entwicklungsmodus
npm run tauri dev

# Nur Frontend bauen
npm run build

# Desktop-Anwendung bauen
npm run tauri build
```

Build-Artefakte befinden sich in `src-tauri/target/release/`.

### Tests

```bash
# Unit-Tests
npm run test          # Vitest Watch-Modus
npm run test:run      # Vitest Einzeldurchlauf

# E2E-Tests
npm run test:e2e      # Playwright
npm run test:e2e:ui   # Playwright UI-Modus

# Rust-Backend-Tests
cd src-tauri && cargo nextest run   # cargo-nextest (2-3x schneller)
cd src-tauri && cargo test          # Standard-Tests

# Typprüfung
npm run typecheck     # TypeScript
cd src-tauri && cargo clippy -- -D warnings  # Rust

# Code-Formatierung
npm run format        # dprint
cd src-tauri && cargo fmt

# CI-Vollprüfung
npm run ci:check
```

---

## Projektstruktur

```
AxAgent/
├── src/                         # Frontend-Quellcode (React + TypeScript)
│   ├── components/              # React-Komponenten (24 Module)
│   │   ├── chat/               # Chat-Interface (90+ Komponenten)
│   │   ├── workflow/           # Workflow-Editor-Komponenten
│   │   ├── gateway/            # API-Gateway-Komponenten
│   │   ├── settings/           # Einstellungs-Panels (40+ Komponenten)
│   │   ├── terminal/           # Terminal-Komponenten
│   │   ├── skill/              # Skill-Editor und -Renderer
│   │   ├── benchmark/          # Benchmarks
│   │   ├── decomposition/      # Skill-Zerlegung
│   │   ├── files/              # Dateiverwaltung
│   │   ├── fine-tune/          # LoRA-Feinabstimmung
│   │   ├── link/               # Externe Links
│   │   ├── llm-wiki/           # LLM-Wiki
│   │   ├── proactive/          # Proaktive Vorschläge
│   │   ├── recommendation/     # Werkzeug-Empfehlung
│   │   ├── wiki/               # Wiki-Verwaltung
│   │   ├── devtools/           # Entwicklerwerkzeuge
│   │   ├── style/              # Code-Stil
│   │   ├── layout/             # Layout-Komponenten
│   │   ├── help/               # Hilfe-Panel
│   │   ├── onboarding/         # Onboarding-Assistent
│   │   ├── notification/       # Benachrichtigungscenter
│   │   ├── search/             # Sitzungssuche
│   │   ├── common/             # Gemeinsame Komponenten
│   │   └── shared/             # Geteilte Komponenten
│   ├── pages/                   # Seitenkomponenten (18 Seiten)
│   ├── stores/                  # Zustand State Management (62 Stores)
│   │   ├── domain/            # Kerngeschäftslogik-State (9 Stores)
│   │   ├── feature/           # Feature-Modul-State (44 Stores)
│   │   ├── devtools/          # Entwicklerwerkzeug-State (5 Stores)
│   │   └── shared/            # Geteilter State (4 Stores)
│   ├── hooks/                   # React-Hooks
│   ├── lib/                     # Hilfsfunktionen (inkl. Web Worker)
│   ├── types/                   # TypeScript-Typdefinitionen
│   ├── sdk/                     # SDK (inkl. Python SDK)
│   └── i18n/                    # 11-Sprach-Übersetzungen
│
├── src-tauri/                    # Backend-Quellcode (Rust)
│   ├── crates/                  # Rust-Workspace (18 Crates)
│   │   ├── agent/             # KI-Agenten-Kern
│   │   ├── core/              # Datenbank, Verschlüsselung, RAG, MCP
│   │   ├── providers/         # Modellanbieter-Adapter
│   │   ├── runtime-core/      # Laufzeit-Abstraktionsschicht
│   │   ├── runtime/           # Laufzeitdienste
│   │   ├── rt-workflow/       # Workflow-Engine
│   │   ├── rt-messaging/      # Nachrichten-Gateway
│   │   ├── rt-webhook/        # Webhook-Server
│   │   ├── rt-dashboard/      # Dashboard-Plugins
│   │   ├── rt-theme/          # Themen-Engine
│   │   ├── gateway/           # API-Gateway-Server
│   │   ├── tools/             # Werkzeugsystem
│   │   ├── trajectory/        # Speicher und Lernen
│   │   ├── telemetry/         # Tracing und Metriken
│   │   ├── plugins/           # Plugin-System
│   │   ├── prompt-guard/      # Prompt-Injection-Schutz
│   │   ├── migration/         # Datenbankmigrationen
│   │   └── npm/               # npm-Paket-Parsing
│   └── src/                    # Tauri-Einstiegspunkt (70+ Befehlsmodule)
│
├── extension/                  # Browser-Erweiterung (Wiki Clipper)
├── e2e/                        # Playwright E2E-Tests
├── scripts/                    # Build- und Werkzeugskripte
└── website/                    # Projekt-Website (VitePress)
```

## Datenverzeichnis

```
~/.axagent/                      # Konfigurationsverzeichnis
├── axagent.db                   # SQLite-Datenbank
├── master.key                   # AES-256-Hauptschlüssel
├── vector_db/                   # Vektordatenbank (sqlite-vec)
└── ssl/                         # SSL-Zertifikate

~/Documents/axagent/            # Benutzerdateiverzeichnis
├── images/                     # Bildanhänge
├── files/                      # Dateianhänge
└── backups/                    # Backup-Dateien
```

---

## FAQ

### macOS: „App ist beschädigt" oder „Entwickler kann nicht überprüft werden"

Da die Anwendung nicht von Apple signiert ist:

**1. Apps aus „Beliebiger Herkunft" zulassen**
```bash
sudo spctl --master-disable
```

Gehen Sie dann zu **Systemeinstellungen → Datenschutz & Sicherheit → Sicherheit** und wählen Sie **Beliebige Herkunft**.

**2. Das Quarantäne-Attribut entfernen**
```bash
sudo xattr -dr com.apple.quarantine /Applications/AxAgent.app
```

**3. Zusätzlicher Schritt für macOS Ventura+**
Gehen Sie zu **Systemeinstellungen → Datenschutz & Sicherheit** und klicken Sie auf **Trotzdem öffnen**.

---

## Community

- [LinuxDO](https://linux.do)

## Lizenz

Dieses Projekt ist unter der [AGPL-3.0](LICENSE)-Lizenz lizenziert.
