# Anbieter konfigurieren

AxAgent verbindet sich gleichzeitig mit beliebig vielen KI-Anbietern. Jeder Anbieter hat seine eigenen API-Schlüssel, Modellliste und Parameterstandards.

## Unterstützte Anbieter

| Anbieter | Beispielmodelle |
|---------|----------------|
| **OpenAI** | GPT-4o, GPT-4, o3, o4-mini |
| **Anthropic** | Claude 4 Sonnet, Claude 4 Opus, Claude 3.5 Sonnet |
| **Google** | Gemini 2.5 Pro, Gemini 2.5 Flash, Gemini 2.0 |
| **DeepSeek** | DeepSeek V3, DeepSeek R1 |
| **Alibaba Cloud** | Qwen-Serie |
| **Zhipu AI** | GLM-Serie |
| **xAI** | Grok-Serie |
| **OpenAI-kompatible API** | Ollama, vLLM, LiteLLM, Drittanbieter-Relays usw. |

---

## Anbieter hinzufügen

1. Gehen Sie zu **Einstellungen → Anbieter**.
2. Klicken Sie auf die Schaltfläche **+** unten links.
3. Füllen Sie die Anbieterdetails aus:

| Feld | Beschreibung |
|------|-------------|
| **Name** | Anzeigename für die Seitenleiste (z.B. *OpenAI*) |
| **Typ** | Anbietertyp — bestimmt Standard-Base-URL und API-Verhalten |
| **Symbol** | Optionales Symbol zur visuellen Identifikation |
| **API-Schlüssel** | Der geheime Schlüssel aus dem Dashboard Ihres Anbieters |
| **Base URL** | API-Endpoint (für integrierte Typen vorausgefüllt) |
| **API-Pfad** | Anfragepfad — Standard ist `/v1/chat/completions` |

---

## Multi-Key-Rotation

AxAgent unterstützt mehrere API-Schlüssel pro Anbieter. Klicken Sie auf **Schlüssel hinzufügen** im Anbieter-Detailpanel.

---

## Modellverwaltung

Klicken Sie auf **Modelle abrufen** im Anbieter-Detailpanel, um die vollständige Liste der verfügbaren Modelle zu laden. Sie können Modell-IDs auch manuell eingeben.

Jedes Modell kann eigene Standardparameterüberschreibungen haben: Temperatur, Max. Tokens, Top P, Häufigkeitsstrafe, Präsenzstrafe.

---

## Ollama (lokale Modelle)

1. Installieren und starten Sie [Ollama](https://ollama.com/).
2. Erstellen Sie in AxAgent einen neuen Anbieter mit Typ **OpenAI**.
3. Setzen Sie die **Base URL** auf `http://localhost:11434`.
4. Klicken Sie auf **Modelle abrufen**, um lokal heruntergeladene Modelle zu entdecken.

---

## Nächste Schritte

- [MCP-Server](./mcp) — Externe Tools zur Erweiterung der KI-Fähigkeiten verbinden
- [API-Gateway](./gateway) — Ihre Anbieter als lokalen API-Server exponieren
