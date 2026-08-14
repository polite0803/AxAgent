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

**AxAgent** est un client de bureau AI multiplateforme basé sur Tauri 2 (Windows / macOS / Linux / Android / iOS), conçu comme un poste de travail AI pour le développement quotidien, la recherche, la gestion des connaissances et l'automatisation. Il intègre un moteur d'agent ReAct, un routage cognitif (routage hiérarchique à trois niveaux + routage augmenté par récupération RAR), un orchestrateur visuel de workflows, des bases de connaissances RAG locales, des extensions de protocole MCP, une passerelle multi-modèles unifiée, l'automatisation de navigateur et le contrôle d'ordinateur — permettant à l'IA de passer de la « conversation » à l'« exécution ».

> **Langues**: [简体中文](./README.md) | [English](./README-EN.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

---

## Positionnement du projet

AxAgent résout trois problèmes fondamentaux :

1. **Accès multi-modèles unifié et routage intelligent** — Une seule interface pour utiliser simultanément OpenAI, Anthropic Claude, Google Gemini, DeepSeek, Qwen, GLM, Kimi, Wenxin, les modèles locaux Ollama et toute API compatible OpenAI, avec rotation automatique des quotas multi-clés, routage intelligent par type de tâche et comparaison en streaming
2. **Boucle fermée de la conversation à l'exécution** — 163+ outils intégrés + workflows visuels + extensions MCP + contrôle du navigateur/ordinateur, l'IA peut manipuler des fichiers, exécuter du code, gérer Git et planifier des tâches
3. **Souveraineté des données locale d'abord** — Les conversations, bases de connaissances, mémoires et configurations sont stockées dans une base SQLite locale, les clés API sont chiffrées en AES-256-GCM, et les fonctionnalités principales fonctionnent sans services cloud tiers

---

## Capacités principales

### Système de routage cognitif (Cognitive Router)

AxAgent utilise `cognitive_query` comme point d'entrée unifié pour toutes les conversations, mappant l'intention de l'utilisateur vers des capacités concrètes via un **routage hiérarchique à trois niveaux** :

- **Routage de domaine L1** (`domain_router`): règles + repli LLM, identifie 9 grands domaines métier (analyse de données / création de contenu / communication / exploitation / médias IA / finance / automatisation / général, etc.)
- **Routage de cluster L2** (`cluster_router`): localise les clusters de capacités au sein du domaine (27 clusters couvrant 8 grands domaines métier)
- **Routage de capacité L3**: **Routage augmenté par récupération (RAR)** — rappelle les Top-K workflows similaires depuis le vecteur de capacités pour les injecter dans le Prompt, combiné à la recherche de chemin dans le DAG du workflow, et génère l'adresse de chemin (ex. `/finance/stock_analysis/tech`) et le mode d'exécution
- **Modes d'exécution**: `Ask / Plan / Act / Workflow / Direct / Delegate / ParameterExtract / Clarify`, sélectionnés automatiquement selon la confiance
- **Système de capacités**: registre unifié (`CapabilityRegistry`) + index vectoriel (`CapabilityIndexer`) + recherche hybride (`CapabilityRetriever`, vecteur + BM25 + correspondance stricte par étiquettes + exclusion des échantillons négatifs)
- **Isolation des capacités système**: l'orchestrateur cognitif et les workflows métier sont physiquement isolés, les capacités système portent le marqueur de visibilité `SYSTEM_ONLY`, et la couche de routage intègre un disjoncteur auto-référentiel pour empêcher les paradoxes d'auto-référence
- **Routage à trois niveaux implémenté en DAG de workflow**: 4 modèles de workflow de routage prédéfinis (orchestrateur principal ~20 nœuds + sous-routages L1/L2/L3), exécutés par le moteur `rt-workflow`

### Moteur multi-modèles

- **13 adaptateurs de fournisseurs**: OpenAI (Chat Completions + Responses + Realtime), Anthropic Claude, Google Gemini, DeepSeek, Qwen, GLM, Kimi, Wenxin Yiyan, Ollama, Llama.cpp (modèles locaux GGUF), OpenClaw, Hermes, ainsi que toutes les API compatibles OpenAI
- **Rotation multi-clés**: plusieurs clés API pour un même fournisseur, rotation automatique selon les quotas, bascule automatique en cas de limitation d'une clé
- **Routage intelligent**: sélection automatique du meilleur modèle selon le type de tâche (revue de code / résumé / traduction / général), avec règles personnalisables
- **Surveillance de la santé des fournisseurs**: suivi en temps réel du taux de réussite, de la latence et de l'état de disponibilité, avec dégradation automatique par niveaux
- **Génération d'images IA**: DALL-E 3 et Flux avec préréglages multi-tailles
- **Voix en temps réel**: conversation vocale WebSocket basée sur l'API OpenAI Realtime, avec interruption et transcription en streaming

### Système d'agents (moteur ReAct)

- **Planificateur hiérarchique** (`hierarchical_planner`): décompose les tâches complexes en plans structurés Phase → Task, compilés en exécution topologique DAG
- **Recherche approfondie** (`deep_research`): orchestration de recherche multi-sources, avec plan de recherche, exécution de recherche, synthèse de contenu et suivi des citations
- **Vérification des faits** (`fact_checker`): vérification factuelle pilotée par l'IA, avec classificateur de sources et évaluation de crédibilité
- **Arbre de pensées** (`tree_of_thoughts`): exploration de raisonnement multi-chemins, évaluation des branches et retour en arrière
- **Réflecteur** (`reflector`): auto-évaluation et suggestions d'amélioration après exécution de la tâche
- **Auto-vérification** (`self_verifier`): validation automatique des résultats de raisonnement, avec détection de boucles
- **Récupération d'erreurs** (`error_recovery_engine`): classification des types d'erreurs → sélection de stratégie de récupération → nouvelle tentative automatique ou ajustement du plan, avec backoff exponentiel
- **Tests A/B** (`ab_testing`): évaluation comparative de différentes stratégies de raisonnement
- **Système d'évaluation** (`evaluator`): framework de tests de référence intégré
- **Fine-tuning LoRA** (`fine_tune`): pipeline d'entraînement intégré, avec gestion des adaptateurs LoRA
- **Optimiseur RL** (`rl_optimizer`): apprentissage par renforcement des stratégies basé sur les retours d'expérience

**Collaboration multi-agents** :

- Architecture de coordination maître-esclave, exécution parallèle des sous-agents, ordonnancement sensible aux dépendances
- Tableau noir partagé pour l'échange d'informations entre agents
- Mode débat contradictoire (tours Pro/Con et notation de la force des arguments)
- Mode cluster Swarm, cluster d'agents multi-processus
- Mode proactif : les agents peuvent initier activement des suggestions et des actions

**Contrôle d'ordinateur**: clics de souris, saisie clavier et défilement d'écran pilotés par l'IA, avec trois niveaux de permissions (par défaut / accepter les modifications / accès complet) et isolation des chemins en sandbox

**Automatisation de navigateur**: contrôle du navigateur via le protocole CDP, avec navigation, capture d'écran, clic, remplissage de formulaires et extraction de texte

### Système de compétences

- **Marché de compétences**: parcourir et installer des compétences communautaires
- **Création assistée par IA**: création automatique de la structure de compétences à partir d'une proposition en langage naturel (`skill:create`)
- **Évolution des compétences** (`evolution_engine`): analyse et amélioration automatiques des compétences basées sur les retours d'exécution
- **Correspondance sémantique**: recommandation automatique de compétences pertinentes selon le contexte sémantique de la conversation
- **Décomposition des compétences** (`skill_decomposition`): décomposition automatique des tâches complexes en combinaisons de compétences atomiques
- **Génération d'outils**: génération et enregistrement de nouveaux outils par l'IA
- **Exécution en sandbox**: les compétences s'exécutent en toute sécurité dans un sandbox isolé

### Workflows visuels

Éditeur de workflows DAG par glisser-déposer basé sur ReactFlow 12 :

- **32 types de nœuds**: déclencheur, agent, appel LLM, branche conditionnelle, fourche parallèle, boucle, fusion, délai, appel d'outil, exécution de code, sous-workflow, récupération vectorielle, analyse de document, validation, fin, requête HTTP, Switch, requête de base de données, notification, approbation, opération sur fichiers, transformation de données, envoi Webhook, journal, classifieur LLM, agrégateur, e-mail, débat, Swarm, multi-agents, stockage, règle métier
- **Exécution par tri topologique de Kahn**: détection automatique des dépendances circulaires, ordonnancement parallèle en pipeline
- **Modèles intégrés**: revue de code, correction de bugs, génération de documentation, tests, refactorisation, exploration, analyse de performance, revue de sécurité, développement de fonctionnalités
- **Sérialisation YAML**: import/export des définitions de workflow
- **Gestion de versions**: contrôle de version des modèles
- **Conception assistée par IA**: conception de workflow assistée par IA, recommandation de nœuds et diagnostic

### Gestion des connaissances

- **RAG multi-bases de connaissances**: téléversement de documents → analyse automatique (PDF/DOCX/XLSX/PPTX/TXT) → découpage en blocs → indexation vectorielle
- **Recherche hybride**: similarité vectorielle (sqlite-vec + embeddings locaux candle) + recherche plein texte BM25 (FTS5), classement hybride
- **Self-RAG**: réflexion et validation automatiques des résultats de recherche
- **Réordonnancement**: réordonnancement des résultats par Cross-encoder
- **Graphe de connaissances**: extraction d'entités → construction de relations → graphe visualisé
- **Surveillance de fichiers**: surveillance en temps réel des modifications de fichiers basée sur `notify`, indexation incrémentale automatique
- **LLM Wiki**: compilateur et validateur de Wiki assistés par IA

### Système de mémoire

- **Mémoire multi-espaces de noms**: isolation par projet/thème, avec saisie manuelle et extraction automatique par l'IA
- **Intégration de persistance**: mémoire en boucle fermée Honcho et Mem0
- **Profil utilisateur**: apprentissage automatique du style de code, des préférences de pile technique et du style de communication
- **Transfert de style**: extraction des caractéristiques de style de code → application au code généré par l'IA
- **Intégration onirique**: intégration automatique en arrière-plan des fragments de mémoire et des schémas de comportement pour générer des connaissances structurées
- **Mémoire de projet**: persistance du contexte par dimension projet

### Passerelle API

Passerelle HTTP + WebSocket intégrée basée sur `axum` :

- **Points d'extrémité compatibles**: OpenAI `/v1/chat/completions`, API Claude Messages, API Gemini, ainsi que OpenAI Responses et Realtime WebSocket
- **Gestion des clés**: génération, révocation, activation/désactivation des clés d'accès, avec prise en charge de l'expiration
- **Suivi de l'utilisation**: statistiques du volume de requêtes et de la consommation de tokens par clé/fournisseur/date, export de métriques Prometheus
- **Limitation de débit**: algorithme du seau à jetons basé sur `governor`
- **SSL/TLS**: certificat auto-signé intégré (`rcgen`), prise en charge de certificats personnalisés
- **Liens externes**: intégration en un clic d'outils externes comme Claude CLI, OpenCode, avec synchronisation automatique des clés API
- **Tickets en temps réel**: tickets d'authentification temporaires basés sur HMAC pour la transmission sécurisée des connexions WebSocket
- **Mode serveur**: binaire optionnel `axagent-server` qui expose les capacités de l'application de bureau en tant que service

### Intégration des plateformes de messagerie

Passerelle multi-plateformes via `rt-messaging`, prenant en charge la réception de messages, l'analyse de commandes et la réponse automatique par IA sur **DingTalk, Feishu, QQ, Slack, WeChat, WhatsApp, Telegram, Discord**.

### Système d'outils

**163+ outils intégrés**, tous enregistrés via le trait `Tool`, couvrant 15 grandes catégories :

| Catégorie             | Exemples d'outils                                                                                                                                                         |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Opérations fichiers   | `file_read`, `file_write`, `file_edit`, `glob`, `grep`, répertoire/suppression/déplacement, etc. — 11 au total                                                            |
| Shell/Web             | `bash`, `web_fetch`, `web_search`                                                                                                                                         |
| Réseau                | `http_request`, `ping`, `dns_lookup`, `json_api`, `rss_reader`, `graphql`, `websocket`                                                                                    |
| Navigateur            | `browser_navigate`, `browser_click`, `browser_fill`, `browser_screenshot`, etc. — 10 au total (CDP)                                                                       |
| Contrôle d'ordinateur | `computer_use` (souris/clavier/capture d'écran)                                                                                                                           |
| Git                   | `git_status`, `git_diff`, `git_commit`, `git_log`, `git_branch`, `git_review`                                                                                             |
| Base de connaissances | `list_knowledge_bases`, `search_knowledge`, `add_knowledge_document`, etc. — 6 au total                                                                                   |
| Gestion des tâches    | `todo_write`, `task_*` (6), `cron_*` (3), liés à `plan`                                                                                                                   |
| Notifications         | `push_notification`, `send_message`, outils de collaboration d'équipe                                                                                                     |
| Base de données       | `database_query`, `database_list_tables`, `database_migration_status`                                                                                                     |
| Stockage              | `get_storage_info`, `upload_storage_file`, `download_storage_file`, etc. — 5 au total                                                                                     |
| Export/Format         | `export_word`, `export_pdf`, `export_xlsx`, `export_pptx`, `render_markdown`, etc. — 9 au total                                                                           |
| OCR                   | `ocr_image`, `ocr_detect_langs`                                                                                                                                           |
| Obsidian              | `obsidian_search`, `obsidian_read`, `obsidian_backlinks`, etc. — 9 au total                                                                                               |
| Autres                | `agent`, `delegate_task`, `skills_*`, `lsp`, `repl`, `monitor`, `workspace_*`, `session_search`, `generate_image`, `sequential_thinking`, CI/CD, DevOps, RPC, tests, etc. |

### Protocole MCP

Implémentation complète du MCP (Model Context Protocol) basée sur `rmcp` :

- **Couche de transport**: processus enfant stdio + Streamable HTTP + SSE
- **Authentification OAuth**: prise en charge du flux d'autorisation OAuth des serveurs MCP
- **Découverte d'outils**: découverte et enregistrement automatiques des outils exposés par les serveurs MCP
- **Gestionnaire MCP**: gestion du cycle de vie des serveurs, contrôles de santé et reconnexion automatique

### Système de plugins

Architecture de plugins à trois niveaux compatible OpenClaw (intégré / groupé / externe) :

- Installation via packages npm, avec UI de marché intégrée pour la recherche et l'installation
- Définition du manifest du plugin, déclaration des permissions, exécution isolée en sandbox
- Enregistrement d'outils personnalisés, fournisseurs d'agents, interception par Hooks
- Installateur de compétences : installation de compétences depuis les packages de plugins vers le système de compétences

### Moteur d'UI dynamique

- **Piloté par schéma**: construction déclarative d'interfaces via JSON Schema, sans écrire de code
- **31 composants intégrés**: conteneurs (7) / affichage de données (6) / formulaires (9) / médias (4) / autres (5)
- **Liaison de données**: liaison déclarative des sources de données et rendu conditionnel
- **NL2UI**: génération directe d'interfaces UI dynamiques à partir du langage naturel

### SDK client ACP

- **ACP (Agent Client Protocol)**: SDK bilingue (TypeScript + Python), zéro dépendance tierce
- Gestion de session, envoi de prompts, enregistrement des appels d'outils, flux d'événements WebSocket
- Communication avec le service AxAgent via les points d'extrémité `/acp/v1/*`

### Sécurité

- **Chiffrement AES-256-GCM**: stockage chiffré local des clés API et des configurations sensibles (crate `crypto`)
- **Protection contre l'injection de prompts**: pipeline de défense à quatre niveaux (`prompt-guard`) — détection de motifs → échappement des séparateurs → wrapper XML → étiquettes de confiance, intégré à toute la chaîne des sessions, de la construction des prompts, de Git et du RAG
- **Protection SSRF**: vérification de sécurité des URL, blocage des requêtes vers les adresses du réseau interne
- **Filtrage de contenu**: filtrage de sécurité du contenu multi-types
- **Limitation de débit**: limitation par seau à jetons des appels d'outils et des requêtes API
- **Disjoncteur**: coupure automatique en cas d'échecs consécutifs
- **Contrôle d'accès**: contrôle des permissions d'accès aux outils basé sur des politiques
- **Isolation en sandbox**: isolation des environnements d'exécution des agents et des compétences

### Outils de développement

- **Traçage distribué** (`telemetry`): intégration OpenTelemetry, visualisation Span/Trace
- **Journaux structurés**: tracing-subscriber + horodatage chrono
- **Débogage par relecture**: enregistrement (`trajectory_recorder`) et relecture des trajectoires d'exécution des agents
- **Panneau DevTools**: visualiseur de chronologie Trace Explorer, Benchmark Runner, Tool Recommender
- **Tests de référence**: benchmarks Criterion (tool_exec / llm_call / search)
- **Vérification CI**: `npm run ci:check` intègre la vérification des types, le lint et la validation du formatage

### Expérience bureau et mobile

- **Mise en page réactive**: points de rupture CSS adaptatifs pour bureau/tablette/mobile (3 niveaux de mise en page appareil : `desktop` / `tablet` / `mobile`)
- **11 langues**: chinois simplifié, chinois traditionnel, anglais, japonais, coréen, français, allemand, espagnol, russe, hindi, arabe
- **Moteur de thème** (`rt-theme`): thèmes sombre/clair + plusieurs préréglages, personnalisation approfondie d'Ant Design 6
- **Éditeur Monaco**: coloration syntaxique, aperçu des différences, prise en charge multilingue
- **Terminal xterm.js**: WebLinks, Unicode 11, recherche
- **Défilement virtuel**: @tanstack/react-virtual + react-virtuoso
- **Rendu de graphiques**: D2 + Mermaid + Recharts + Sigma (graphes)
- **Palette de commandes**: panneau de commandes global Ctrl+K
- **Tray système + raccourcis globaux + démarrage automatique**: exécution en arrière-plan sans distraction
- **Mise à jour automatique**: détection de versions GitHub Releases à intervalle configurable
- **Prise en charge des proxys**: configuration des proxys HTTP / SOCKS5
- **Espace de travail cloud**: synchronisation du stockage S3 et WebDAV, détection des conflits et synchronisation bidirectionnelle

### Mobile

- Android APK/AAB (arm64-v8a, armeabi-v7a, x86_64)
- iOS IPA (arm64)
- Adaptations spécifiques au mobile : adaptation des zones de sécurité, navigation en bas d'écran, navigation Drawer

---

## Architecture technique

### Pile technologique

| Couche               | Technologie                              | Version |
| -------------------- | ---------------------------------------- | ------- |
| Framework bureau     | Tauri                                    | 2.11    |
| Framework front-end  | React                                    | 19      |
| Système de types     | TypeScript                               | 7       |
| Bibliothèque UI      | Ant Design                               | 6       |
| Framework CSS        | TailwindCSS                              | 4       |
| Gestion d'état       | Zustand                                  | 5       |
| Routage              | React Router                             | 7       |
| Éditeur de code      | Monaco Editor                            | 0.55    |
| Terminal             | xterm.js                                 | 6       |
| Éditeur de workflows | ReactFlow                                | 12      |
| Graphiques           | D2 + Mermaid + Recharts + Sigma          |         |
| Animation            | Framer Motion                            | 12      |
| Défilement virtuel   | @tanstack/react-virtual + react-virtuoso |         |
| Glisser-déposer      | @dnd-kit                                 | 6       |
| Rendu Markdown       | markstream-react + stream-markdown       |         |
| Internationalisation | i18next + react-i18next                  |         |
| Outil de build       | Vite                                     | 8       |
| Tests                | Vitest + Playwright                      |         |
| Formatage            | dprint (TS/JSON/Markdown/TOML) + rustfmt |         |
| Lint                 | ESLint + Oxlint + Clippy                 |         |

### Architecture back-end : modèle d'injection de dépendances Harness

Adopte une architecture workspace Rust comprenant **37 membres** (crate principal + 35 crates de bibliothèque + schema-gen), conforme à l'**architecture d'injection de dépendances Harness** :

> Tous les crates sont découplés via les interfaces de trait définies par axagent-harness ; à l'exécution, axagent-runtime assemble et injecte les dépendances.
> Direction des dépendances : `implémentation concrète → harness ← appelant`

**harness** est la pierre angulaire de l'architecture — zéro logique métier, zéro implémentation concrète, uniquement des définitions de trait, des DTO de données purs, des constantes et des types d'erreur unifiés. Il est dépendu par tous les autres crates et ne dépend lui-même d'aucun crate axagent-* (200+ définitions de trait couvrant Agent/Provider/Tool/RAG/stockage/MCP/plugins/sécurité/observabilité/mémoire/apprentissage/navigateur/messagerie/routage cognitif, etc.).

```
src-tauri/crates/
├── harness/          # Pierre angulaire de l'architecture — interfaces de trait, DTO, types d'erreur, contrat DI
├── entities/         # Modèles d'entités SeaORM
├── dao/              # Couche d'accès aux données (CRUD)
├── migration/        # Migrations de base de données
├── crypto/           # Chiffrement/déchiffrement AES-256-GCM et gestion des clés
├── credential/       # Stockage sécurisé des identifiants
├── storage/          # Abstraction de stockage de fichiers (local/S3/WebDAV), lecture/écriture ZIP
├── cache/            # Couche de cache en mémoire
├── disk-cache/       # Cache au niveau des fichiers disque
├── search/           # Moteur de recherche (FTS5 + sqlite-vec + embeddings locaux candle)
├── document-parser/  # Extraction de texte de documents (PDF/DOCX/XLSX/PPTX)
├── kit/              # Boîte à outils générique (chemins/encodage/hachage/dates)
├── runtime-core/     # Types communs du runtime, constantes de configuration
├── runtime/          # Orchestration des services du runtime — conteneur DI assemblant tous les crates
├── rt-workflow/      # Moteur de workflow — orchestration DAG, exécuteurs de nœuds, sérialisation YAML
├── rt-messaging/     # Passerelle de plateformes de messagerie — DingTalk/Feishu/QQ/Slack/WeChat/WhatsApp/Telegram/Discord
├── rt-webhook/       # Serveur Webhook générique
├── rt-dashboard/     # Framework de plugins de tableau de bord
├── rt-theme/         # Moteur de thème
├── agent/            # Noyau d'agents IA — 80+ modules
│                     #   Moteur ReAct/planification hiérarchique/recherche approfondie/vérification des faits/
│                     #   arbre de pensées/réflexion/auto-vérification/récupération d'erreurs/optimisation RL/
│                     #   fine-tuning LoRA/évaluation/recommandation d'outils/tests A-B/coordinateur/
│                     #   tableau noir/pipeline visuel/recherche web/recherche académique/compilation Wiki, etc.
├── orchestrator/     # Orchestration d'agents — ordonnancement multi-agents, décomposition DAG, exécution de sous-graphes dynamiques
├── providers/        # Adaptateurs de fournisseurs de modèles (13)
├── tools/            # Système d'outils — trait Tool/registre/orchestration/streaming/sandbox/163+ outils intégrés
├── gateway/          # Passerelle API — serveur axum HTTP/WS, OAuth, limitation de débit, Prometheus
├── mcp/              # Protocole MCP — stdio + Streamable HTTP + SSE, basé sur rmcp
├── trajectory/       # Système d'apprentissage — mémoire/évolution des compétences/profil utilisateur/intégration onirique
├── plugins/          # Système de plugins — compatible OpenClaw, installation npm, marché
├── telemetry/        # Observabilité — OpenTelemetry, journaux structurés, métriques du runtime
├── prompt-guard/     # Protection contre l'injection de prompts — pipeline de détection multi-niveaux L1-L4
├── npm/              # Client de registre npm
├── crdt/             # Structures de données d'édition collaborative
├── device/           # Gestion des appareils
├── axagent-mobile/   # Couche d'adaptation mobile
├── agent-macro/      # Macros d'agents
├── agent-command-types/ # Types de commandes d'agents
└── schema-gen/       # Outil de génération de schéma de base de données
```

### Architecture front-end

```
src/
├── pages/            # Pages (24)
│   ├── ChatPage           # Interface de conversation principale — barre latérale/flux de messages/panneau Agent/onglets multiples
│   ├── DashboardPage      # Tableau de bord de données — statistiques d'utilisation/répartition des modèles/graphiques de tendance
│   ├── WorkflowPage       # Éditeur de workflows — visualisation DAG ReactFlow
│   ├── KnowledgeHubPage   # Gestion des bases de connaissances — téléversement/indexation/recherche de documents
│   ├── MemoryPage         # Gestion de la mémoire
│   ├── SkillsPage         # Marché de compétences
│   ├── SettingsPage       # Panneau de paramètres — 40+ éléments de configuration
│   ├── TerminalPage       # Terminal intégré — xterm.js
│   ├── FilesPage          # Gestion des fichiers
│   ├── GatewayLinkPage    # Passerelle API et gestion des liens externes
│   ├── QuickBarPage       # Barre rapide (fenêtre indépendante)
│   ├── DynamicUIManagerPage / DynamicPageViewer  # Moteur d'UI dynamique
│   ├── WikiGraphPage / WikiEditPage / IngestPage # LLM Wiki
│   ├── LearningGraphPage  # Graphe d'apprentissage
│   ├── FineTunePage       # Fine-tuning LoRA
│   ├── PersonaPage        # Gestion des rôles
│   ├── WorkflowMarketplace # Marché de workflows
│   ├── DevTools/          # TraceExplorer / BenchmarkRunner / ToolRecommender
│   └── Workflow/          # WorkflowListPage
│
├── components/       # 33 modules, 500+ composants
│   ├── chat/         # Conversation (flux de messages/saisie/ChatView/TabBar/RightPanel/pièces jointes/rendu des appels d'outils)
│   ├── layout/       # Mise en page — AppInitializer / Sidebar / ContentArea / TitleBar /
│   │                 #   CommandPalette / GlobalCopyMenu / GlobalErrorBoundary /
│   │                 #   GlobalStatusBar / ErrorNotificationToast / AppHeader, etc.
│   ├── agent/        # Panneau Agent/entrées/panneau miniature
│   ├── workflow/     # Éditeur de workflows (nœuds/connexions/panneaux/modèles/aide IA)
│   ├── settings/     # Panneau de paramètres (40+ sous-composants)
│   ├── skill/        # Éditeur/rendu de compétences/panneau flottant
│   ├── dynamicUI/    # Composants d'UI dynamique (31 composants intégrés)
│   ├── gateway/      # Gestion de la passerelle API
│   ├── files/        # Gestion des fichiers
│   ├── terminal/     # Composants de terminal
│   ├── search/       # Interface de recherche
│   ├── benchmark/    # Panneau de tests de référence
│   ├── decomposition/# Décomposition de compétences et génération d'outils
│   ├── devtools/     # Chronologie Trace/Span + panneau d'entraînement RL
│   ├── approval/     # Interface des flux d'approbation
│   ├── recommendation/ # Recommandation d'outils/modèles
│   ├── onboarding/   # WelcomeWizard / InteractiveTutorial
│   ├── help/         # Panneau d'aide
│   ├── notification/ # Composants de notification
│   ├── proactive/    # Suggestions proactives
│   ├── llm-wiki/     # Composants LLM Wiki
│   ├── wiki/         # Composants Wiki
│   ├── fine-tune/    # Interface de fine-tuning
│   ├── trace/        # Composants Trace
│   ├── style/        # Styles/thèmes
│   ├── shared/       # Composants partagés (ErrorBoundary / PageContextProvider)
│   └── common/       # Composants génériques (Icon, etc.)
│
├── stores/           # Gestion d'état Zustand (82 stores)
│   ├── domain/       # 9 stores métier principaux (conversation/flux/compression/préférences/multi-modèles, etc.)
│   ├── feature/      # 61 stores de modules fonctionnels (agents/workflows/bases de connaissances/compétences/passerelle/mémoire/terminal, etc.)
│   ├── shared/       # 8 stores partagés entre composants (UI/onglets/espace de travail/état back-end, etc.)
│   └── devtools/     # 4 stores d'outils de développement
│
├── hooks/            # Hooks React (raccourcis/palette de commandes/réactif/barres de défilement/thème/Avatar, etc.)
├── lib/              # Bibliothèque de fonctions utilitaires (invoke/pageRegistry/shortcuts/skillLifecycle/
│                     #   chartGenerator/codeExecutor/tokenEstimator/workflowLayout, etc. — 45+ modules)
├── types/            # Définitions de types TypeScript
├── theme/            # Moteur de thème Shadcn
├── i18n/             # Fichiers de traduction en 11 langues (zh-CN/zh-TW/en-US/ja/ko/fr/de/es/ru/hi/ar)
├── constants/        # Constantes et commutateurs de fonctionnalités
└── sdk/              # SDK client ACP (TypeScript + Python)
```

### Commutateurs de fonctionnalités

Le projet gère le déploiement progressif des fonctionnalités via `featureFlags.ts` :

| Commutateur         | Statut | Description                                            |
| ------------------- | ------ | ------------------------------------------------------ |
| `AGENT_IN_THE_LOOP` | ✅     | Panneau Agent global + injection du contexte de page   |
| `DYNAMIC_UI`        | ✅     | Moteur de construction d'UI dynamique                  |
| `SELF_EVOLUTION_UI` | ❌     | Surface de contrôle front-end de l'auto-évolution      |
| `NL_EXTENSION`      | ❌     | Extension métier dynamique pilotée par langage naturel |

### Plugins Tauri

| Plugin              | Utilisation                                 |
| ------------------- | ------------------------------------------- |
| `autostart`         | Démarrage automatique                       |
| `clipboard-manager` | Lecture/écriture du presse-papiers          |
| `dialog`            | Boîtes de dialogue de sélection de fichiers |
| `fs`                | Accès au système de fichiers                |
| `global-shortcut`   | Enregistrement des raccourcis globaux       |
| `notification`      | Notifications système                       |
| `opener`            | Ouverture de liens externes/fichiers        |
| `process`           | Gestion des processus                       |
| `updater`           | Mise à jour automatique                     |

---

## Répertoires de données

```
~/.axagent/                    # Configuration de l'application
├── axagent.db                 # Base de données principale SQLite (SeaORM)
├── master.key                 # Clé principale AES-256
├── vector_db/                 # Index vectoriel sqlite-vec
└── ssl/                       # Certificats SSL auto-signés

~/Documents/axagent/          # Fichiers utilisateur
├── images/                   # Pièces jointes d'images
├── files/                    # Pièces jointes de fichiers
└── backups/                  # Sauvegardes automatiques
```

---

## Démarrage rapide

### Prérequis

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+ (édition 2024)
- [npm](https://www.npmjs.com/) 10+
- Windows : [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (MSVC + Windows SDK)
- macOS : Xcode Command Line Tools
- Linux : `build-essential` + `libwebkit2gtk-4.1-dev` + `libssl-dev`

### Développement

```bash
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent
npm install
npm run tauri dev      # Mode développement (Vite HMR front-end + fenêtre Tauri)
```

### Construction

```bash
npm run tauri build    # Construction de production bureau

npm run tauri:android:build   # Construction Android
npm run tauri:ios:build       # Construction iOS
```

Les artefacts de construction bureau se trouvent dans `src-tauri/target/release/`.

### Tests

```bash
npm run test           # Tests unitaires front-end (watch Vitest)
npm run test:run       # Tests unitaires front-end (exécution unique)
npm run test:e2e       # Tests E2E (Playwright)

# Tests back-end Rust
cd src-tauri && cargo test

# Vérification des types & Lint
npm run typecheck
cd src-tauri && cargo clippy -- -D warnings
npm run format         # Formatage dprint
npm run lint:eslint    # Vérification ESLint
npm run contracts      # Vérification des contrats API

# Vérification complète CI
npm run ci:check
```

### Scripts courants

| Commande                 | Utilisation                                     |
| ------------------------ | ----------------------------------------------- |
| `npm run bump`           | Mise à niveau du numéro de version (interactif) |
| `npm run docs`           | Génération de la documentation TypeDoc          |
| `npm run skill:create`   | Création d'un squelette de nouvelle compétence  |
| `npm run skill:validate` | Validation de la définition d'une compétence    |
| `npm run check:types`    | Vérification de la cohérence des types          |

---

## Plateformes prises en charge

| Plateforme | Architecture                          |
| ---------- | ------------------------------------- |
| Windows    | x86_64, ARM64                         |
| macOS      | Apple Silicon (arm64), Intel (x86_64) |
| Linux      | x86_64, ARM64                         |
| Android    | arm64-v8a, armeabi-v7a, x86_64        |
| iOS        | arm64                                 |

---

## Licence open source

Ce projet est publié sous licence [AGPL-3.0-only](LICENSE).

---

## Remerciements

AxAgent est construit sur de nombreux excellents projets open source :

- [Tauri](https://tauri.app/) — framework de bureau multiplateforme
- [React](https://react.dev/) + [Ant Design](https://ant.design/) — UI front-end
- [SeaORM](https://www.sea-ql.org/SeaORM/) — ORM Rust
- [sqlite-vec](https://github.com/asg017/sqlite-vec) — recherche vectorielle
- [candle](https://github.com/huggingface/candle) — inférence d'embeddings locaux
- [rmcp](https://github.com/nicholasxjy/rmcp) — SDK MCP Rust
- [ReactFlow](https://reactflow.dev/) — éditeur de workflows visuels
- [axum](https://github.com/tokio-rs/axum) — framework HTTP
- [Monaco Editor](https://microsoft.github.io/monaco-editor/) — éditeur de code
- [xterm.js](https://xtermjs.org/) — émulateur de terminal
- [Zustand](https://zustand.docs.pmnd.rs/) — gestion d'état
- [Framer Motion](https://www.framer.com/motion/) — bibliothèque d'animations
- [Recharts](https://recharts.org/) — bibliothèque de graphiques
