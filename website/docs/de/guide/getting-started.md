# Schnellstart

## Installation

Laden Sie den neuesten Installer von der [Download-Seite](/de/download) oder [GitHub Releases](https://github.com/polite0803/AxAgent/releases) herunter.

### macOS

| Chip | Datei |
|------|-------|
| Apple Silicon (M1 / M2 / M3 / M4) | `AxAgent_x.x.x_aarch64.dmg` |
| Intel | `AxAgent_x.x.x_x64.dmg` |

1. Öffnen Sie die `.dmg` und ziehen Sie **AxAgent** in den **Programme**-Ordner.
2. Starten Sie AxAgent. Wenn macOS die App blockiert, gehen Sie zu **Systemeinstellungen → Datenschutz & Sicherheit** und klicken Sie auf **Trotzdem öffnen**.

::: warning macOS: „App ist beschädigt" oder „Entwickler kann nicht überprüft werden"
Wenn Sie eine dieser Meldungen sehen, öffnen Sie Terminal und führen Sie aus:

```bash
xattr -c /Applications/AxAgent.app
```

Starten Sie die App dann erneut. Dies entfernt das Quarantäne-Flag, das macOS auf unsignierte Downloads anwendet.
:::

### Windows

| Architektur | Datei |
|-------------|-------|
| x64 (die meisten PCs) | `AxAgent_x.x.x_x64-setup.exe` |
| ARM64 | `AxAgent_x.x.x_arm64-setup.exe` |

Führen Sie den Installer aus und folgen Sie dem Assistenten. Starten Sie AxAgent über das Startmenü oder die Desktop-Verknüpfung.

### Linux

| Format | Architektur | Datei |
|--------|-------------|-------|
| Debian / Ubuntu | x64 | `AxAgent_x.x.x_amd64.deb` |
| Debian / Ubuntu | ARM64 | `AxAgent_x.x.x_arm64.deb` |
| Fedora / openSUSE | x64 | `AxAgent_x.x.x_x86_64.rpm` |
| Fedora / openSUSE | ARM64 | `AxAgent_x.x.x_aarch64.rpm` |
| Beliebige Distribution | x64 | `AxAgent_x.x.x_amd64.AppImage` |
| Beliebige Distribution | ARM64 | `AxAgent_x.x.x_aarch64.AppImage` |

```bash
# Debian / Ubuntu
sudo dpkg -i AxAgent_x.x.x_amd64.deb

# Fedora / openSUSE
sudo rpm -i AxAgent_x.x.x_x86_64.rpm

# AppImage (beliebige Distribution)
chmod +x AxAgent_x.x.x_amd64.AppImage
./AxAgent_x.x.x_amd64.AppImage
```

---

## Ersteinrichtung

### 1. Einstellungen öffnen

Starten Sie AxAgent und klicken Sie auf das **Zahnrad-Symbol** unten in der Seitenleiste, oder drücken Sie <kbd>Cmd/Ctrl</kbd>+<kbd>,</kbd>.

### 2. Anbieter hinzufügen

Navigieren Sie zu **Einstellungen → Anbieter** und klicken Sie auf die Schaltfläche **+**.

1. Geben Sie einen Anzeigenamen ein (z.B. *OpenAI*).
2. Wählen Sie den Anbietertyp (OpenAI, Anthropic, Google Gemini usw.).
3. Fügen Sie Ihren API-Schlüssel ein.
4. Bestätigen Sie die **Base URL** — für integrierte Typen ist der offizielle Endpoint vorausgefüllt.

::: tip
Sie können beliebig viele Anbieter hinzufügen. Jeder Anbieter verwaltet seine eigenen API-Schlüssel und Modelle unabhängig.
:::

### 3. Modelle abrufen

Klicken Sie auf **Modelle abrufen**, um die Liste der verfügbaren Modelle von der API des Anbieters zu laden. Sie können Modell-IDs auch manuell hinzufügen.

### 4. Standardmodell festlegen

Gehen Sie zu **Einstellungen → Standardmodell** und wählen Sie den Anbieter und das Modell, das neue Gespräche standardmäßig verwenden sollen.

---

## Ihr erstes Gespräch

1. Klicken Sie auf **Neuer Chat** in der Seitenleiste (oder drücken Sie <kbd>Cmd/Ctrl</kbd>+<kbd>N</kbd>).
2. Wählen Sie ein Modell aus dem Modell-Selektor oben im Chat.
3. Geben Sie eine Nachricht ein und drücken Sie <kbd>Enter</kbd>.
4. AxAgent streamt die Antwort in Echtzeit. Modelle mit Denkblöcken (z.B. Claude, DeepSeek R1) zeigen den Reasoning-Prozess in einem einklappbaren Bereich.

---

## Tastenkürzel

| Kürzel | Aktion |
|--------|--------|
| <kbd>Cmd/Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>A</kbd> | Aktuelles Fenster ein-/ausblenden |
| <kbd>Cmd/Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Alt</kbd>+<kbd>A</kbd> | Alle Fenster ein-/ausblenden |
| <kbd>Cmd/Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>W</kbd> | Fenster schließen |
| <kbd>Cmd/Ctrl</kbd>+<kbd>N</kbd> | Neues Gespräch |
| <kbd>Cmd/Ctrl</kbd>+<kbd>,</kbd> | Einstellungen öffnen |
| <kbd>Cmd/Ctrl</kbd>+<kbd>K</kbd> | Befehlspalette |
| <kbd>Cmd/Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>G</kbd> | API-Gateway umschalten |

---

## Daten und Backup

### Datenverzeichnisse

| Pfad | Inhalt |
|------|--------|
| `~/.axagent/` | Anwendungszustand — Datenbank, Verschlüsselungsschlüssel, Vektor-DB, SSL-Zertifikate |
| `~/Documents/axagent/` | Benutzerdateien — Bilder, Dokumente, Backups |

---

## Nächste Schritte

- [Anbieter konfigurieren](./providers) — KI-Anbieter hinzufügen und verwalten
- [MCP-Server](./mcp) — Externe Tools zur Erweiterung der KI-Fähigkeiten verbinden
- [API-Gateway](./gateway) — Ihre Anbieter als lokalen API-Server exponieren
