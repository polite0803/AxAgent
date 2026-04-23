# Funktionen

AxAgent ist ein voll ausgestatteter Desktop-KI-Assistent, der Multi-Provider-Chat, leistungsstarkes Content-Rendering, Tool-Integration und ein integriertes API-Gateway kombiniert — alles läuft lokal mit starker Datensicherheit.

## Chat und Modelle

Verbinden Sie sich über eine einzige, einheitliche Oberfläche mit den führenden KI-Anbietern.

- **Multi-Provider-Unterstützung** — Kompatibel mit OpenAI, Anthropic Claude, Google Gemini und allen OpenAI-kompatiblen APIs.
- **Modellverwaltung** — Automatisches Abrufen von Remote-Modelllisten und Anpassen der Generierungsparameter pro Gespräch.
- **Multi-Key-Rotation** — Konfigurieren Sie mehrere API-Schlüssel pro Anbieter mit automatischer Rotation.
- **Streaming-Ausgabe** — Echtzeit-Token-für-Token-Rendering. Einklappbare Denkblöcke zur Überprüfung des Modell-Reasonings.
- **Nachrichtenversionen** — Jede Antwort kann mehrere Versionen haben. Vergleichen Sie verschiedene Modelle oder Parametereinstellungen nebeneinander.
- **Gesprächsverzweigung** — Erstellen Sie einen neuen Zweig von jedem Nachrichtenknoten. Nebeneinander-Vergleichsansicht für Zweige.
- **Gesprächsverwaltung** — Wichtige Gespräche anheften, alte archivieren, zeitgruppierte Historie durchsuchen.
- **Gesprächskomprimierung** — Lange Gespräche automatisch komprimieren und wichtige Informationen bewahren.
- **Multi-Modell-Simultanantwort** — Stellen Sie dieselbe Frage gleichzeitig mehreren Modellen mit Nebeneinander-Vergleich.

## AI Agent

AxAgent enthält einen integrierten Agent-Modus, der es der KI ermöglicht, mehrstufige Aufgaben autonom mit feingranularer Berechtigungskontrolle auszuführen.

- **Agent-Modus** — Wechseln Sie jedes Gespräch in den Agent-Modus für autonome Aufgabenausführung. Die KI kann Dateien lesen und schreiben, Shell-Befehle ausführen, Code analysieren und komplexe mehrstufige Workflows durchführen — alles in einer kontrollierten Umgebung
- **Drei Berechtigungsstufen** — Wählen Sie die richtige Sicherheitsstufe für Ihren Workflow:
  - **Standard** — Leseoperationen werden automatisch genehmigt; Schreibvorgänge und Befehlsausführung erfordern explizite Benutzergenehmigung
  - **Bearbeitungen akzeptieren** — Datei-Lese- und Schreibvorgänge werden automatisch genehmigt; Befehlsausführung erfordert weiterhin Genehmigung
  - **Vollzugriff** — Alle Operationen werden ohne Abfragen ausgeführt (Pfad-Sicherheitsprüfungen bleiben aktiv)
- **Arbeitsverzeichnis-Sandbox** — Alle Dateioperationen des Agents sind strikt auf das angegebene Arbeitsverzeichnis beschränkt. Pfadtraversierung, Symlink-Escapes und Zugriff außerhalb der Sandbox werden auf Systemebene blockiert
- **Werkzeug-Genehmigungspanel** — Jeder Werkzeugaufruf wird in Echtzeit mit seinen Parametern angezeigt. Überprüfen Sie jede Anfrage einzeln, klicken Sie auf „Immer erlauben", um Ihre Entscheidung zu speichern, oder lehnen Sie nicht vertrauenswürdige Operationen ab
- **Kostenverfolgung** — Überwachen Sie den Token-Verbrauch und die geschätzten USD-Kosten in Echtzeit für jede Agent-Sitzung

::: tip Beta-Funktion
Der Agent-Modus befindet sich derzeit in der Beta-Phase. Er unterstützt OpenAI, Anthropic und Gemini-Modelle über open-agent-sdk.
:::

## Content-Rendering

AxAgent geht weit über einfachen Textchat hinaus mit einer reichhaltigen, interaktiven Rendering-Pipeline.

- **Markdown-Rendering** — Vollständige Unterstützung für syntaxhervorgehobene Codeblöcke, LaTeX-Matheformeln, Tabellen und Aufgabenlisten.
- **Monaco-Code-Editor** — Codeblöcke betten den Monaco-Editor (die Engine hinter VS Code) ein.
- **Diagramm-Rendering** — Integriertes Rendering für Mermaid-Flussdiagramme und D2-Architekturdiagramme.
- **Artifact-Panel** — Code-Snippets, HTML-Entwürfe, Markdown-Notizen und Berichte können in einem dedizierten Seitenbereich geöffnet werden.
- **Echtzeit-Sprach-Chat** — (Demnächst) WebRTC-basierte Sprachgespräche über die OpenAI Realtime API.

## Suche und Wissen

Bereichern Sie Ihre Gespräche mit Live-Webdaten, lokalen Dokumenten und dauerhaftem Gedächtnis.

- **Websuche** — Integration mit Tavily, Zhipu WebSearch, Bocha und mehr.
- **Lokale Wissensbasis (RAG)** — Unterstützt mehrere Wissensbasen. Laden Sie Dokumente für automatisches Parsing, Chunking und Vektorindizierung (sqlite-vec) hoch.
- **Gedächtnissystem** — Unterstützt Multi-Namespace-Gesprächsgedächtnis. Einträge können manuell hinzugefügt oder automatisch von der KI extrahiert werden (demnächst).
- **Kontestverwaltung** — Fügen Sie Dateianhänge, Suchergebnisse, Wissensdatenbankpassagen und Tool-Ausgaben an jede Nachricht an.

::: tip Demnächst
KI-gestützte automatische Gedächtnisextraktion ist in aktiver Entwicklung und wird in einem kommenden Release verfügbar sein.
:::

## Tools und Erweiterungen

Erweitern Sie die Fähigkeiten des Modells mit externen Tools und einer leistungsstarken Befehlsoberfläche.

- **MCP-Protokoll** — Vollständige [Model Context Protocol](https://modelcontextprotocol.io/)-Implementierung mit Unterstützung für **stdio** und **HTTP**-Transporte.
- **Integrierte Tools** — Sofort einsatzbereite integrierte MCP-Tools wie `@axagent/fetch`.
- **Tool-Ausführungspanel** — Ein visuelles Panel zeigt jede Tool-Aufrufanfrage und ihr Rückgabeergebnis an.

## API-Gateway

AxAgent enthält einen integrierten lokalen API-Server, der Ihre Desktop-App in ein leistungsstarkes KI-Gateway verwandelt.

- **Lokales API-Gateway** — Exponieren Sie einen lokalen Server mit nativer Unterstützung für OpenAI-kompatible, Claude- und Gemini-Schnittstellen.
- **API-Schlüsselverwaltung** — Generieren, widerrufen und aktivieren oder deaktivieren Sie Zugriffsschlüssel.
- **Nutzungsanalysen** — Analysieren Sie Anfragevolumen und Token-Nutzung aufgeschlüsselt nach Schlüssel, Anbieter und Datum.
- **SSL/TLS-Unterstützung** — Integrierte selbstsignierte Zertifikatsgenerierung mit Unterstützung für den Import benutzerdefinierter Zertifikate.
- **Anfrage-Logs** — Vollständige Aufzeichnung jeder API-Anfrage und -Antwort durch das Gateway.
- **Konfigurationsvorlagen** — Fertige Integrationsvorlagen für Claude Code, Codex CLI, OpenCode und Gemini CLI.

::: tip Warum ein lokales Gateway?
Das Gateway ermöglicht es Ihnen, AxAgent als einheitliches KI-Backend für alle Ihre Tools zu verwenden. Konfigurieren Sie Ihre CLI-Clients, IDE-Erweiterungen oder benutzerdefinierten Skripte auf das lokale Gateway.
:::

## Daten und Sicherheit

Ihre Daten verlassen nie Ihren Computer. AxAgent ist mit lokal-first Sicherheit auf jeder Ebene konzipiert.

- **AES-256-Verschlüsselung** — API-Schlüssel und andere sensible Daten werden lokal mit AES-256 verschlüsselt.
- **Isolierte Datenverzeichnisse** — Anwendungszustand in `~/.axagent/`. Benutzersichtbare Dateien in `~/Documents/axagent/`.
- **Automatisches Backup** — Planen Sie automatische Backups in lokale Verzeichnisse oder WebDAV-Speicher.
- **Backup-Wiederherstellung** — Ein-Klick-Wiederherstellung aus jedem historischen Backup.
- **Gesprächsexport** — Exportieren Sie Gespräche als PNG, Markdown, Klartext oder strukturiertes JSON.

::: warning Schützen Sie Ihren Hauptschlüssel
Die Datei `~/.axagent/master.key` ist die Wurzel aller Verschlüsselung in AxAgent. Bewahren Sie sie sicher auf und fügen Sie sie in Ihre Backups ein.
:::

## Desktop-Erlebnis

AxAgent ist als native Desktop-Anwendung mit der Politur und Integration aufgebaut, die Sie von einem täglich genutzten Tool erwarten.

- **Theme-Wechsel** — Dunkle und helle Themes, die der Systemeinstellung folgen oder manuell gesetzt werden können.
- **Oberflächensprache** — Vollständige Unterstützung für Deutsch, vereinfachtes Chinesisch und Englisch.
- **System-Tray** — Minimieren Sie in den System-Tray beim Schließen des Fensters. Hintergrunddienste laufen ununterbrochen weiter.
- **Immer im Vordergrund** — Heften Sie das Hauptfenster über alle anderen Fenster.
- **Globale Tastenkürzel** — Anpassbare globale Tastenkürzel zum Aufrufen des Hauptfensters von überall.
- **Auto-Start** — Starten Sie AxAgent optional beim Systemstart.
- **Proxy-Unterstützung** — Konfigurieren Sie HTTP- und SOCKS5-Proxys für Umgebungen mit eingeschränktem Netzwerkzugang.
- **Automatische Updates** — AxAgent prüft beim Start automatisch auf neue Versionen.
