# Serveurs MCP

## Qu'est-ce que MCP ?

Le [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) est un standard ouvert qui permet aux modèles IA d'interagir avec des outils et des sources de données externes. AxAgent agit comme un client MCP — vous ajoutez des serveurs MCP, et l'IA peut appeler les outils qu'ils exposent pendant une conversation.

---

## Protocoles de transport

AxAgent supporte trois protocoles de transport pour communiquer avec les serveurs MCP :

| Protocole | Connexion | Cas d'utilisation | Configuration |
|-----------|-----------|------------------|---------------|
| **Stdio** | Processus local | Outils installés sur votre machine, lancés via `npx`, `uvx`, `python`, etc. | `command` + `args` + `env` optionnel |
| **SSE** | Serveur distant | Endpoint Server-Sent Events hébergé sur une machine distante | `url` |
| **StreamableHTTP** | Serveur distant | Endpoint HTTP streaming, alternative plus récente à SSE | `url` |

---

## Ajouter des serveurs MCP

### Création par formulaire

1. Allez dans **Paramètres → Serveurs MCP**.
2. Cliquez sur **Ajouter un serveur MCP**.
3. Entrez un nom et sélectionnez le protocole de transport.
4. Remplissez les champs pour votre protocole choisi.
5. Cliquez sur **Enregistrer**.

### Import JSON

Cliquez sur **Import JSON** et collez un objet de configuration. AxAgent accepte le format JSON MCP standard :

```json
{
  "mcpServers": {
    "fetch": {
      "command": "uvx",
      "args": ["mcp-server-fetch"]
    }
  }
}
```

---

## Outils intégrés

| Outil | Description |
|-------|-------------|
| **@axagent/fetch** | Récupérer des pages web et des ressources HTTP |
| **@axagent/search-file** | Rechercher des fichiers sur votre système de fichiers local |

---

## Prochaines étapes

- [Passerelle API](./gateway) — exposer vos fournisseurs comme serveur API local
- [Démarrage rapide](./getting-started) — retourner au guide de démarrage rapide
