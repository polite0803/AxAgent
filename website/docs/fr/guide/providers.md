# Configurer les fournisseurs

AxAgent se connecte simultanément à n'importe quel nombre de fournisseurs IA. Chaque fournisseur a ses propres clés API, liste de modèles et paramètres par défaut.

## Fournisseurs supportés

| Fournisseur | Modèles exemple |
|------------|----------------|
| **OpenAI** | GPT-4o, GPT-4, o3, o4-mini |
| **Anthropic** | Claude 4 Sonnet, Claude 4 Opus, Claude 3.5 Sonnet |
| **Google** | Gemini 2.5 Pro, Gemini 2.5 Flash, Gemini 2.0 |
| **DeepSeek** | DeepSeek V3, DeepSeek R1 |
| **Alibaba Cloud** | Série Qwen |
| **Zhipu AI** | Série GLM |
| **xAI** | Série Grok |
| **API compatible OpenAI** | Ollama, vLLM, LiteLLM, relais tiers, etc. |

---

## Ajouter un fournisseur

1. Allez dans **Paramètres → Fournisseurs**.
2. Cliquez sur le bouton **+** en bas à gauche.
3. Remplissez les détails du fournisseur :

| Champ | Description |
|-------|-------------|
| **Nom** | Nom d'affichage pour la barre latérale (ex. *OpenAI*) |
| **Type** | Type de fournisseur — détermine l'URL de base par défaut |
| **Icône** | Icône optionnelle pour l'identification visuelle |
| **Clé API** | La clé secrète du tableau de bord de votre fournisseur |
| **URL de base** | Endpoint API (pré-rempli pour les types intégrés) |
| **Chemin API** | Chemin de requête — par défaut `/v1/chat/completions` |

---

## Rotation de clés multiples

AxAgent supporte plusieurs clés API par fournisseur pour la distribution de charge et l'évitement des limites de débit. Cliquez sur **Ajouter une clé** dans le panneau de détails du fournisseur.

---

## Gestion des modèles

Cliquez sur **Récupérer les modèles** pour obtenir la liste complète des modèles disponibles depuis l'API du fournisseur. Vous pouvez également ajouter des IDs de modèles manuellement.

Chaque modèle peut avoir ses propres paramètres par défaut : température, tokens maximum, Top P, pénalité de fréquence, pénalité de présence.

---

## Endpoints personnalisés et locaux

### Ollama (modèles locaux)

1. Installez et démarrez [Ollama](https://ollama.com/).
2. Dans AxAgent, créez un nouveau fournisseur avec le type **OpenAI**.
3. Définissez l'**URL de base** à `http://localhost:11434`.
4. Cliquez sur **Récupérer les modèles** pour découvrir les modèles téléchargés localement.

---

## Prochaines étapes

- [Serveurs MCP](./mcp) — connecter des outils externes pour étendre les capacités IA
- [Passerelle API](./gateway) — exposer vos fournisseurs comme serveur API local
