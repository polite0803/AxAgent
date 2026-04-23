[简体中文](./README.md) | [繁體中文](./README-ZH-TW.md) | [English](./README-EN.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | **Deutsch** | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

[![AxAgent](https://github.com/polite0803/AxAgent/blob/main/src/assets/image/logo.png?raw=true)](https://github.com/polite0803/AxAgent)

<p align="center">
    <a href="https://www.producthunt.com/products/axagent?embed=true&amp;utm_source=badge-featured&amp;utm_medium=badge&amp;utm_campaign=badge-axagent" target="_blank" rel="noopener noreferrer"><img alt="AxAgent - Lightweight, high-perf cross-platform AI desktop client | Product Hunt" width="250" height="54" src="https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=1118403&amp;theme=light&amp;t=1775627359538"></a>
</p>

## Screenshots

| Chat-Diagramm-Rendering | Anbieter und Modelle |
|:---:|:---:|
| ![](.github/images/s1-0412.png) | ![](.github/images/s2-0412.png) |

| Wissensdatenbank | Gedächtnis |
|:---:|:---:|
| ![](.github/images/s3-0412.png) | ![](.github/images/s4-0412.png) |

| Agent - Anfrage | API-Gateway Ein-Klick-Zugang |
|:---:|:---:|
| ![](.github/images/s5-0412.png) | ![](.github/images/s6-0412.png) |

| Chat-Modell-Auswahl | Chat-Navigation |
|:---:|:---:|
| ![](.github/images/s7-0412.png) | ![](.github/images/s8-0412.png) |

| Agent - Berechtigungsgenehmigung | API-Gateway-Übersicht |
|:---:|:---:|
| ![](.github/images/s9-0412.png) | ![](.github/images/s10-0412.png) |

## Funktionen

### Chat & Modelle

- **Multi-Anbieter-Unterstützung** — Kompatibel mit OpenAI, Anthropic Claude, Google Gemini und allen OpenAI-kompatiblen APIs; auch Ollama für lokale Modelle, OpenClaw/Hermes für Remote-Gateway-Verbindungen
- **Modellverwaltung** — Remote-Modelllisten abrufen, Parameter anpassen (Temperatur, maximale Tokens, Top-P usw.)
- **Multi-Key-Rotation** — Mehrere API-Schlüssel pro Anbieter konfigurieren mit automatischer Rotation zur Verteilung des Rate-Limit-Drucks
- **Streaming-Ausgabe** — Echtzeit-Token-für-Token-Rendering mit einklappbaren Denkblöcken
- **Nachrichtenversionen** — Zwischen mehreren Antwortversionen pro Nachricht wechseln, um Modell- oder Parametereffekte zu vergleichen
- **Gesprächsverzweigung** — Neue Zweige von einem beliebigen Nachrichtenknoten erstellen, mit seitenweisem Zweigvergleich
- **Gesprächsverwaltung** — Anheften, Archivieren, zeitgruppierte Anzeige und Massenoperationen
- **Gesprächskomprimierung** — Lange Gespräche automatisch komprimieren und dabei wichtige Informationen beibehalten, um Kontextraum zu sparen
- **Simultane Multi-Modell-Antwort** — Dieselbe Frage gleichzeitig an mehrere Modelle stellen, mit seitenweisem Antwortvergleich
- **Kategorie-System** — Benutzerdefinierte Gesprächskategorien mit themenbasierter Organisation

### AI Agent

- **Agent-Modus** — Wechseln Sie in den Agent-Modus für die autonome Ausführung mehrstufiger Aufgaben: Dateien lesen/schreiben, Befehle ausführen, Code analysieren und mehr
- **Drei Berechtigungsstufen** — Standard (Schreibvorgänge erfordern Genehmigung), Bearbeitungen akzeptieren (Dateiänderungen automatisch genehmigen), Vollzugriff (keine Abfragen) — sicher und kontrollierbar
- **Arbeitsverzeichnis-Sandbox** — Agent-Operationen sind strikt auf das angegebene Arbeitsverzeichnis beschränkt, um unbefugten Zugriff zu verhindern
- **Werkzeug-Genehmigungspanel** — Echtzeit-Anzeige von Werkzeugaufruf-Anfragen mit einzelner Überprüfung, Ein-Klick „Immer erlauben" oder Ablehnen
- **Kostenverfolgung** — Echtzeit-Token-Nutzung und Kostenstatistiken pro Sitzung
- **Pause/Fortsetzen** — Agent-Aufgaben jederzeit zur Überprüfung anhalten und dann fortsetzen
- **Bash-Befehlsausführung** — Shell-Befehle in Sandbox-Umgebung mit automatischer Risikovalidierung ausführen

### Multi-Agent-System

- **Sub-Agent-Koordination** — Mehrere Sub-Agents erstellen mit Master-Slave-Koordinationsarchitektur
- **Parallele Ausführung** — Mehrere Agents parallel verarbeiten für verbesserte Effizienz bei komplexen Aufgaben
- **Adversariale Debatte** — Mehrere Agents debattieren unterschiedliche Standpunkte um durch Ideenkollision bessere Lösungen zu finden
- **Workflow-Engine** — Leistungsstarke Workflow-Orchestrierung mit Unterstützung für Bedingungsverzweigungen, Schleifen und parallele Ausführung
- **Team-Rollen** — Verschiedenen Agents spezifische Rollen zuweisen (Code-Review, Tests, Dokumentation, etc.) für kooperative Aufgabenbearbeitung

### Skill-System

- **Skill-Marktplatz** — Integrierter Marktplatz zum Durchsuchen und Installieren von Community-beigesteuerten Skills
- **Skill-Erstellung** — Automatische Skill-Erstellung aus Vorschlägen mit Markdown-Editor-Unterstützung
- **Skill-Evolution** — AI analysiert und verbessert vorhandene Skills automatisch für bessere Ausführung
- **Skill-Matching** — Intelligente Empfehlungen zur automatischen Anwendung relevanter Skills auf passende Gesprächsszenarien
- **Lokaler Skill-Registrierung** — Benutzerdefinierte lokale Tools als wiederverwendbare Skills registrieren
- **Plugin-Hooks** — Pre/post Hooks unterstützt um benutzerdefinierte Logik vor und nach der Skill-Ausführung einzufügen

### Inhaltsrendering

- **Markdown-Rendering** — Vollständige Unterstützung für Code-Hervorhebung, LaTeX-Mathematikformeln, Tabellen und Aufgabenlisten
- **Monaco Code-Editor** — Monaco Editor in Codeblöcken eingebettet mit Syntaxhervorhebung, Kopieren und Diff-Vorschau
- **Diagramm-Rendering** — Integriertes Rendering von Mermaid-Flussdiagrammen und D2-Architekturdiagrammen
- **Artifact-Panel** — Codeausschnitte, HTML-Entwürfe, Markdown-Notizen und Berichte in einem dedizierten Panel anzeigen
- **Sitzungs-Inspektor** — Echtzeit-Anzeige der Sitzungsstruktur als Baumansicht für schnelle Navigation zu einer beliebigen Nachricht

### Suche & Wissen

- **Websuche** — Integriert mit Tavily, Zhipu WebSearch, Bocha und mehr, mit Quellenangaben
- **Lokale Wissensbasis (RAG)** — Unterstützt mehrere Wissensbasen; Dokumente hochladen für automatisches Parsen, Chunking und Vektorindexierung, mit semantischer Abrufung während Gesprächen
- **Wissensgraph** — Wissensentitäts-Beziehungsgraphen, die Verbindungen zwischen Wissenspunkten visualisieren
- **Gedächtnissystem** — Multi-Namespace-Gedächtnis mit manuellem Eintrag oder KI-gestützter automatischer Extraktion wichtiger Informationen
- **Volltextsuche** — FTS5-Engine für schnelle Suche über Gespräche, Dateien und Erinnerungen
- **Kontextverwaltung** — Flexibles Anhängen von Dateianhängen, Suchergebnissen, Wissensbasisabschnitten, Gedächtniseinträgen und Werkzeugausgaben

### Werkzeuge & Erweiterungen

- **MCP-Protokoll** — Vollständige Model Context Protocol-Implementierung mit Unterstützung für stdio- und HTTP/WebSocket-Transporte
- **OAuth-Authentifizierung** — OAuth-Authentifizierungsfluss-Unterstützung für MCP-Server
- **Integrierte Werkzeuge** — Sofort einsatzbereite integrierte Werkzeuge für Dateioperationen, Codeausführung, Suche und mehr
- **Werkzeugausführungs-Panel** — Visuelle Anzeige von Werkzeugaufruf-Anfragen und zurückgegebenen Ergebnissen
- **LSP-Client** — Integrierte LSP-Protokoll-Unterstützung für intelligente Code-Vervollständigung und Diagnose

### API-Gateway

- **Lokales API-Gateway** — Integrierter lokaler API-Server mit nativer Unterstützung für OpenAI-kompatible, Claude- und Gemini-Schnittstellen
- **Externe Links** — One-Click-Integration mit externen Tools wie Claude CLI und OpenCode mit automatischer API-Schlüssel-Synchronisation
- **API-Schlüsselverwaltung** — Zugriffsschlüssel generieren, widerrufen und aktivieren/deaktivieren mit Beschreibungsnotizen
- **Nutzungsanalyse** — Anfragevolumen und Token-Nutzungsanalyse nach Schlüssel, Anbieter und Datum
- **Diagnose-Tools** — Gateway-Gesundheitsprüfungen, Verbindungstests und Anfrage-Debugging
- **SSL/TLS-Unterstützung** — Integrierte Generierung selbstsignierter Zertifikate, mit Unterstützung für benutzerdefinierte Zertifikate
- **Anfrage-Logs** — Vollständige Aufzeichnung aller API-Anfragen und -Antworten, die das Gateway passieren
- **Konfigurationsvorlagen** — Vorgefertigte Integrationsvorlagen für beliebte CLI-Tools wie Claude, Codex, OpenCode und Gemini
- **Echtzeit-Kommunikation** — WebSocket-Echtzeit-Ereignispush, kompatibel mit OpenAI Realtime API

### Daten & Sicherheit

- **AES-256-Verschlüsselung** — API-Schlüssel und sensible Daten werden lokal mit AES-256-GCM verschlüsselt
- **Isolierte Datenverzeichnisse** — Anwendungsstatus in `~/.axagent/`; Benutzerdateien in `~/Documents/axagent/`
- **Automatisches Backup** — Geplante automatische Backups in lokale Verzeichnisse oder WebDAV-Speicher
- **Backup-Wiederherstellung** — Ein-Klick-Wiederherstellung aus historischen Backups
- **Gesprächsexport** — Gespräche als PNG-Screenshots, Markdown, Klartext oder JSON exportieren
- **Speicherplatz-Verwaltung** — Visuelle Anzeige der Plattennutzung mit Bereinigung unnötiger Dateien

### Desktop-Erfahrung

- **Themenwechsel** — Dunkle/helle Themes, die den Systemeinstellungen folgen oder manuell festgelegt werden können
- **Oberflächensprache** — Vollständige Unterstützung für vereinfachtes Chinesisch, traditionelles Chinesisch, Englisch, Japanisch, Koreanisch, Französisch, Deutsch, Spanisch, Russisch, Hindi und Arabisch
- **Systemtray** — Beim Schließen des Fensters in den Systemtray minimieren, ohne Hintergrunddienste zu unterbrechen
- **Immer im Vordergrund** — Das Hauptfenster über allen anderen Fenstern anheften
- **Globale Tastenkürzel** — Anpassbare globale Tastaturkürzel, um das Hauptfenster jederzeit aufzurufen
- **Autostart** — Optionaler Start beim Systemstart
- **Proxy-Unterstützung** — HTTP- und SOCKS5-Proxy-Konfiguration
- **Automatische Updates** — Prüft beim Start automatisch auf neue Versionen und fordert zur Aktualisierung auf
- **Befehlspalette** — `Cmd/Ctrl+K` für schnellen Zugriff auf alle Befehle und Einstellungen

## Plattformunterstützung

| Plattform | Architektur |
|-----------|------------|
| macOS | Apple Silicon (arm64), Intel (x86_64) |
| Windows 10/11 | x86_64, arm64 |
| Linux | x86_64 (AppImage/deb/rpm), arm64 (AppImage/deb/rpm) |

## Erste Schritte

Gehen Sie zur [Releases](https://github.com/polite0803/AxAgent/releases)-Seite und laden Sie das Installationsprogramm für Ihre Plattform herunter.

## Aus dem Quellcode erstellen

### Voraussetzungen

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+
- [npm](https://www.npmjs.com/) 10+
- Windows erfordert [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) und [Rust MSVC targets](https://doc.rust-lang.org/cargo/reference/config.html#cfgtarget)

### Build-Schritte

```bash
# Repository klonen
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent

# Abhängigkeiten installieren
npm install

# Im Entwicklungsmodus ausführen
npm run tauri dev

# Nur Frontend bauen
npm run build

# Desktop-Anwendung bauen
npm run tauri build
```

Build-Artefakte befinden sich im `src-tauri/target/release/`-Verzeichnis.

### Tests

```bash
# Unit-Tests ausführen
npm test

# End-to-End-Tests ausführen
npm run test:e2e

# Typprüfung
npm run typecheck
```

## FAQ

### macOS: „App ist beschädigt" oder „Entwickler kann nicht überprüft werden"

Da die Anwendung nicht von Apple signiert ist, kann macOS eine der folgenden Meldungen anzeigen:

- „AxAgent" ist beschädigt und kann nicht geöffnet werden
- „AxAgent" kann nicht geöffnet werden, da Apple es nicht auf Schadsoftware überprüfen kann

**Lösungsschritte:**

**1. Apps aus „Beliebiger Herkunft" zulassen**

```bash
sudo spctl --master-disable
```

Gehen Sie dann zu **Systemeinstellungen → Datenschutz & Sicherheit → Sicherheit** und wählen Sie **Beliebige Herkunft**.

**2. Das Quarantäne-Attribut entfernen**

```bash
sudo xattr -dr com.apple.quarantine /Applications/AxAgent.app
```

> Tipp: Sie können das App-Symbol in das Terminal ziehen, nachdem Sie `sudo xattr -dr com.apple.quarantine ` eingegeben haben.

**3. Zusätzlicher Schritt für macOS Ventura und höher**

Nach Abschluss der obigen Schritte kann der erste Start immer noch blockiert werden. Gehen Sie zu **Systemeinstellungen → Datenschutz & Sicherheit** und klicken Sie im Sicherheitsbereich auf **Trotzdem öffnen**. Dies muss nur einmal durchgeführt werden.

## Community
- [LinuxDO](https://linux.do)

## Lizenz

Dieses Projekt ist unter der [AGPL-3.0](LICENSE)-Lizenz lizenziert.